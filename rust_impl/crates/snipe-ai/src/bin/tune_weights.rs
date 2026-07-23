//! Deterministic, fixed-depth evaluation-weight tournament.
//!
//! This is deliberately a standalone development tool: production continues
//! to use `SnipeWeights::default()`.  Every match uses each deal twice with the
//! contestants swapping move order, so deal and first-player luck cancel out.

use std::time::Duration;

use snipe_ai::{
    evaluate_state, extract_features, tactical_move_score, GamePosition, SearchConfig,
    SearchEngine, SnipeWeights, MATE_SCORE,
};
use snipe_core::{Location, Move, Player, State};

#[derive(Clone)]
struct WeightedState {
    state: State,
    weights: SnipeWeights,
}

impl WeightedState {
    fn new(state: State, weights: SnipeWeights) -> Self {
        Self { state, weights }
    }
}

impl GamePosition for WeightedState {
    type Move = Move;

    fn legal_moves(&self, moves: &mut Vec<Move>) {
        State::legal_moves_into(self.state, moves);
    }

    fn apply_move(&self, mv: Move) -> Self {
        Self::new(
            self.state
                .apply_move(mv)
                .expect("rules-generated move must apply"),
            self.weights,
        )
    }

    fn position_hash(&self) -> u64 {
        self.state.position_hash()
    }

    fn terminal_score(&self) -> Option<i32> {
        self.state.winner().map(|winner| {
            if winner == self.state.side_to_move() {
                MATE_SCORE
            } else {
                -MATE_SCORE
            }
        })
    }

    fn evaluate(&self) -> i32 {
        self.weights.evaluate(extract_features(self.state))
    }

    fn move_ordering_score(&self, mv: Move, child: &Self) -> i32 {
        // Retain all of production's tactical ordering, changing only the
        // evaluator used for its quiet-move tie-break.
        tactical_move_score(self.state, mv, child.state) + evaluate_state(child.state) / 4
            - child.evaluate() / 4
    }

    fn is_tactical(&self, _mv: Move, child: &Self) -> bool {
        if child.state.captured_snipe_winner().is_some() {
            return true;
        }
        let player = self.state.side_to_move();
        let reserve = Location::reserve_of(player);
        let before = self.state.animal_bits(reserve, player);
        let after = child.state.animal_bits(reserve, player);
        after & !before != 0
    }
}

struct Contestant {
    weights: SnipeWeights,
    engine: SearchEngine<WeightedState>,
}

impl Contestant {
    fn new(weights: SnipeWeights, depth: u8) -> Self {
        Self {
            weights,
            engine: SearchEngine::new(SearchConfig {
                // Fixed depth, not wall-clock speed, determines every move.
                // One hour simply makes the deadline irrelevant.
                time_limit: Duration::from_secs(3_600),
                max_depth: depth,
                quiescence_depth: 2,
                transposition_table_mb: 16,
                selective_move_limit: 48,
                ..SearchConfig::default()
            }),
        }
    }

    fn choose(&mut self, state: State) -> (Move, u64) {
        let result = self.engine.search(&WeightedState::new(state, self.weights));
        assert!(
            result.depth == self.engine.config().max_depth || result.score.abs() >= 990_000,
            "fixed-depth iteration unexpectedly stopped at depth {} with score {}",
            result.depth,
            result.score
        );
        (
            result
                .best_move
                .expect("a nonterminal Snipe Hunt position has a move"),
            result.stats.nodes + result.stats.qnodes,
        )
    }
}

#[derive(Default)]
struct Summary {
    first_wins: u32,
    second_wins: u32,
    draws: u32,
    turns: u64,
    nodes: u64,
}

impl Summary {
    fn score(&self) -> f64 {
        let games = self.first_wins + self.second_wins + self.draws;
        (self.first_wins as f64 + 0.5 * self.draws as f64) / games as f64
    }
}

fn play_weight_match(
    first_weights: SnipeWeights,
    second_weights: SnipeWeights,
    first_seed: u64,
    seeds: u64,
    depth: u8,
    max_turns: usize,
) -> Summary {
    let mut first = Contestant::new(first_weights, depth);
    let mut second = Contestant::new(second_weights, depth);
    let mut summary = Summary::default();
    for seed in first_seed..first_seed + seeds {
        for first_moves_first in [true, false] {
            let mut state = State::initial(seed);
            let mut turns = 0;
            while state.winner().is_none() && turns < max_turns {
                let first_to_move = (turns & 1 == 0) == first_moves_first;
                let (mv, nodes) = if first_to_move {
                    first.choose(state)
                } else {
                    second.choose(state)
                };
                summary.nodes += nodes;
                state = state.apply_move(mv).expect("selected move must apply");
                turns += 1;
            }
            summary.turns += turns as u64;
            match state.winner() {
                Some(winner) => {
                    let winner_moved_first = winner == Player::Beta;
                    if winner_moved_first == first_moves_first {
                        summary.first_wins += 1;
                    } else {
                        summary.second_wins += 1;
                    }
                }
                None => summary.draws += 1,
            }
        }
    }
    summary
}

