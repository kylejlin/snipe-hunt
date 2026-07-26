//! A deterministic, heuristic-guided Monte Carlo tree search challenger.
//!
//! This is intentionally independent from the production alpha-beta search.
//! It uses PUCT, progressive widening, and short stochastic heuristic rollouts
//! to sample a much wider set of complete turns than fixed-depth minimax.

use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
#[cfg(target_arch = "wasm32")]
use web_time::Instant;

use snipe_core::{activates_triplet, Move, State};

use crate::evaluate_state;

#[derive(Clone, Debug)]
pub struct MctsConfig {
    pub time_limit: Duration,
    pub max_iterations: u64,
    pub rollout_depth: u8,
    pub max_moves_per_node: usize,
    pub exploration: f64,
    pub widening: f64,
    pub seed: u64,
}

impl Default for MctsConfig {
    fn default() -> Self {
        Self {
            time_limit: Duration::from_secs(5),
            max_iterations: u64::MAX,
            rollout_depth: 5,
            max_moves_per_node: 72,
            exploration: 1.35,
            widening: 1.8,
            seed: 0x534e_4950_4548_554e,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct MctsStats {
    pub iterations: u64,
    pub nodes: usize,
    pub elapsed: Duration,
    pub root_children: usize,
}

#[derive(Clone, Debug)]
pub struct MctsResult {
    pub best_move: Option<Move>,
    /// Root-perspective value in `[-1, 1]`.
    pub value: f64,
    pub visits: u32,
    pub stats: MctsStats,
}

#[derive(Debug)]
struct Node {
    state: State,
    incoming: Option<Move>,
    children: Vec<usize>,
    /// Ordered worst-to-best so `pop()` expands the strongest remaining move.
    unexpanded: Vec<Move>,
    visits: u32,
    /// Mean value is always from this node's side-to-move perspective.
    value_sum: f64,
    prior: f64,
}

impl Node {
    fn new(state: State, incoming: Option<Move>, prior: f64, move_limit: usize) -> Self {
        let mut unexpanded = ordered_moves(state, move_limit);
        // `ordered_moves` is best-to-worst.
        unexpanded.reverse();
        Self {
            state,
            incoming,
            children: Vec::new(),
            unexpanded,
            visits: 0,
            value_sum: 0.0,
            prior,
        }
    }

    #[inline]
    fn mean(&self) -> f64 {
        if self.visits == 0 {
            0.0
        } else {
            self.value_sum / self.visits as f64
        }
    }
}

pub struct MctsEngine {
    config: MctsConfig,
    rng: SplitMix64,
}

impl MctsEngine {
    pub fn new(config: MctsConfig) -> Self {
        let rng = SplitMix64::new(config.seed);
        Self { config, rng }
    }

    pub fn config(&self) -> &MctsConfig {
        &self.config
    }

    pub fn set_time_limit(&mut self, time_limit: Duration) {
        self.config.time_limit = time_limit;
    }

    pub fn search(&mut self, root: State) -> MctsResult {
        let started = Instant::now();
        if root.winner().is_some() {
            return MctsResult {
                best_move: None,
                value: terminal_value(root),
                visits: 0,
                stats: MctsStats {
                    elapsed: started.elapsed(),
                    ..MctsStats::default()
                },
            };
        }

        let fallback = ordered_moves(root, 1).into_iter().next();
        if fallback.is_none() {
            return MctsResult {
                best_move: None,
                value: -1.0,
                visits: 0,
                stats: MctsStats {
                    elapsed: started.elapsed(),
                    ..MctsStats::default()
                },
            };
        }

        // Reseed per position: repeated analysis is reproducible and arena
        // seat order cannot perturb future searches.
        self.rng = SplitMix64::new(self.config.seed ^ root.position_hash());
        let mut tree = Vec::with_capacity(8_192);
        tree.push(Node::new(root, None, 1.0, self.config.max_moves_per_node));
        let deadline = started + self.config.time_limit;
        let mut iterations = 0_u64;

        while iterations < self.config.max_iterations && Instant::now() < deadline {
            iterations += 1;
            let mut path = Vec::with_capacity(64);
            let mut current = 0_usize;
            path.push(current);

            loop {
                if tree[current].state.winner().is_some() {
                    break;
                }

                let visits = tree[current].visits.max(1) as f64;
                let widening_limit = (self.config.widening * visits.sqrt()).ceil() as usize;
                let may_expand = !tree[current].unexpanded.is_empty()
                    && (tree[current].children.is_empty()
                        || tree[current].children.len() < widening_limit);

                if may_expand {
                    let mv = tree[current].unexpanded.pop().expect("checked non-empty");
                    let child_state = tree[current]
                        .state
                        .apply_move(mv)
                        .expect("generated move must apply");
                    let rank = tree[current].children.len();
                    let prior = 1.0 / ((rank + 2) as f64).sqrt();
                    let child = tree.len();
                    tree.push(Node::new(
                        child_state,
                        Some(mv),
                        prior,
                        self.config.max_moves_per_node,
                    ));
                    tree[current].children.push(child);
                    current = child;
                    path.push(current);
                    break;
                }

                let parent_visits = tree[current].visits.max(1);
                let Some(next) = tree[current]
                    .children
                    .iter()
                    .copied()
                    .max_by(|&left, &right| {
                        let left_score =
                            puct_score(&tree[left], parent_visits, self.config.exploration);
                        let right_score =
                            puct_score(&tree[right], parent_visits, self.config.exploration);
                        left_score
                            .total_cmp(&right_score)
                            .then_with(|| right.cmp(&left))
                    })
                else {
                    break;
                };
                current = next;
                path.push(current);
            }

            let mut value = self.rollout(tree[current].state);
            for &node_index in path.iter().rev() {
                let node = &mut tree[node_index];
                node.visits = node.visits.saturating_add(1);
                node.value_sum += value;
                value = -value;
            }
        }

        let best_child = tree[0].children.iter().copied().max_by(|&left, &right| {
            tree[left]
                .visits
                .cmp(&tree[right].visits)
                .then_with(|| (-tree[left].mean()).total_cmp(&(-tree[right].mean())))
                .then_with(|| right.cmp(&left))
        });
        let best_move = best_child
            .and_then(|index| tree[index].incoming)
            .or(fallback);
        let value = best_child.map(|index| -tree[index].mean()).unwrap_or(0.0);
        let visits = best_child.map(|index| tree[index].visits).unwrap_or(0);

        MctsResult {
            best_move,
            value,
            visits,
            stats: MctsStats {
                iterations,
                nodes: tree.len(),
                elapsed: started.elapsed(),
                root_children: tree[0].children.len(),
            },
        }
    }

    /// A shallow rollout mixes tactical greed with deterministic exploration.
    /// The return value is from the original state's side-to-move perspective.
    fn rollout(&mut self, mut state: State) -> f64 {
        let perspective = state.side_to_move();
        for _ in 0..self.config.rollout_depth {
            if let Some(winner) = state.winner() {
                return if winner == perspective { 1.0 } else { -1.0 };
            }
            let moves = ordered_moves(state, 24);
            if moves.is_empty() {
                return -1.0;
            }
            // One rollout in eight probes a non-greedy top candidate. This is
            // seeded and reproducible, unlike random playout MCTS.
            let index = if self.rng.next_u64() & 7 == 0 {
                self.rng.index(moves.len().min(8))
            } else {
                0
            };
            state = state
                .apply_move(moves[index])
                .expect("generated rollout move must apply");
        }

        if let Some(winner) = state.winner() {
            return if winner == perspective { 1.0 } else { -1.0 };
        }
        let score = evaluate_state(state) as f64;
        let perspective_score = if state.side_to_move() == perspective {
            score
        } else {
            -score
        };
        (perspective_score / 1_400.0).tanh()
    }
}

fn puct_score(child: &Node, parent_visits: u32, exploration: f64) -> f64 {
    let exploitation = -child.mean();
    let exploration =
        exploration * child.prior * (parent_visits as f64).sqrt() / (1.0 + child.visits as f64);
    exploitation + exploration
}

fn terminal_value(state: State) -> f64 {
    match state.winner() {
        Some(winner) if winner == state.side_to_move() => 1.0,
        Some(_) => -1.0,
        None => 0.0,
    }
}

fn ordered_moves(state: State, limit: usize) -> Vec<Move> {
    let mut moves = state.legal_moves();
    // Applying and evaluating every complete two-animal turn defeats MCTS on
    // the opening's very wide move list. This cheap prior still recognizes
    // triplet sweeps and snipe captures directly from the destination cell.
    moves.sort_unstable_by_key(|&mv| (std::cmp::Reverse(move_prior(state, mv)), mv));
    if limit != 0 && moves.len() > limit {
        moves.truncate(limit);
    }
    moves
}

fn move_prior(state: State, mv: Move) -> i32 {
    let player = state.side_to_move();
    match mv {
        Move::Snipe { destination } => 30 - (destination.number() as i32 * 2 - 7).abs(),
        Move::Drop {
            animal,
            destination,
        } => {
            let progress = match player {
                snipe_core::Player::Alpha => destination.number() as i32,
                snipe_core::Player::Beta => 7 - destination.number() as i32,
            };
            100 + progress * 5 + i32::from(!animal.can_retreat()) * 4
        }
        Move::Animals { first, second } => {
            let step_score = |step: snipe_core::AnimalStep| {
                let cell = state.cell(step.destination.location());
                let triplet = activates_triplet(cell.all_animals(), step.moved);
                let captures_snipe = triplet && cell.has_snipe(player.opponent());
                let progress = match player {
                    snipe_core::Player::Alpha => step.destination.number() as i32,
                    snipe_core::Player::Beta => 7 - step.destination.number() as i32,
                };
                i32::from(captures_snipe) * 100_000
                    + i32::from(triplet) * (10_000 + cell.card_count() as i32 * 500)
                    + progress * 7
            };
            200 + step_score(first) + second.map(step_score).unwrap_or(0)
        }
    }
}

#[derive(Clone, Copy)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    const fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    fn index(&mut self, length: usize) -> usize {
        (self.next_u64() as usize) % length
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_a_legal_move_even_with_no_budget() {
        let state = State::initial(7);
        let mut engine = MctsEngine::new(MctsConfig {
            time_limit: Duration::ZERO,
            ..MctsConfig::default()
        });
        let result = engine.search(state);
        assert!(state.legal_moves().contains(&result.best_move.unwrap()));
    }

    #[test]
    fn fixed_iteration_search_is_reproducible() {
        let state = State::initial(19);
        let config = MctsConfig {
            time_limit: Duration::from_secs(10),
            max_iterations: 80,
            rollout_depth: 3,
            ..MctsConfig::default()
        };
        let first = MctsEngine::new(config.clone()).search(state);
        let second = MctsEngine::new(config).search(state);
        assert_eq!(first.best_move, second.best_move);
        assert_eq!(first.stats.iterations, 80);
        assert_eq!(second.stats.iterations, 80);
    }
}
