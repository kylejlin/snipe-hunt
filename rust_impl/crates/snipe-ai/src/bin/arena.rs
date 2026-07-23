use std::time::Duration;

use snipe_ai::{evaluate_state, tactical_move_score, SearchConfig, SearchEngine};
use snipe_core::{Move, Player, State};

fn main() {
    let games = argument(1, 10_u64);
    let milliseconds = argument(2, 50_u64);
    let max_turns = argument(3, 300_usize);
    let opponent = std::env::args()
        .nth(4)
        .unwrap_or_else(|| "greedy".to_owned());
    let max_depth = argument(5, 32_u8);
    let mut engine = SearchEngine::<State>::new(SearchConfig {
        time_limit: Duration::from_millis(milliseconds),
        max_depth,
        transposition_table_mb: 64,
        ..SearchConfig::default()
    });

    let mut search_wins = 0;
    let mut opponent_wins = 0;
    let mut draws = 0;
    let mut total_depth = 0_u64;
    let mut searches = 0_u64;

    for game in 0..games {
        // Play each deal twice with the engines swapping sides. Pairing the
        // same deal prevents a favorable random allocation from being
        // mistaken for engine strength.
        let seed = game / 2;
        let search_moves_first = game & 1 == 0;
        let mut state = State::initial(seed);
        let mut turns = 0;
        while state.winner().is_none() && turns < max_turns {
            let search_to_move = (turns & 1 == 0) == search_moves_first;
            let mv = if search_to_move {
                let result = engine.search(&state);
                total_depth += result.depth as u64;
                searches += 1;
                result
                    .best_move
                    .expect("nonterminal state has a legal move")
            } else {
                match opponent.as_str() {
                    "random" => random_move(state, seed ^ turns as u64),
                    "greedy" => greedy_move(state),
                    other => panic!("unknown opponent {other:?}; use greedy or random"),
                }
            };
            state = state.apply_move(mv).expect("selected move must be legal");
            turns += 1;
        }

        match state.winner() {
            Some(winner) => {
                // Initial side-to-move is Beta, so Beta moved first.
                let winner_moved_first = winner == Player::Beta;
                if winner_moved_first == search_moves_first {
                    search_wins += 1;
                } else {
                    opponent_wins += 1;
                }
            }
            None => draws += 1,
        }
        println!(
            "game {game:>3} seed {seed:>3}: turns={turns:>3}, running search/opponent/draw={search_wins}/{opponent_wins}/{draws}"
        );
    }

    let average_depth = if searches == 0 {
        0.0
    } else {
        total_depth as f64 / searches as f64
    };
    println!(
        "RESULT games={games} time_ms={milliseconds} opponent={opponent} search_wins={search_wins} opponent_wins={opponent_wins} draws={draws} avg_depth={average_depth:.2}"
    );
}

fn random_move(state: State, mut value: u64) -> Move {
    let moves = state.legal_moves();
    // One SplitMix64 round gives reproducible, well-distributed indexing.
    value = value.wrapping_add(0x9e3779b97f4a7c15);
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^= value >> 31;
    moves[value as usize % moves.len()]
}

fn greedy_move(state: State) -> Move {
    state
        .legal_moves()
        .into_iter()
        .max_by_key(|&mv| {
            let child = state.apply_move(mv).expect("generated move must apply");
            (
                tactical_move_score(state, mv, child),
                -evaluate_state(child),
                std::cmp::Reverse(mv),
            )
        })
        .expect("nonterminal state has a legal move")
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
