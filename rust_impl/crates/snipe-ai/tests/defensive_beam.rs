use std::time::Duration;

use snipe_ai::{SearchConfig, SearchEngine};
use snipe_core::{State, StateData};

/// This reachable position came from seed 8 after 41 deliberately varied
/// legal plies. Alpha has 129 legal turns. Under the production ordering as of
/// d45ec10, all first 48 turns leave Alpha's snipe capturable immediately; the
/// first safe turn ranked 89th and was therefore invisible to search.
fn beam_trap() -> State {
    State::from_data(StateData {
        alpha_animals: [
            540_631_750,
            0,
            32_768,
            8_388_865,
            268_439_552,
            33_554_432,
            2_214_593_552,
            0,
        ],
        beta_animals: [0, 393_216, 1_077_936_128, 0, 0, 2_080, 0, 150_994_952],
        snipes: [0, 1, 0, 0, 0, 2, 0, 0],
        side_to_move: 0,
        pending_animal: u8::MAX,
        pending_destination: 0,
    })
    .unwrap()
}

#[test]
fn selective_search_keeps_a_move_that_avoids_mate_in_one() {
    let state = beam_trap();
    assert_eq!(state.legal_moves().len(), 129);
    assert!(state
        .legal_moves()
        .into_iter()
        .map(|mv| state.apply_move(mv).unwrap())
        .any(|child| !has_immediate_winning_reply(child)));

    let mut engine = SearchEngine::new(SearchConfig {
        time_limit: Duration::from_secs(20),
        max_depth: 1,
        quiescence_depth: 1,
        selective_move_limit: 48,
        deadline_check_interval: 1,
        ..SearchConfig::default()
    });
    let result = engine.search(&state);
    let chosen = state
        .apply_move(result.best_move.expect("position is nonterminal"))
        .unwrap();

    assert!(
        !has_immediate_winning_reply(chosen),
        "beam chose a move allowing immediate loss: {result:?}; winning reply: {:?}",
        immediate_winning_reply(chosen)
    );
}

fn has_immediate_winning_reply(state: State) -> bool {
    immediate_winning_reply(state).is_some()
}

fn immediate_winning_reply(state: State) -> Option<snipe_core::Move> {
    let attacker = state.side_to_move();
    state.legal_moves().into_iter().find(|&reply| {
        state
            .apply_move(reply)
            .is_ok_and(|child| child.winner() == Some(attacker))
    })
}
