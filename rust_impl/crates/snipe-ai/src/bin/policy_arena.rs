use std::time::Duration;

use snipe_ai::{GamePosition, SearchConfig, SearchEngine, SearchPolicy};
use snipe_core::{Player, State};

#[derive(Default)]
struct Summary {
    baseline_wins: u64,
    candidate_wins: u64,
    draws: u64,
    turns: u64,
    baseline_moves: u64,
    candidate_moves: u64,
    baseline_nodes: u64,
    candidate_nodes: u64,
    baseline_depth: u64,
    candidate_depth: u64,
}

fn main() {
    let pairs = argument(1, 30_u64);
    let node_limit = argument(2, 20_000_u64);
    let candidate_name = std::env::args().nth(3).unwrap_or_else(|| "tt".to_owned());
    let max_turns = argument(4, 120_usize);

    let baseline_policy = SearchPolicy {
        node_limit: Some(node_limit),
        ..SearchPolicy::default()
    };
    let candidate_policy = SearchPolicy {
        protect_deep_tt_entries: matches!(candidate_name.as_str(), "tt" | "both"),
        qsearch_direct_snipe_threats: matches!(
            candidate_name.as_str(),
            "threat" | "both" | "threat-defense"
        ),
        preserve_critical_snipe_defenses: matches!(
            candidate_name.as_str(),
            "defense" | "threat-defense"
        ),
        node_limit: Some(node_limit),
    };
    assert!(
        matches!(
            candidate_name.as_str(),
            "tt" | "threat" | "both" | "defense" | "threat-defense"
        ),
        "candidate must be tt, threat, both, defense, or threat-defense"
    );

    let config = SearchConfig {
        // The deterministic node limit ends each search. This distant deadline
        // remains solely as a safety valve for unexpectedly expensive root
        // move generation, which is outside the recursive node counter.
        time_limit: Duration::from_secs(60),
        max_depth: 64,
        transposition_table_mb: 64,
        deadline_check_interval: 1,
        ..SearchConfig::default()
    };
    let mut baseline = SearchEngine::<State>::new_with_policy(config.clone(), baseline_policy);
    let mut candidate = SearchEngine::<State>::new_with_policy(config, candidate_policy);
    let mut summary = Summary::default();

    for seed in 0..pairs {
        for mirror in 0..2 {
            play_one(
                State::initial(seed),
                mirror == 0,
                &mut baseline,
                &mut candidate,
                max_turns,
                &mut summary,
            );
        }
    }

    let baseline_nodes_per_turn = ratio(summary.baseline_nodes, summary.baseline_moves);
    let candidate_nodes_per_turn = ratio(summary.candidate_nodes, summary.candidate_moves);
    let baseline_depth = ratio(summary.baseline_depth, summary.baseline_moves);
    let candidate_depth = ratio(summary.candidate_depth, summary.candidate_moves);
    let games = summary.baseline_wins + summary.candidate_wins + summary.draws;
    println!(
        "RESULT candidate={candidate_name} pairs={pairs} node_limit={node_limit} \
baseline_wins={} candidate_wins={} draws={} avg_turns={:.1} \
baseline_nodes_per_turn={baseline_nodes_per_turn:.1} \
candidate_nodes_per_turn={candidate_nodes_per_turn:.1} \
baseline_depth={baseline_depth:.2} candidate_depth={candidate_depth:.2}",
        summary.baseline_wins,
        summary.candidate_wins,
        summary.draws,
        summary.turns as f64 / games as f64,
    );
}

fn play_one(
    mut state: State,
    baseline_moves_first: bool,
    baseline: &mut SearchEngine<State>,
    candidate: &mut SearchEngine<State>,
    max_turns: usize,
    summary: &mut Summary,
) {
    let mut turns = 0;
    while state.winner().is_none() && turns < max_turns {
        let use_baseline = (turns & 1 == 0) == baseline_moves_first;
        let result = if use_baseline {
            baseline.search(&state)
        } else {
            candidate.search(&state)
        };
        let mv = result
            .best_move
            .expect("a nonterminal state must have a legal move");
        let searched_nodes = result.stats.nodes + result.stats.qnodes;
        if use_baseline {
            summary.baseline_moves += 1;
            summary.baseline_nodes += searched_nodes;
            summary.baseline_depth += u64::from(result.depth);
        } else {
            summary.candidate_moves += 1;
            summary.candidate_nodes += searched_nodes;
            summary.candidate_depth += u64::from(result.depth);
        }
        state = state.apply_move(mv).expect("generated move must apply");
        turns += 1;
    }
    summary.turns += turns as u64;

    let winner = state.winner().or_else(|| {
        let score = state.evaluate();
        if score > 0 {
            Some(state.side_to_move())
        } else if score < 0 {
            Some(state.side_to_move().opponent())
        } else {
            None
        }
    });
    let Some(winner) = winner else {
        summary.draws += 1;
        return;
    };
    let winner_moved_first = winner == Player::Beta;
    if winner_moved_first == baseline_moves_first {
        summary.baseline_wins += 1;
    } else {
        summary.candidate_wins += 1;
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