fn play_greedy_match(
    weights: SnipeWeights,
    first_seed: u64,
    seeds: u64,
    depth: u8,
    max_turns: usize,
) -> Summary {
    let mut candidate = Contestant::new(weights, depth);
    let mut summary = Summary::default();
    for seed in first_seed..first_seed + seeds {
        for candidate_moves_first in [true, false] {
            let mut state = State::initial(seed);
            let mut turns = 0;
            while state.winner().is_none() && turns < max_turns {
                let candidate_to_move = (turns & 1 == 0) == candidate_moves_first;
                let mv = if candidate_to_move {
                    let (mv, nodes) = candidate.choose(state);
                    summary.nodes += nodes;
                    mv
                } else {
                    greedy_move(state)
                };
                state = state.apply_move(mv).expect("selected move must apply");
                turns += 1;
            }
            summary.turns += turns as u64;
            match state.winner() {
                Some(winner) => {
                    let winner_moved_first = winner == Player::Beta;
                    if winner_moved_first == candidate_moves_first {
                        summary.first_wins += 1;
                    } else {
                        summary.second_wins += 1;
                    }
                }
                None => summary.draws += 1,
            }
        }
    }
    summary
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
        .expect("nonterminal state has a move")
}

fn print_result(label: &str, summary: &Summary) {
    println!(
        "{label:<28} W/L/D={}/{}/{} score={:.1}% avg_turns={:.1} nodes={}",
        summary.first_wins,
        summary.second_wins,
        summary.draws,
        summary.score() * 100.0,
        summary.turns as f64 / (summary.first_wins + summary.second_wins + summary.draws) as f64,
        summary.nodes,
    );
}

#[allow(clippy::too_many_arguments)]
fn weights(
    material: i32,
    reserve: i32,
    mobility: i32,
    progress: i32,
    retreaters: i32,
    near_triplets: i32,
    capture_pressure: i32,
    snipe_pressure: i32,
    snipe_liberties: i32,
    row_freedom: i32,
) -> SnipeWeights {
    SnipeWeights {
        material,
        reserve,
        mobility,
        progress,
        retreaters,
        near_triplets,
        capture_pressure,
        snipe_pressure,
        snipe_liberties,
        row_freedom,
    }
}

fn main() {
    let seeds = argument(1, 6_u64);
    let depth = argument(2, 1_u8);
    let max_turns = argument(3, 220_usize);
    let first_seed = argument(4, 0_u64);
    let mode = std::env::args()
        .nth(5)
        .unwrap_or_else(|| "suite".to_owned());
    let baseline = SnipeWeights::default();
    let candidates = [
        (
            "material_low",
            weights(80, 18, 3, 8, 10, 34, 55, 310, 42, 24),
        ),
        (
            "material_high",
            weights(180, 18, 3, 8, 10, 34, 55, 310, 42, 24),
        ),
        (
            "reserve_high",
            weights(120, 32, 3, 8, 10, 34, 55, 310, 42, 24),
        ),
        (
            "mobility_high",
            weights(120, 18, 7, 8, 10, 34, 55, 310, 42, 24),
        ),
        (
            "progress_high",
            weights(120, 18, 3, 16, 10, 34, 55, 310, 42, 24),
        ),
        (
            "triplets_high",
            weights(120, 18, 3, 8, 10, 60, 55, 310, 42, 24),
        ),
        (
            "triplets_45",
            weights(120, 18, 3, 8, 10, 45, 55, 310, 42, 24),
        ),
        (
            "triplets_75",
            weights(120, 18, 3, 8, 10, 75, 55, 310, 42, 24),
        ),
        (
            "triplets_100",
            weights(120, 18, 3, 8, 10, 100, 55, 310, 42, 24),
        ),
        (
            "capture_high",
            weights(120, 18, 3, 8, 10, 34, 90, 310, 42, 24),
        ),
        (
            "snipe_high",
            weights(120, 18, 3, 8, 10, 34, 55, 500, 42, 24),
        ),
        (
            "liberties_high",
            weights(120, 18, 3, 8, 10, 34, 55, 310, 75, 24),
        ),
        (
            "freedom_high",
            weights(120, 18, 3, 8, 10, 34, 55, 310, 42, 45),
        ),
        ("tactical", weights(150, 18, 2, 7, 10, 55, 90, 450, 60, 24)),
        ("survival", weights(115, 18, 3, 7, 10, 40, 65, 600, 90, 28)),
        (
            "triplet_material",
            weights(80, 18, 3, 8, 10, 60, 55, 310, 42, 24),
        ),
        (
            "triplet_snipe",
            weights(120, 18, 3, 8, 10, 60, 55, 500, 42, 24),
        ),
        (
            "triplet_freedom",
            weights(120, 18, 3, 8, 10, 60, 55, 310, 42, 45),
        ),
        (
            "positive_combo",
            weights(90, 18, 3, 8, 10, 60, 55, 450, 42, 36),
        ),
    ];

    println!(
        "paired seeds={first_seed}..{} games={} depth={depth} max_turns={max_turns}",
        first_seed + seeds,
        seeds * 2
    );
    for (name, candidate) in candidates {
        if (mode == "finalists"
            && !matches!(name, "triplets_75" | "triplet_material" | "positive_combo"))
            || (mode == "best" && name != "positive_combo")
        {
            continue;
        }
        let result = play_weight_match(candidate, baseline, first_seed, seeds, depth, max_turns);
        print_result(name, &result);
    }
    let baseline_greedy = play_greedy_match(baseline, first_seed, seeds, depth, max_turns);
    print_result("baseline vs greedy", &baseline_greedy);
    for (name, candidate) in candidates {
        if (mode == "suite" && (name == "tactical" || name == "survival"))
            || (mode == "finalists"
                && matches!(name, "triplets_75" | "triplet_material" | "positive_combo"))
            || (mode == "best" && name == "positive_combo")
        {
            let result = play_greedy_match(candidate, first_seed, seeds, depth, max_turns);
            print_result(&format!("{name} vs greedy"), &result);
        }
    }
}

fn argument<T: std::str::FromStr>(index: usize, default: T) -> T {
    std::env::args()
        .nth(index)
        .and_then(|value| value.parse().ok())
        .unwrap_or(default)
}
