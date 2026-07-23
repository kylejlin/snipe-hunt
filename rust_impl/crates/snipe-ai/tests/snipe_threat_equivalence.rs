use snipe_ai::GamePosition;
use snipe_core::{AtomicMove, State};

const SEEDS: u64 = 64;
const MAX_PLIES: usize = 64;

#[test]
fn threat_gate_matches_exhaustive_complete_turn_search_on_reachable_states() {
    let mut checked = 0_u64;
    let mut threatened = 0_u64;
    let mut pending_checked = 0_u64;

    for seed in 0..SEEDS {
        let mut rng = SplitMix64::new(seed ^ 0xd6e8_feb8_6659_fd93);
        let mut state = State::initial(seed);
        for _ in 0..MAX_PLIES {
            if state.winner().is_some() {
                break;
            }

            check_position(state, &mut checked, &mut threatened, &mut pending_checked);
            // Reflection changes both player and board direction and catches
            // accidental Alpha/Beta asymmetry in the hypothetical side swap.
            check_position(
                state.reflected(),
                &mut checked,
                &mut threatened,
                &mut pending_checked,
            );

            let moves = state.legal_moves();
            let mv = moves[(rng.next() as usize) % moves.len()];
            state = state.apply_move(mv).unwrap();
        }
    }

    assert!(
        checked >= 2_000,
        "random games terminated too quickly: checked {checked}"
    );
    assert!(
        threatened > 0,
        "suite must exercise at least one actual one-turn snipe threat"
    );
    assert!(
        pending_checked >= 1_000,
        "suite must exercise pending-turn detection broadly: checked {pending_checked}"
    );
    eprintln!(
        "equivalence coverage: complete={checked}, pending={pending_checked}, \
         opponent_threats={threatened}"
    );
}

#[test]
fn complete_moves_never_leak_pending_state_into_search() {
    let mut observed_pending_atomic = false;
    for seed in 0..32 {
        let state = State::initial(seed);
        for mv in state.legal_moves() {
            let child = state.apply_move(mv).unwrap();
            assert!(child.pending_animal_step().is_none());
        }

        // The rules layer supports a pending first animal step for the UI, but
        // the AI's GamePosition contract deliberately consumes complete turns.
        if let Some(AtomicMove::Animal(step)) = state
            .legal_atomics()
            .into_iter()
            .find(|mv| matches!(mv, AtomicMove::Animal(_)))
        {
            let pending = state.apply_atomic(AtomicMove::Animal(step)).unwrap();
            assert!(pending.pending_animal_step().is_some());
            assert!(pending.legal_moves().is_empty());
            observed_pending_atomic = true;
        }
    }
    assert!(observed_pending_atomic);
}

fn check_position(
    state: State,
    checked: &mut u64,
    threatened: &mut u64,
    pending_checked: &mut u64,
) {
    assert!(state.pending_animal_step().is_none());

    let attacker = state.side_to_move();
    let expected_current = state.legal_moves().into_iter().any(|mv| {
        state
            .apply_move(mv)
            .is_ok_and(|child| child.captured_snipe_winner() == Some(attacker))
    });
    assert_eq!(
        state.has_winning_snipe_capture(),
        expected_current,
        "optimized current-player mismatch at hash {:016x}",
        state.position_hash()
    );

    let expected = exhaustive_opponent_snipe_capture(state);
    let actual = GamePosition::has_immediate_snipe_capture_threat(&state);
    assert_eq!(
        actual,
        expected,
        "threat mismatch at hash {:016x}: {:?}",
        state.position_hash(),
        state.to_data()
    );
    *checked += 1;
    *threatened += u64::from(expected);

    if let Some(pending) = state.legal_atomics().into_iter().find_map(|first| {
        let AtomicMove::Animal(_) = first else {
            return None;
        };
        let child = state.apply_atomic(first).ok()?;
        child.pending_animal_step().is_some().then_some(child)
    }) {
        let pending_attacker = pending.side_to_move();
        let expected_pending = pending.legal_atomics().into_iter().any(|second| {
            pending
                .apply_atomic(second)
                .is_ok_and(|child| child.captured_snipe_winner() == Some(pending_attacker))
        });
        assert_eq!(
            pending.has_winning_snipe_capture(),
            expected_pending,
            "optimized pending-turn mismatch at hash {:016x}: {:?}",
            pending.position_hash(),
            pending.to_data()
        );
        // The adapter's hypothetical-opponent query must also remain robust
        // when passed an editor/UI state with a pending animal step.
        let adapter_expected = exhaustive_opponent_snipe_capture(pending);
        assert_eq!(
            GamePosition::has_immediate_snipe_capture_threat(&pending),
            adapter_expected
        );
        *pending_checked += 1;
    }
}

fn exhaustive_opponent_snipe_capture(state: State) -> bool {
    let defender = state.side_to_move();
    let attacker = defender.opponent();
    let mut data = state.to_data();
    data.side_to_move = attacker as u8;
    data.pending_animal = u8::MAX;
    data.pending_destination = 0;
    let attacker_to_move = State::from_data(data).unwrap();
    attacker_to_move.legal_moves().into_iter().any(|mv| {
        attacker_to_move
            .apply_move(mv)
            .is_ok_and(|child| child.captured_snipe_winner() == Some(attacker))
    })
}

struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    }
}
