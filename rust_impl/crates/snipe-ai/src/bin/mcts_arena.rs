//! Paired-match evidence for the independent MCTS challenger.

use std::time::Duration;

use snipe_ai::{
    evaluate_state, tactical_move_score, MctsConfig, MctsEngine, SearchConfig, SearchEngine,
};
use snipe_core::{Move, Player, State};

#[derive(Clone, Copy)]
enum Opponent {
    AlphaBeta,
    Greedy,
}

#[derive(Default)]
struct Score {
    mcts_wins: u32,
    opponent_wins: u32,
    draws: u32,
    turns: u64,
    iterations: u64,
    searches: u64,
}

fn main() {
    let pairs = argument(1, 6_u64);
    let milliseconds = argument(2, 25_u64);
    let max_turns = argument(3, 220_usize);

    println!("MCTS challenger: pairs={pairs}, time={milliseconds}ms/move, max_turns={max_turns}");
    let versus_ab = run_suite(pairs, milliseconds, max_turns, Opponent::AlphaBeta);
    print_score("alpha-beta depth 3", versus_ab, pairs * 2);
    let versus_greedy = run_suite(pairs, milliseconds, max_turns, Opponent::Greedy);
    print_score("greedy", versus_greedy, pairs * 2);
}

fn run_suite(pairs: u64, milliseconds: u64, max_turns: usize, opponent: Opponent) -> Score {
    let mut score = Score::default();
    let mut mcts = MctsEngine::new(MctsConfig {
        time_limit: Duration::from_millis(milliseconds),
        max_iterations: 50_000,
        rollout_depth: 5,
        max_moves_per_node: 72,
        seed: 0x4d43_5453_5f41_5245,
        ..MctsConfig::default()
    });
    let mut alpha_beta = SearchEngine::<State>::new(SearchConfig {
        time_limit: Duration::from_millis(milliseconds),
        max_depth: 3,
        transposition_table_mb: 32,
        ..SearchConfig::default()
    });

    for seed in 0..pairs {
        for seat in 0..2 {
            let mcts_moves_first = seat == 0;
            let (winner, turns) = play_one(
                State::initial(seed),
                mcts_moves_first,
                &mut mcts,
                &mut alpha_beta,
                opponent,
                max_turns,
                &mut score,
            );
            score.turns += turns as u64;
            match winner {
                Some(player) => {
                    let winner_moved_first = player == Player::Beta;
                    if winner_moved_first == mcts_moves_first {
                        score.mcts_wins += 1;
                    } else {
                        score.opponent_wins += 1;
                    }
                }
                None => score.draws += 1,
            }
            println!(
                "  seed={seed:>2} seat={} turns={turns:>3} running={}/{}/{}",
                if mcts_moves_first { "first " } else { "second" },
                score.mcts_wins,
                score.opponent_wins,
                score.draws
            );
        }
    }
    score
}

fn play_one(
    mut state: State,
    mcts_moves_first: bool,
    mcts: &mut MctsEngine,
    alpha_beta: &mut SearchEngine<State>,
    opponent: Opponent,
    max_turns: usize,
    score: &mut Score,
) -> (Option<Player>, usize) {
    for turn in 0..max_turns {
        if let Some(winner) = state.winner() {
            return (Some(winner), turn);
        }
        let mcts_to_move = (turn & 1 == 0) == mcts_moves_first;
        let mv = if mcts_to_move {
            let result = mcts.search(state);
            score.iterations += result.stats.iterations;
            score.searches += 1;
            result
                .best_move
                .expect("nonterminal position has a legal move")
        } else {
            match opponent {
                Opponent::AlphaBeta => alpha_beta
                    .search(&state)
                    .best_move
                    .expect("nonterminal position has a legal move"),
                Opponent::Greedy => greedy_move(state),
            }
        };
        state = state.apply_move(mv).expect("selected move must apply");
    }
    (None, max_turns)
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
        .expect("nonterminal position has a legal move")
}

fn print_score(opponent: &str, score: Score, games: u64) {
    let pct = if games == 0 {
        0.0
    } else {
        100.0 * (score.mcts_wins as f64 + score.draws as f64 * 0.5) / games as f64
    };
    let avg_turns = score.turns as f64 / games.max(1) as f64;
    let avg_iterations = score.iterations as f64 / score.searches.max(1) as f64;
    println!(
        "RESULT vs={opponent:?} games={games} mcts_wins={} opponent_wins={} draws={} score={pct:.1}% avg_turns={avg_turns:.1} avg_iterations={avg_iterations:.0}",
        score.mcts_wins, score.opponent_wins, score.draws
    );
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
