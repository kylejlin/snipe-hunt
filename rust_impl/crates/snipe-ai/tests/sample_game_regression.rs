use std::time::Duration;

use snipe_ai::{extract_features, SearchConfig, SearchEngine, SearchPolicy};
use snipe_core::{Animal, AnimalStep, Location, Move, Player, Row, State, StateData};

const GAME: &str = include_str!("../../../../game3.shgh");
const ANIMAL_NAMES: [&str; 16] = [
    "Rat", "Ox", "Tiger", "Rabbit", "Dragon", "Snake", "Horse", "Ram", "Monkey", "Rooster", "Dog",
    "Boar", "Fish", "Elephant", "Squid", "Frog",
];

fn sample_timeline() -> Vec<State> {
    let lines = GAME
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with("//"))
        .collect::<Vec<_>>();
    let mut position = initial_state(lines[0], lines[1]);
    let mut timeline = vec![position];

    for line in &lines[2..] {
        let (_, body) = line.split_once(' ').expect("move line has a ply prefix");
        let notation = body
            .strip_suffix(" +#0")
            .or_else(|| body.strip_suffix(" -#0"))
            .unwrap_or(body);
        let mv = position
            .legal_moves()
            .into_iter()
            .filter(|mv| format_move(position, *mv) == notation)
            .min()
            .unwrap_or_else(|| panic!("recorded move is legal: {line}"));
        position = position.apply_move(mv).expect("recorded move applies");
        timeline.push(position);
    }
    timeline
}

fn initial_state(beta_line: &str, alpha_line: &str) -> State {
    let mut data = StateData {
        alpha_animals: [0; 8],
        beta_animals: [0; 8],
        snipes: [0; 8],
        side_to_move: Player::Beta as u8,
        pending_animal: u8::MAX,
        pending_destination: 0,
    };
    let mut occurrences = [0_u8; 16];
    add_layout(
        &mut data,
        beta_line,
        Player::Beta,
        [
            Location::BetaReserve,
            Location::Row6,
            Location::Row5,
            Location::Row4,
        ],
        &mut occurrences,
    );
    add_layout(
        &mut data,
        alpha_line,
        Player::Alpha,
        [
            Location::AlphaReserve,
            Location::Row1,
            Location::Row2,
            Location::Row3,
        ],
        &mut occurrences,
    );
    assert!(occurrences.into_iter().all(|count| count == 2));
    State::from_data(data).expect("game3 initial position is valid")
}

fn add_layout(
    data: &mut StateData,
    line: &str,
    owner: Player,
    locations: [Location; 4],
    occurrences: &mut [u8; 16],
) {
    let (_, layout) = line
        .split_once('=')
        .expect("layout line has an equals sign");
    let groups = layout.split(';').map(str::trim).collect::<Vec<_>>();
    assert_eq!(groups.len(), locations.len());
    for (group, location) in groups.into_iter().zip(locations) {
        for name in group.split_whitespace() {
            if name == player_name(owner) {
                data.snipes[location.index()] |= 1 << owner as u8;
                continue;
            }
            let base = ANIMAL_NAMES
                .iter()
                .position(|candidate| *candidate == name)
                .unwrap_or_else(|| panic!("known animal name: {name}"));
            let animal = Animal::from_index(base as u8 + occurrences[base] * 16)
                .expect("game contains at most two copies of each animal");
            occurrences[base] += 1;
            match owner {
                Player::Alpha => data.alpha_animals[location.index()] |= animal.bit(),
                Player::Beta => data.beta_animals[location.index()] |= animal.bit(),
            }
        }
    }
}

fn player_name(player: Player) -> &'static str {
    match player {
        Player::Alpha => "Alpha",
        Player::Beta => "Beta",
    }
}

fn animal_name(animal: Animal) -> &'static str {
    ANIMAL_NAMES[animal.index() % 16]
}

fn step_notation(name: &str, source: Location, destination: Row, player: Player) -> String {
    if source == Location::reserve_of(player) {
        return format!("{name} {}!", destination.number());
    }
    let source_rank = source.row().expect("played card starts on a row").number();
    let destination_rank = destination.number();
    let advances = match player {
        Player::Alpha => destination_rank > source_rank,
        Player::Beta => destination_rank < source_rank,
    };
    format!(
        "{name} {destination_rank}{}",
        if advances { "" } else { "*" }
    )
}

fn animal_step_notation(position: State, step: AnimalStep, player: Player) -> String {
    step_notation(
        animal_name(step.moved),
        position
            .location_of_animal(step.moved)
            .expect("played animal is on the board"),
        step.destination,
        player,
    )
}

fn format_move(position: State, mv: Move) -> String {
    let player = position.side_to_move();
    match mv {
        Move::Snipe { destination } => step_notation(
            player_name(player),
            position
                .snipe_location(player)
                .expect("current player's snipe is on the board"),
            destination,
            player,
        ),
        Move::Drop {
            animal,
            destination,
        } => step_notation(
            animal_name(animal),
            Location::reserve_of(player),
            destination,
            player,
        ),
        Move::Animals { first, second } => {
            let first = animal_step_notation(position, first, player);
            match second {
                Some(second) => {
                    format!(
                        "{first}, {}",
                        animal_step_notation(position, second, player)
                    )
                }
                None => first,
            }
        }
    }
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
