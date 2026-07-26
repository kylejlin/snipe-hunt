use std::time::Duration;

use snipe_ai::{evaluate_state, SearchConfig, SearchEngine};
use snipe_banana::{evaluate, BananaConfig, BananaEngine};
use snipe_core::{Player, State};

#[derive(Default)]
struct Summary {
    banana_wins: u64,
    almond_wins: u64,
    draws: u64,
    turns: u64,
    banana_moves: u64,
    almond_moves: u64,
    banana_nodes: u64,
    almond_nodes: u64,
    banana_depth: u64,
    almond_depth: u64,
}

fn main() {
    let pairs = argument(1, 10_u64);
    let milliseconds = argument(2, 250_u64);
    let max_turns = argument(3, 180_usize);
    let first_seed = argument(4, 0_u64);
    let beam_width = argument(5, 18_usize);
    let almond_milliseconds = argument(6, milliseconds);
    let banana_max_depth = argument(7, 64_u8);

    let mut banana = BananaEngine::new(BananaConfig {
        time_limit: Duration::from_millis(milliseconds),
        beam_width,
        transposition_table_mb: 64,
        max_depth: banana_max_depth,
        ..BananaConfig::default()
    });
    let mut almond = SearchEngine::<State>::new(SearchConfig {
        time_limit: Duration::from_millis(almond_milliseconds),
        transposition_table_mb: 64,
        ..SearchConfig::default()
    });
    let mut summary = Summary::default();

    for seed in first_seed..first_seed + pairs {
        play(
            State::initial(seed),
            true,
            &mut banana,
            &mut almond,
            max_turns,
            milliseconds >= 1_000 || almond_milliseconds >= 1_000,
            &mut summary,
        );
        play(
            State::initial(seed),
            false,
            &mut banana,
            &mut almond,
            max_turns,
            milliseconds >= 1_000 || almond_milliseconds >= 1_000,
            &mut summary,
        );
        eprintln!(
            "seed={seed} banana/almond/draw={}/{}/{}",
            summary.banana_wins, summary.almond_wins, summary.draws
        );
    }

    println!(
        "RESULT pairs={pairs} banana_ms={milliseconds} almond_ms={almond_milliseconds} beam={beam_width} banana_max_depth={banana_max_depth} \
banana_wins={} almond_wins={} draws={} avg_turns={:.1} \
banana_depth={:.2} almond_depth={:.2} banana_nodes={:.0} almond_nodes={:.0}",
        summary.banana_wins,
        summary.almond_wins,
        summary.draws,
        ratio(summary.turns, pairs * 2),
        ratio(summary.banana_depth, summary.banana_moves),
        ratio(summary.almond_depth, summary.almond_moves),
        ratio(summary.banana_nodes, summary.banana_moves),
        ratio(summary.almond_nodes, summary.almond_moves),
    );
}

fn play(
    mut state: State,
    banana_first: bool,
    banana: &mut BananaEngine,
    almond: &mut SearchEngine<State>,
    max_turns: usize,
    verbose: bool,
    summary: &mut Summary,
) {
    let mut turns = 0;
    let mut repetition_history = Vec::new();
    let mut convergence_history = Vec::new();
    while state.winner().is_none() && turns < max_turns {
        let banana_to_move = (turns % 2 == 0) == banana_first;
        let mv = if banana_to_move {
            let result = banana.search_with_history(&state, &repetition_history);
            summary.banana_moves += 1;
            summary.banana_nodes += result.stats.nodes;
            summary.banana_depth += u64::from(result.depth);
            if verbose {
                eprintln!(
                    "  turn={turns:>3} player=banana depth={} nodes={} elapsed_ms={}",
                    result.depth,
                    result.stats.nodes,
                    result.stats.elapsed.as_millis()
                );
            }
            result.best_move.expect("nonterminal state has a move")
        } else {
            let result =
                almond.search_with_context(&state, &repetition_history, &convergence_history);
            summary.almond_moves += 1;
            summary.almond_nodes += result.stats.nodes + result.stats.qnodes;
            summary.almond_depth += u64::from(result.depth);
            if verbose {
                eprintln!(
                    "  turn={turns:>3} player=almond depth={} nodes={} elapsed_ms={}",
                    result.depth,
                    result.stats.nodes + result.stats.qnodes,
                    result.stats.elapsed.as_millis()
                );
            }
            result.best_move.expect("nonterminal state has a move")
        };
        repetition_history.push(state.repetition_hash());
        convergence_history.push(state.convergence_hash());
        state = state.apply_move(mv).expect("engine move must be legal");
        turns += 1;
    }
    summary.turns += turns as u64;

    let winner = state.winner().or_else(|| {
        let banana_to_move = (turns % 2 == 0) == banana_first;
        let score = if banana_to_move {
            evaluate(state)
        } else {
            evaluate_state(state)
        };
        (score != 0).then(|| {
            if score > 0 {
                state.side_to_move()
            } else {
                state.side_to_move().opponent()
            }
        })
    });
    let Some(winner) = winner else {
        summary.draws += 1;
        return;
    };
    let winner_moved_first = winner == Player::Beta;
    if winner_moved_first == banana_first {
        summary.banana_wins += 1;
    } else {
        summary.almond_wins += 1;
    }
}

fn ratio(total: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
