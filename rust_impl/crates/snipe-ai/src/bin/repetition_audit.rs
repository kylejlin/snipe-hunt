//! Search production-policy self-play for real repeated positions.
//!
//! The game rules do not declare repetition a draw. A repeat here is therefore
//! evidence of a practical engine-cycle risk, while the absence of repeats is
//! only a bounded audit result.

use std::collections::HashMap;
use std::time::Duration;

use snipe_ai::{SearchConfig, SearchEngine, SearchPolicy};
use snipe_core::{Move, State};

fn main() {
    let seeds = argument(1, 32_u64);
    let milliseconds = argument(2, 100_u64);
    let max_turns = argument(3, 200_usize);
    let first_seed = argument(4, 0_u64);
    let history_aware = argument(5, 0_u8) != 0;
    let convergence_penalty = argument(6, 300_i32);
    let mut repeats = 0_u64;
    let mut terminals = 0_u64;
    let mut capped = 0_u64;
    let mut total_turns = 0_u64;

    for seed in first_seed..first_seed.saturating_add(seeds) {
        let mut state = State::initial(seed);
        let mut seen = HashMap::new();
        let mut moves_played = Vec::<Move>::new();
        let mut prior_hashes = Vec::<u64>::new();
        let mut prior_convergence = Vec::<u64>::new();
        seen.insert(state.position_hash(), 0_usize);

        for turn in 0..max_turns {
            if state.winner().is_some() {
                terminals += 1;
                total_turns += turn as u64;
                break;
            }
            let mut engine = SearchEngine::new_with_policy(
                SearchConfig {
                    time_limit: Duration::from_millis(milliseconds),
                    max_depth: 3,
                    ..SearchConfig::default()
                },
                SearchPolicy {
                    convergence_history_penalty: convergence_penalty,
                    ..SearchPolicy::production()
                },
            );
            let result = if history_aware {
                engine.search_with_context(&state, &prior_hashes, &prior_convergence)
            } else {
                engine.search(&state)
            };
            let Some(mv) = result.best_move else {
                terminals += 1;
                total_turns += turn as u64;
                break;
            };
            moves_played.push(mv);
            prior_hashes.push(if history_aware {
                state.repetition_hash()
            } else {
                state.position_hash()
            });
            if history_aware {
                prior_convergence.push(state.convergence_hash());
            }
            state = state.apply_move(mv).unwrap();

            if let Some(&first_turn) = seen.get(&state.position_hash()) {
                repeats += 1;
                total_turns += (turn + 1) as u64;
                println!(
                    "REPEAT seed={seed} first_turn={first_turn} repeated_turn={} \
cycle_len={} hash={:016x} state={:?} cycle={:?}",
                    turn + 1,
                    turn + 1 - first_turn,
                    state.position_hash(),
                    state.to_data(),
                    &moves_played[first_turn..]
                );
                break;
            }
            seen.insert(state.position_hash(), turn + 1);

            if turn + 1 == max_turns {
                capped += 1;
                total_turns += max_turns as u64;
                println!("CAPPED seed={seed} turns={max_turns}");
            }
        }
    }

    println!(
        "RESULT seeds={seeds} first_seed={first_seed} time_ms={milliseconds} max_turns={max_turns} \
history_aware={history_aware} \
convergence_penalty={convergence_penalty} \
terminals={terminals} repeats={repeats} capped={capped} avg_turns={:.1}",
        total_turns as f64 / seeds.max(1) as f64
    );
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
