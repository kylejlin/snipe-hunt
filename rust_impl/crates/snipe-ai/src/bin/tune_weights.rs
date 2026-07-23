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
use snipe_core::{activates_triplet, Animal, AtomicMove, Location, Move, Player, Row, State};

#[derive(Clone, Copy, Debug, Default)]
struct NovelWeights {
    reserve_floor: i32,
    versatile_animals: i32,
    latent_triplets: i32,
    snipe_crowding: i32,
    reserve_retreaters: i32,
    unique_types: i32,
    animal_turn_ready: i32,
    waiting_turns: i32,
    binary_snipe_danger: i32,
}

#[derive(Clone)]
struct WeightedState {
    state: State,
    weights: SnipeWeights,
    novel: NovelWeights,
}

impl WeightedState {
    fn new(state: State, weights: SnipeWeights, novel: NovelWeights) -> Self {
        Self {
            state,
            weights,
            novel,
        }
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
            self.novel,
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
        self.weights.evaluate(extract_features(self.state)) + novel_score(self.state, self.novel)
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
    novel: NovelWeights,
    engine: SearchEngine<WeightedState>,
}

impl Contestant {
    fn new(weights: SnipeWeights, novel: NovelWeights, depth: u8) -> Self {
        Self {
            weights,
            novel,
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
        let result = self
            .engine
            .search(&WeightedState::new(state, self.weights, self.novel));
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

#[allow(clippy::too_many_arguments)]
fn play_weight_match(
    first_weights: SnipeWeights,
    first_novel: NovelWeights,
    second_weights: SnipeWeights,
    second_novel: NovelWeights,
    first_seed: u64,
    seeds: u64,
    depth: u8,
    max_turns: usize,
) -> Summary {
    let mut first = Contestant::new(first_weights, first_novel, depth);
    let mut second = Contestant::new(second_weights, second_novel, depth);
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
    novel: NovelWeights,
    first_seed: u64,
    seeds: u64,
    depth: u8,
    max_turns: usize,
) -> Summary {
    let mut candidate = Contestant::new(weights, novel, depth);
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

fn novel_score(state: State, weights: NovelWeights) -> i32 {
    let me = state.side_to_move();
    let them = me.opponent();
    weights.reserve_floor
        * (i32::from(state.reserve_count(me) > 1) - i32::from(state.reserve_count(them) > 1))
        + weights.versatile_animals
            * (versatile_animals(state, me) - versatile_animals(state, them))
        + weights.latent_triplets * (latent_triplets(state, me) - latent_triplets(state, them))
        + weights.snipe_crowding * (snipe_crowding(state, them) - snipe_crowding(state, me))
        + weights.reserve_retreaters
            * (reserve_retreaters(state, them) - reserve_retreaters(state, me))
        + weights.unique_types * (unique_types(state, me) - unique_types(state, them))
        + weights.animal_turn_ready
            * (animal_turn_ready(state, me) - animal_turn_ready(state, them))
        + weights.waiting_turns * (waiting_turns(state, me) - waiting_turns(state, them))
        + weights.binary_snipe_danger
            * (i32::from(snipe_danger(state, them) > 0) - i32::from(snipe_danger(state, me) > 0))
}

fn versatile_animals(state: State, player: Player) -> i32 {
    Animal::ALL
        .into_iter()
        .filter(|&animal| {
            state.owner_of_animal(animal) == Some(player)
                && !matches!(animal.index() & 15, 2 | 4 | 12 | 13)
        })
        .count() as i32
}

fn latent_triplets(state: State, player: Player) -> i32 {
    let mut count = 0;
    for animal in Animal::ALL {
        if state.owner_of_animal(animal) != Some(player) {
            continue;
        }
        let source = state.location_of_animal(animal);
        for row in Row::ALL {
            if source == Some(row.location()) {
                continue;
            }
            let is_latent = match source.and_then(Location::row) {
                Some(source_row) => (source_row.number() as i32 - row.number() as i32).abs() > 1,
                None => true,
            };
            if is_latent && activates_triplet(state.cell(row.location()).all_animals(), animal) {
                count += 1;
            }
        }
    }
    count
}

fn snipe_crowding(state: State, player: Player) -> i32 {
    state
        .snipe_location(player)
        .and_then(Location::row)
        .map(|row| (state.cell(row.location()).all_animals().count_ones() as i32 - 2).max(0))
        .unwrap_or(0)
}

fn reserve_retreaters(state: State, player: Player) -> i32 {
    let reserve = Location::reserve_of(player);
    Animal::ALL
        .into_iter()
        .filter(|&animal| {
            animal.can_retreat()
                && state.owner_of_animal(animal) == Some(player)
                && state.location_of_animal(animal) == Some(reserve)
        })
        .count() as i32
}

fn unique_types(state: State, player: Player) -> i32 {
    let mut types = 0_u16;
    for animal in Animal::ALL {
        if state.owner_of_animal(animal) == Some(player) {
            types |= 1 << (animal.index() & 15);
        }
    }
    types.count_ones() as i32
}

fn state_for_player(state: State, player: Player) -> State {
    if state.side_to_move() == player {
        return state;
    }
    let mut data = state.to_data();
    data.side_to_move = player as u8;
    data.pending_animal = u8::MAX;
    data.pending_destination = 0;
    State::from_data(data).expect("changing only side-to-move preserves validity")
}

fn atomic_profile(state: State, player: Player) -> (u32, i32) {
    let state = state_for_player(state, player);
    let mut movable = 0_u32;
    let mut waiting = 0;
    for mv in state.legal_atomics() {
        match mv {
            AtomicMove::Animal(step) => movable |= step.moved.bit(),
            AtomicMove::Snipe { .. } => waiting += 1,
            AtomicMove::Drop { .. } => waiting += 1,
        }
    }
    (movable, waiting)
}

fn animal_turn_ready(state: State, player: Player) -> i32 {
    i32::from(atomic_profile(state, player).0.count_ones() >= 2)
}

fn waiting_turns(state: State, player: Player) -> i32 {
    let waiting = atomic_profile(state, player).1;
    // Availability matters more than dozens of nearly equivalent drops.
    waiting.min(3)
}

fn snipe_danger(state: State, defender: Player) -> i32 {
    let attacker = defender.opponent();
    let Some(target) = state.snipe_location(defender).and_then(Location::row) else {
        return 0;
    };
    let target_cell = state.cell(target.location());
    Animal::ALL
        .into_iter()
        .filter(|&animal| {
            if state.owner_of_animal(animal) != Some(attacker) {
                return false;
            }
            let Some(source) = state.location_of_animal(animal).and_then(Location::row) else {
                return false;
            };
            let reaches = source.forward(attacker) == Some(target)
                || (animal.can_retreat() && source.backward(attacker) == Some(target));
            reaches
                && activates_triplet(target_cell.all_animals(), animal)
                && (state.cell(source.location()).card_count() > 1
                    || target_cell.has_snipe(defender))
        })
        .count() as i32
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
    if matches!(mode.as_str(), "features" | "feature_best" | "features2") {
        let feature_candidates = [
            (
                "reserve_floor_40",
                NovelWeights {
                    reserve_floor: 40,
                    ..NovelWeights::default()
                },
            ),
            (
                "reserve_floor_80",
                NovelWeights {
                    reserve_floor: 80,
                    ..NovelWeights::default()
                },
            ),
            (
                "versatile_12",
                NovelWeights {
                    versatile_animals: 12,
                    ..NovelWeights::default()
                },
            ),
            (
                "latent_4",
                NovelWeights {
                    latent_triplets: 4,
                    ..NovelWeights::default()
                },
            ),
            (
                "latent_8",
                NovelWeights {
                    latent_triplets: 8,
                    ..NovelWeights::default()
                },
            ),
            (
                "crowding_20",
                NovelWeights {
                    snipe_crowding: 20,
                    ..NovelWeights::default()
                },
            ),
            (
                "crowding_40",
                NovelWeights {
                    snipe_crowding: 40,
                    ..NovelWeights::default()
                },
            ),
            (
                "reserve_retreaters_15",
                NovelWeights {
                    reserve_retreaters: 15,
                    ..NovelWeights::default()
                },
            ),
            (
                "feature_combo",
                NovelWeights {
                    reserve_floor: 40,
                    versatile_animals: 8,
                    latent_triplets: 4,
                    snipe_crowding: 20,
                    reserve_retreaters: 10,
                    ..NovelWeights::default()
                },
            ),
            (
                "unique_types_20",
                NovelWeights {
                    unique_types: 20,
                    ..NovelWeights::default()
                },
            ),
            (
                "animal_turn_ready_50",
                NovelWeights {
                    animal_turn_ready: 50,
                    ..NovelWeights::default()
                },
            ),
            (
                "animal_turn_ready_100",
                NovelWeights {
                    animal_turn_ready: 100,
                    ..NovelWeights::default()
                },
            ),
            (
                "waiting_turns_15",
                NovelWeights {
                    waiting_turns: 15,
                    ..NovelWeights::default()
                },
            ),
            (
                "waiting_turns_30",
                NovelWeights {
                    waiting_turns: 30,
                    ..NovelWeights::default()
                },
            ),
            (
                "binary_danger_250",
                NovelWeights {
                    binary_snipe_danger: 250,
                    ..NovelWeights::default()
                },
            ),
        ];
        for (name, novel) in feature_candidates {
            if mode == "feature_best"
                && !matches!(name, "versatile_12" | "crowding_20" | "crowding_40")
            {
                continue;
            }
            if mode == "features2"
                && !matches!(
                    name,
                    "unique_types_20"
                        | "animal_turn_ready_50"
                        | "animal_turn_ready_100"
                        | "waiting_turns_15"
                        | "waiting_turns_30"
                        | "binary_danger_250"
                )
            {
                continue;
            }
            let result = play_weight_match(
                baseline,
                novel,
                baseline,
                NovelWeights::default(),
                first_seed,
                seeds,
                depth,
                max_turns,
            );
            print_result(name, &result);
        }
        return;
    }
    for (name, candidate) in candidates {
        if (mode == "finalists"
            && !matches!(name, "triplets_75" | "triplet_material" | "positive_combo"))
            || (mode == "best" && name != "positive_combo")
        {
            continue;
        }
        let result = play_weight_match(
            candidate,
            NovelWeights::default(),
            baseline,
            NovelWeights::default(),
            first_seed,
            seeds,
            depth,
            max_turns,
        );
        print_result(name, &result);
    }
    let baseline_greedy = play_greedy_match(
        baseline,
        NovelWeights::default(),
        first_seed,
        seeds,
        depth,
        max_turns,
    );
    print_result("baseline vs greedy", &baseline_greedy);
    for (name, candidate) in candidates {
        if (mode == "suite" && (name == "tactical" || name == "survival"))
            || (mode == "finalists"
                && matches!(name, "triplets_75" | "triplet_material" | "positive_combo"))
            || (mode == "best" && name == "positive_combo")
        {
            let result = play_greedy_match(
                candidate,
                NovelWeights::default(),
                first_seed,
                seeds,
                depth,
                max_turns,
            );
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
