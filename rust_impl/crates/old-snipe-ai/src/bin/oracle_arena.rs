//! Deterministic asymmetric matches between production search budgets.
//!
//! Each seed is played twice with the engines swapping players. Both engines
//! receive the same complete canonical-repetition and convergence histories.
//! A distant wall-clock deadline is only a safety valve: `node_limit` is the
//! intended deterministic stopping condition.

use std::time::Duration;

use snipe_ai::{GamePosition, SearchConfig, SearchEngine, SearchPolicy};
use snipe_core::{Player, State};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Engine {
    Fast,
    Oracle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Finish {
    Terminal,
    Adjudicated,
    Draw,
}

#[derive(Default)]
struct EngineStats {
    moves: u64,
    nodes: u64,
    depth: u64,
    elapsed_micros: u128,
    incomplete_iterations: u64,
    wins: u64,
    terminal_wins: u64,
    adjudicated_wins: u64,
    wins_as_first: u64,
}

#[derive(Default)]
struct Summary {
    fast: EngineStats,
    oracle: EngineStats,
    draws: u64,
    capped_games: u64,
    turns: u64,
}

fn main() {
    let pairs = argument(1, 2_u64);
    let fast_nodes = argument(2, 20_000_u64);
    let oracle_nodes = argument(3, 200_000_u64);
    let max_turns = argument(4, 200_usize);
    let first_seed = argument(5, 0_u64);
    let fast_depth = argument(6, 64_u8);
    let oracle_depth = argument(7, 64_u8);

    assert!(pairs > 0, "pairs must be positive");
    assert!(fast_nodes > 0, "fast node budget must be positive");
    assert!(
        oracle_nodes > fast_nodes,
        "oracle budget must exceed fast budget"
    );
    assert!(max_turns > 0, "max turns must be positive");
    assert!(
        fast_depth > 0 && oracle_depth > 0,
        "depths must be positive"
    );

    let mut fast = engine(fast_nodes, fast_depth);
    let mut oracle = engine(oracle_nodes, oracle_depth);
    let mut summary = Summary::default();

    for seed in first_seed..first_seed.saturating_add(pairs) {
        // Beta is the first player in State::initial.
        play_one(
            seed,
            Player::Beta,
            &mut fast,
            &mut oracle,
            max_turns,
            &mut summary,
        );
        play_one(
            seed,
            Player::Alpha,
            &mut fast,
            &mut oracle,
            max_turns,
            &mut summary,
        );
    }

    let games = pairs.saturating_mul(2);
    println!(
        "RESULT pairs={pairs} games={games} first_seed={first_seed} \
fast_nodes={fast_nodes} oracle_nodes={oracle_nodes} \
fast_depth_limit={fast_depth} oracle_depth_limit={oracle_depth} \
fast_wins={} oracle_wins={} draws={} capped_games={} avg_turns={:.1} \
fast_terminal_wins={} oracle_terminal_wins={} \
fast_adjudicated_wins={} oracle_adjudicated_wins={} \
fast_wins_as_first={} oracle_wins_as_first={} \
fast_nodes_per_move={:.1} oracle_nodes_per_move={:.1} \
fast_depth={:.2} oracle_depth={:.2} \
fast_ms_per_move={:.2} oracle_ms_per_move={:.2} \
fast_incomplete={} oracle_incomplete={}",
        summary.fast.wins,
        summary.oracle.wins,
        summary.draws,
        summary.capped_games,
        ratio(summary.turns, games),
        summary.fast.terminal_wins,
        summary.oracle.terminal_wins,
        summary.fast.adjudicated_wins,
        summary.oracle.adjudicated_wins,
        summary.fast.wins_as_first,
        summary.oracle.wins_as_first,
        ratio(summary.fast.nodes, summary.fast.moves),
        ratio(summary.oracle.nodes, summary.oracle.moves),
        ratio(summary.fast.depth, summary.fast.moves),
        ratio(summary.oracle.depth, summary.oracle.moves),
        micros_per_move(&summary.fast),
        micros_per_move(&summary.oracle),
        summary.fast.incomplete_iterations,
        summary.oracle.incomplete_iterations,
    );
}

fn engine(node_limit: u64, max_depth: u8) -> SearchEngine<State> {
    SearchEngine::new_with_policy(
        SearchConfig {
            // Node limits make the comparison reproducible. This only guards
            // unexpectedly expensive work outside the recursive node counter.
            time_limit: Duration::from_secs(3_600),
            max_depth,
            transposition_table_mb: 64,
            deadline_check_interval: 1,
            ..SearchConfig::default()
        },
        SearchPolicy {
            node_limit: Some(node_limit),
            ..SearchPolicy::production()
        },
    )
}

fn play_one(
    seed: u64,
    fast_player: Player,
    fast: &mut SearchEngine<State>,
    oracle: &mut SearchEngine<State>,
    max_turns: usize,
    summary: &mut Summary,
) {
    let mut state = State::initial(seed);
    let mut repetition_history = Vec::with_capacity(max_turns);
    let mut convergence_history = Vec::with_capacity(max_turns);
    let mut turns = 0_usize;

    while state.winner().is_none() && turns < max_turns {
        let moving_engine = if state.side_to_move() == fast_player {
            Engine::Fast
        } else {
            Engine::Oracle
        };
        let result = match moving_engine {
            Engine::Fast => {
                fast.search_with_context(&state, &repetition_history, &convergence_history)
            }
            Engine::Oracle => {
                oracle.search_with_context(&state, &repetition_history, &convergence_history)
            }
        };
        record_search(summary, moving_engine, &result);
        let mv = result
            .best_move
            .expect("a nonterminal state must have a legal move");

        repetition_history.push(state.repetition_hash());
        convergence_history.push(state.convergence_hash());
        state = state
            .apply_move(mv)
            .expect("a move generated by search must be legal");
        turns += 1;
    }

    summary.turns += turns as u64;
    let (winner, finish) = if let Some(winner) = state.winner() {
        (Some(winner), Finish::Terminal)
    } else {
        summary.capped_games += 1;
        let score = state.evaluate();
        let winner = if score > 0 {
            Some(state.side_to_move())
        } else if score < 0 {
            Some(state.side_to_move().opponent())
        } else {
            None
        };
        (
            winner,
            if winner.is_some() {
                Finish::Adjudicated
            } else {
                Finish::Draw
            },
        )
    };

    let winner_engine = winner.map(|player| {
        if player == fast_player {
            Engine::Fast
        } else {
            Engine::Oracle
        }
    });
    record_finish(summary, winner_engine, finish, fast_player);

    println!(
        "GAME seed={seed} fast_player={fast_player:?} oracle_player={:?} \
turns={turns} finish={finish:?} winner={winner_engine:?}",
        fast_player.opponent()
    );
}

fn record_search(
    summary: &mut Summary,
    engine: Engine,
    result: &snipe_ai::SearchResult<snipe_core::Move>,
) {
    let stats = match engine {
        Engine::Fast => &mut summary.fast,
        Engine::Oracle => &mut summary.oracle,
    };
    stats.moves += 1;
    stats.nodes += result.stats.nodes + result.stats.qnodes;
    stats.depth += u64::from(result.depth);
    stats.elapsed_micros += result.stats.elapsed.as_micros();
    stats.incomplete_iterations += u64::from(!result.completed_iteration);
}

fn record_finish(
    summary: &mut Summary,
    winner: Option<Engine>,
    finish: Finish,
    fast_player: Player,
) {
    let Some(winner) = winner else {
        summary.draws += 1;
        return;
    };
    let stats = match winner {
        Engine::Fast => &mut summary.fast,
        Engine::Oracle => &mut summary.oracle,
    };
    stats.wins += 1;
    match finish {
        Finish::Terminal => stats.terminal_wins += 1,
        Finish::Adjudicated => stats.adjudicated_wins += 1,
        Finish::Draw => unreachable!("a draw has no winner"),
    }
    let winner_player = match winner {
        Engine::Fast => fast_player,
        Engine::Oracle => fast_player.opponent(),
    };
    stats.wins_as_first += u64::from(winner_player == Player::Beta);
}

fn ratio(total: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        total as f64 / count as f64
    }
}

fn micros_per_move(stats: &EngineStats) -> f64 {
    if stats.moves == 0 {
        0.0
    } else {
        stats.elapsed_micros as f64 / stats.moves as f64 / 1_000.0
    }
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
