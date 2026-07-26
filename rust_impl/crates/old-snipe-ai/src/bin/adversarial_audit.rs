//! Find reachable positions where the ordinary 48-move ordering would discard
//! every move that prevents an immediate loss.
//!
//! The production defense policy is intentionally not modeled here: this tool
//! finds regression fixtures that justify and exercise that policy.

use std::cmp::Reverse;

use snipe_ai::tactical_move_score;
use snipe_core::{Location, Move, State};

fn main() {
    let seeds = argument(1, 2_000_u64);
    let plies = argument(2, 100_usize);
    let mut examined = 0_u64;
    let mut threatened = 0_u64;
    let mut beam_traps = 0_u64;

    for seed in 0..seeds {
        let mut rng = SplitMix64::new(seed ^ 0xa076_1d64_78bd_642f);
        let mut state = State::initial(seed);
        for ply in 0..plies {
            if state.winner().is_some() {
                break;
            }
            audit(
                seed,
                ply,
                state,
                &mut examined,
                &mut threatened,
                &mut beam_traps,
            );
            let moves = state.legal_moves();
            if moves.is_empty() {
                break;
            }
            // Mostly-random play reaches scrappy positions while preferring a
            // capture often enough to resemble plausible games.
            let mut ranked = moves
                .iter()
                .copied()
                .map(|mv| {
                    let child = state.apply_move(mv).unwrap();
                    (ordering_key(state, mv, child), mv)
                })
                .collect::<Vec<_>>();
            ranked.sort_unstable_by_key(|&(key, mv)| Reverse((key, Reverse(mv))));
            let choice = if rng.next() & 3 == 0 {
                (rng.next() as usize) % ranked.len().min(12)
            } else {
                (rng.next() as usize) % ranked.len()
            };
            state = state.apply_move(ranked[choice].1).unwrap();
        }
    }

    println!(
        "RESULT seeds={seeds} examined={examined} threatened={threatened} \
beam_traps={beam_traps}"
    );
}

fn audit(
    seed: u64,
    ply: usize,
    state: State,
    examined: &mut u64,
    threatened: &mut u64,
    beam_traps: &mut u64,
) {
    *examined += 1;
    let moves = state.legal_moves();
    if moves.len() <= 48 {
        return;
    }

    let mut ranked = moves
        .iter()
        .copied()
        .map(|mv| {
            let child = state.apply_move(mv).unwrap();
            (ordering_key(state, mv, child), mv, immediately_loses(child))
        })
        .collect::<Vec<_>>();
    let any_unsafe = ranked.iter().any(|entry| entry.2);
    let any_safe = ranked.iter().any(|entry| !entry.2);
    if !any_unsafe || !any_safe {
        return;
    }
    *threatened += 1;

    ranked.sort_unstable_by_key(|&(key, mv, _)| Reverse((key, Reverse(mv))));
    if ranked[..48].iter().all(|entry| entry.2) {
        *beam_traps += 1;
        let safe_rank = ranked.iter().position(|entry| !entry.2).unwrap() + 1;
        println!(
            "BEAM_TRAP seed={seed} ply={ply} legal={} first_safe_rank={safe_rank} \
first_safe={:?} hash={:016x} state={:?}",
            ranked.len(),
            ranked[safe_rank - 1].1,
            state.position_hash(),
            state.to_data()
        );
    }
}

fn immediately_loses(child: State) -> bool {
    let attacker = child.side_to_move();
    child.legal_moves().into_iter().any(|reply| {
        child
            .apply_move(reply)
            .is_ok_and(|grandchild| grandchild.winner() == Some(attacker))
    })
}

fn ordering_key(parent: State, mv: Move, child: State) -> i32 {
    let player = parent.side_to_move();
    let reserve = Location::reserve_of(player);
    let tactical = child.captured_snipe_winner().is_some()
        || child.animal_bits(reserve, player) & !parent.animal_bits(reserve, player) != 0;
    i32::from(tactical) * 1_000_000 + tactical_move_score(parent, mv, child)
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
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
