use std::time::Duration;

use serde_json::Value;
use snipe_ai::{extract_features, SearchConfig, SearchEngine, SearchPolicy};
use snipe_core::{Move, Player, State, StateData};

const SAMPLE: &str = include_str!("../../../../sample1.json");
const STORAGE_KEY: &str = "snipe-hunt.mission-7.game";
const LOCATIONS: [&str; 8] = [
    "alpha-reserve",
    "row-1",
    "row-2",
    "row-3",
    "row-4",
    "row-5",
    "row-6",
    "beta-reserve",
];

fn sample_timeline() -> Vec<State> {
    let dump: Value = serde_json::from_str(SAMPLE).expect("sample localStorage dump is JSON");
    let saved: Value = serde_json::from_str(
        dump[STORAGE_KEY]
            .as_str()
            .expect("saved game is a JSON string"),
    )
    .expect("saved game string is JSON");
    saved["timeline"]
        .as_array()
        .expect("saved game has a timeline")
        .iter()
        .map(|entry| state_from_json(&entry["position"]))
        .collect()
}

fn state_from_json(position: &Value) -> State {
    let mut data = StateData {
        alpha_animals: [0; 8],
        beta_animals: [0; 8],
        snipes: [0; 8],
        side_to_move: match position["turn"].as_str() {
            Some("Alpha") => Player::Alpha as u8,
            Some("Beta") => Player::Beta as u8,
            turn => panic!("invalid side to move: {turn:?}"),
        },
        pending_animal: u8::MAX,
        pending_destination: 0,
    };

    for (location_index, location) in LOCATIONS.into_iter().enumerate() {
        for card in position["locations"][location]
            .as_array()
            .expect("location contains cards")
        {
            let owner = match card["owner"].as_str() {
                Some("Alpha") => Player::Alpha,
                Some("Beta") => Player::Beta,
                owner => panic!("invalid owner: {owner:?}"),
            };
            if card["isSnipe"].as_bool() == Some(true) {
                data.snipes[location_index] |= 1 << owner as u8;
            } else {
                let index = card["id"]
                    .as_str()
                    .and_then(|id| id.strip_prefix("animal-"))
                    .and_then(|index| index.parse::<u8>().ok())
                    .expect("animal card has a numeric id");
                let animals = match owner {
                    Player::Alpha => &mut data.alpha_animals,
                    Player::Beta => &mut data.beta_animals,
                };
                animals[location_index] |= 1_u32 << index;
            }
        }
    }
    State::from_data(data).expect("sample position is a valid core state")
}

fn played_move(before: State, after: State) -> Move {
    before
        .legal_moves()
        .into_iter()
        .find(|&mv| before.apply_move(mv) == Ok(after))
        .expect("recorded sample transition is legal")
}

#[test]
fn saved_game_replays_through_the_authoritative_rules() {
    let timeline = sample_timeline();
    assert_eq!(timeline.len(), 103);
    for pair in timeline.windows(2) {
        let _ = played_move(pair[0], pair[1]);
    }
    assert_eq!(
        timeline.last().and_then(|state| state.winner()),
        Some(Player::Alpha)
    );
}

#[test]
#[ignore = "diagnostic search over the human-vs-engine sample"]
fn diagnose_beta_material_losses() {
    let timeline = sample_timeline();
    let config = SearchConfig {
        time_limit: Duration::ZERO,
        max_depth: 3,
        quiescence_depth: 5,
        transposition_table_mb: 32,
        aspiration_window: 80,
        deadline_check_interval: 1_024,
        lmr_after_move: 5,
        selective_move_limit: 48,
    };
    let mut engine = SearchEngine::new_with_policy(config, SearchPolicy::production());
    let critical_plies = [21_usize, 43, 59, 63, 65, 67, 69, 73, 75, 77, 85, 93, 101];

    for ply in critical_plies {
        let root = timeline[ply - 1];
        let actual = played_move(root, timeline[ply]);
        let history = timeline[..ply - 1]
            .iter()
            .map(|state| state.repetition_hash())
            .collect::<Vec<_>>();
        let convergence = timeline[..ply - 1]
            .iter()
            .map(|state| state.convergence_hash())
            .collect::<Vec<_>>();
        let result = engine.search_to_depth_with_context_and_progress(
            &root,
            &history,
            &convergence,
            None,
            3,
            |_| {},
        );
        let best = result.best_move.expect("sample root is nonterminal");
        println!(
            "ply={ply} actual={actual:?} best={best:?} same={} score={} nodes={} \
             before={:?} actual_child={:?} best_child={:?}",
            actual == best,
            result.score,
            result.stats.nodes + result.stats.qnodes,
            extract_features(root),
            extract_features(root.apply_move(actual).unwrap()),
            extract_features(root.apply_move(best).unwrap()),
        );
    }
}

#[test]
#[ignore = "five-second native reproduction of the sample's major losses"]
fn diagnose_timed_major_losses() {
    let timeline = sample_timeline();
    for ply in [21, 63, 75, 85] {
        let root = timeline[ply - 1];
        let actual = played_move(root, timeline[ply]);
        let history = timeline[..ply - 1]
            .iter()
            .map(|state| state.repetition_hash())
            .collect::<Vec<_>>();
        let convergence = timeline[..ply - 1]
            .iter()
            .map(|state| state.convergence_hash())
            .collect::<Vec<_>>();
        let mut engine = SearchEngine::new(SearchConfig {
            time_limit: Duration::from_secs(5),
            ..SearchConfig::default()
        });
        let result = engine.search_with_context(&root, &history, &convergence);
        println!(
            "ply={ply} actual={actual:?} best={:?} same={} score={} depth={} nodes={} qnodes={} \
             elapsed={:?} pv={:?}",
            result.best_move,
            result.best_move == Some(actual),
            result.score,
            result.depth,
            result.stats.nodes,
            result.stats.qnodes,
            result.stats.elapsed,
            result.principal_variation,
        );
    }
}
