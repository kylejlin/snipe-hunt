//! Avocado: a deterministic, patient Snipe Hunt calculator.
//!
//! The search and evaluation in this crate are intentionally private and
//! independent. Only `snipe-core` types cross its public boundary.

use snipe_core::{
    Action, ActionWriter, Analyzer, Animal, Card, Evaluation, EvaluationEstimate, MateInN, Player,
    State,
};
use std::collections::HashMap;

const MATE: f64 = 1_000_000.0;
const MAX_DEPTH: usize = 4;

#[derive(Clone)]
struct Candidate {
    actions: Vec<Action>,
    state: State,
    score: f64,
    continuation: Vec<Action>,
}

#[derive(Clone)]
struct SearchResult {
    score: f64,
    line: Vec<Action>,
}

impl SearchResult {
    fn leaf(score: f64) -> Self {
        Self {
            score,
            line: Vec::new(),
        }
    }
}

/// A deterministic, positionally conservative analyzer.
pub struct AvocadoAnalyzer {
    root: Option<State>,
    candidates: Vec<Candidate>,
    best: usize,
    cursor: usize,
    depth: usize,
    evaluation: Evaluation,
    transpositions: HashMap<(u64, usize, u32), SearchResult>,
}

impl Default for AvocadoAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl AvocadoAnalyzer {
    pub fn new() -> Self {
        Self {
            root: None,
            candidates: Vec::new(),
            best: 0,
            cursor: 0,
            depth: 1,
            evaluation: estimate(0.0),
            transpositions: HashMap::with_capacity(4_096),
        }
    }

    fn refresh_best(&mut self) {
        let Some(root) = &self.root else { return };
        if self.candidates.is_empty() {
            self.evaluation =
                terminal_evaluation(root, 0).unwrap_or_else(|| estimate(evaluate(root)));
            return;
        }
        let alpha = root.active_player == Player::Alpha;
        self.best = (0..self.candidates.len())
            .max_by(|&left, &right| {
                let ordering = self.candidates[left]
                    .score
                    .total_cmp(&self.candidates[right].score);
                if alpha { ordering } else { ordering.reverse() }
            })
            .unwrap_or(0);
        self.evaluation = score_to_evaluation(self.candidates[self.best].score);
    }
}

impl Analyzer for AvocadoAnalyzer {
    fn set_state(&mut self, state: State) {
        self.root = Some(state.clone());
        self.candidates = full_plies(&state)
            .into_iter()
            .filter_map(|actions| {
                apply_actions(state.clone(), &actions).map(|child| Candidate {
                    score: terminal_score(&child, 1).unwrap_or_else(|| evaluate(&child)),
                    actions,
                    state: child,
                    continuation: Vec::new(),
                })
            })
            .collect();
        self.best = 0;
        self.cursor = 0;
        self.depth = 1;
        self.evaluation =
            terminal_evaluation(&state, 0).unwrap_or_else(|| estimate(evaluate(&state)));
        self.transpositions.clear();
        self.refresh_best();
    }

    fn think_for_one_tick(&mut self) {
        if self.candidates.is_empty() {
            return;
        }
        let index = self.cursor;
        let mut visited = 0;
        let state = self.candidates[index].state.clone();
        let result = minimax(
            &state,
            self.depth.saturating_sub(1),
            1,
            -MATE,
            MATE,
            &mut visited,
            384,
            &mut self.transpositions,
        );
        self.candidates[index].score = result.score;
        self.candidates[index].continuation = result.line;
        self.cursor += 1;
        if self.cursor == self.candidates.len() {
            self.cursor = 0;
            self.depth = (self.depth + 1).min(MAX_DEPTH);
        }
        self.refresh_best();
    }

    fn evaluation(&self) -> Evaluation {
        self.evaluation
    }

    fn write_optimal_lop<W: ActionWriter>(&self, writer: &mut W) {
        if let Some(candidate) = self.candidates.get(self.best) {
            writer.reserve(candidate.actions.len() + candidate.continuation.len());
            for &action in candidate.actions.iter().chain(&candidate.continuation) {
                writer.push(action);
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn minimax(
    state: &State,
    depth: usize,
    plies_from_root: u32,
    mut alpha: f64,
    mut beta: f64,
    visited: &mut usize,
    budget: usize,
    transpositions: &mut HashMap<(u64, usize, u32), SearchResult>,
) -> SearchResult {
    *visited += 1;
    if let Some(score) = terminal_score(state, plies_from_root) {
        return SearchResult::leaf(score);
    }
    if depth == 0 || *visited >= budget {
        return SearchResult::leaf(evaluate(state));
    }
    let key = (fingerprint(state), depth, plies_from_root);
    if let Some(cached) = transpositions.get(&key) {
        return cached.clone();
    }
    let moves = full_plies(state);
    if moves.is_empty() {
        return SearchResult::leaf(evaluate(state));
    }
    if state.active_player == Player::Alpha {
        let mut best = SearchResult::leaf(-MATE);
        for actions in moves {
            let Some(child) = apply_actions(state.clone(), &actions) else {
                continue;
            };
            let child_result = minimax(
                &child,
                depth - 1,
                plies_from_root + 1,
                alpha,
                beta,
                visited,
                budget,
                transpositions,
            );
            if child_result.score > best.score {
                let mut line = actions;
                line.extend(child_result.line);
                best = SearchResult {
                    score: child_result.score,
                    line,
                };
            }
            alpha = alpha.max(best.score);
            if alpha >= beta || *visited >= budget {
                break;
            }
        }
        transpositions.insert(key, best.clone());
        best
    } else {
        let mut best = SearchResult::leaf(MATE);
        for actions in moves {
            let Some(child) = apply_actions(state.clone(), &actions) else {
                continue;
            };
            let child_result = minimax(
                &child,
                depth - 1,
                plies_from_root + 1,
                alpha,
                beta,
                visited,
                budget,
                transpositions,
            );
            if child_result.score < best.score {
                let mut line = actions;
                line.extend(child_result.line);
                best = SearchResult {
                    score: child_result.score,
                    line,
                };
            }
            beta = beta.min(best.score);
            if alpha >= beta || *visited >= budget {
                break;
            }
        }
        transpositions.insert(key, best.clone());
        best
    }
}

fn fingerprint(state: &State) -> u64 {
    let locations = [
        &state.reserves,
        &state.r1,
        &state.r2,
        &state.r3,
        &state.r4,
        &state.r5,
        &state.r6,
    ];
    let mut hash: u64 = if state.active_player == Player::Alpha {
        0xA11F_A11F
    } else {
        0xBE7A_BE7A
    };
    for (location, cards) in locations.into_iter().enumerate() {
        for (index, animal) in animals().into_iter().enumerate() {
            let packed = u64::from(cards.count(Card::Animal(animal), Player::Alpha))
                | (u64::from(cards.count(Card::Animal(animal), Player::Beta)) << 2);
            hash ^= packed.wrapping_add((location * 19 + index) as u64);
            hash = hash.rotate_left(7).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
        for player in [Player::Alpha, Player::Beta] {
            hash ^= u64::from(cards.count(Card::Snipe, player));
            hash = hash.rotate_left(3);
        }
    }
    hash
}

fn evaluate(state: &State) -> f64 {
    if let Some(score) = terminal_score(state, 0) {
        return score;
    }
    let ranks = [
        &state.r1, &state.r2, &state.r3, &state.r4, &state.r5, &state.r6,
    ];
    let mut score = 0.0;
    let mut alpha_reserve = 0u32;
    let mut beta_reserve = 0u32;
    for animal in animals() {
        alpha_reserve += u32::from(state.reserves.count(Card::Animal(animal), Player::Alpha));
        beta_reserve += u32::from(state.reserves.count(Card::Animal(animal), Player::Beta));
    }
    score += (f64::from(alpha_reserve) - f64::from(beta_reserve)) * 3.0;
    for (index, cards) in ranks.into_iter().enumerate() {
        let center = 3.5 - ((index + 1) as f64 - 3.5).abs();
        for animal in animals() {
            let alpha = f64::from(cards.count(Card::Animal(animal), Player::Alpha));
            let beta = f64::from(cards.count(Card::Animal(animal), Player::Beta));
            score += (alpha - beta) * (2.0 + center * 0.7);
        }
        if cards.count(Card::Snipe, Player::Alpha) != 0 {
            score += 18.0 - index as f64 * 1.8;
        }
        if cards.count(Card::Snipe, Player::Beta) != 0 {
            score -= 18.0 - (5 - index) as f64 * 1.8;
        }
    }
    let mobility = legal_actions(state).len() as f64;
    score
        + if state.active_player == Player::Alpha {
            mobility * 0.04
        } else {
            -mobility * 0.04
        }
}

fn full_plies(state: &State) -> Vec<Vec<Action>> {
    let player = state.active_player;
    let mut result = Vec::new();
    for first in legal_actions(state) {
        let Ok(after) = state.clone().apply(first) else {
            continue;
        };
        if after.active_player != player || after.winner().is_some() {
            result.push(vec![first]);
        } else {
            for second in legal_actions(&after) {
                if after.clone().apply(second).is_ok() {
                    result.push(vec![first, second]);
                }
            }
        }
    }
    result
}

fn legal_actions(state: &State) -> Vec<Action> {
    let mut actions = Vec::new();
    state.write_legal_actions(&mut actions);
    actions
}

fn apply_actions(mut state: State, actions: &[Action]) -> Option<State> {
    for &action in actions {
        state = state.apply(action).ok()?;
    }
    Some(state)
}

fn terminal_score(state: &State, distance: u32) -> Option<f64> {
    state.winner().map(|winner| {
        let score = MATE - distance as f64;
        if winner == Player::Alpha {
            score
        } else {
            -score
        }
    })
}

fn terminal_evaluation(state: &State, distance: u32) -> Option<Evaluation> {
    state.winner().map(|winner| mate_in(winner, distance))
}

fn score_to_evaluation(score: f64) -> Evaluation {
    if score.abs() >= MATE - 128.0 {
        mate_in(
            if score > 0.0 {
                Player::Alpha
            } else {
                Player::Beta
            },
            (MATE - score.abs()).max(0.0).round() as u32,
        )
    } else {
        estimate(score)
    }
}

fn mate_in(winner: Player, plies: u32) -> Evaluation {
    MateInN::new(winner, plies)
        .expect("search depth is within the supported mate distance")
        .into()
}

fn estimate(value: f64) -> Evaluation {
    assert!(value.is_finite(), "evaluation must be finite");
    let millipoints = (value * 1_000.0).round().clamp(
        f64::from(EvaluationEstimate::MIN.millipoints()),
        f64::from(EvaluationEstimate::MAX.millipoints()),
    ) as i32;
    EvaluationEstimate::from_millipoints(millipoints)
        .expect("clamped evaluation is in range")
        .into()
}

fn animals() -> [Animal; 16] {
    [
        Animal::Mouse,
        Animal::Ox,
        Animal::Tiger,
        Animal::Rabbit,
        Animal::Dragon,
        Animal::Snake,
        Animal::Horse,
        Animal::Ram,
        Animal::Monkey,
        Animal::Rooster,
        Animal::Dog,
        Animal::Boar,
        Animal::Fish,
        Animal::Elephant,
        Animal::Squid,
        Animal::Frog,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_core::{
        AnimalDrop, AnimalStep, CardMultiset, InitialStateBuilder, Rank, SnipeStep, StepDirection,
    };
    use snipe_prng::initial_state;

    fn cards(cards: &[(Card, Player)]) -> CardMultiset {
        cards
            .iter()
            .fold(CardMultiset::EMPTY, |result, &(card, player)| {
                result
                    .checked_add(CardMultiset::singleton(card, player))
                    .unwrap()
            })
    }

    fn state() -> State {
        let deck = animals();
        InitialStateBuilder {
            alpha_reserve: [deck[0]],
            r1: [deck[1], deck[2]],
            r2: [
                deck[3], deck[4], deck[5], deck[6], deck[7], deck[8], deck[9], deck[10], deck[11],
                deck[12], deck[13], deck[14],
            ],
            r3: [deck[15]],
            r4: [deck[15]],
            r5: [
                deck[3], deck[4], deck[5], deck[6], deck[7], deck[8], deck[9], deck[10], deck[11],
                deck[12], deck[13], deck[14],
            ],
            r6: [deck[1], deck[2]],
            beta_reserve: [deck[0]],
        }
        .build()
        .unwrap()
    }

    fn animal(name: &str) -> Animal {
        match name {
            "Rat" => Animal::Mouse,
            "Ox" => Animal::Ox,
            "Tiger" => Animal::Tiger,
            "Rabbit" => Animal::Rabbit,
            "Dragon" => Animal::Dragon,
            "Snake" => Animal::Snake,
            "Horse" => Animal::Horse,
            "Ram" => Animal::Ram,
            "Monkey" => Animal::Monkey,
            "Rooster" => Animal::Rooster,
            "Dog" => Animal::Dog,
            "Boar" => Animal::Boar,
            "Fish" => Animal::Fish,
            "Elephant" => Animal::Elephant,
            "Squid" => Animal::Squid,
            "Frog" => Animal::Frog,
            _ => panic!("unknown animal {name}"),
        }
    }

    fn rank(notation: &str) -> Rank {
        match notation
            .bytes()
            .find(|byte| byte.is_ascii_digit())
            .expect("move notation has a rank")
        {
            b'1' => Rank::R1,
            b'2' => Rank::R2,
            b'3' => Rank::R3,
            b'4' => Rank::R4,
            b'5' => Rank::R5,
            b'6' => Rank::R6,
            value => panic!("invalid rank {}", char::from(value)),
        }
    }

    fn action(notation: &str) -> Action {
        let (actor, movement) = notation
            .split_once(' ')
            .expect("move notation has an actor and movement");
        let destination = rank(movement);
        if actor == "Alpha" || actor == "Beta" {
            Action::SnipeStep(SnipeStep { destination })
        } else if movement.starts_with('&') {
            Action::Drop(AnimalDrop {
                actor: animal(actor),
                destination,
            })
        } else {
            Action::AnimalStep(AnimalStep {
                actor: animal(actor),
                direction: if movement.starts_with('*') {
                    StepDirection::Retreat
                } else {
                    StepDirection::Advance
                },
                destination,
            })
        }
    }

    fn screenshot_position() -> State {
        const HISTORY: &str = "\
1b. Ox 4, Fish 4x
2a. Dragon 3, Squid *1
3b. Dog 4, Boar 4
4a. Ram 3, Boar 3
5b. Tiger 4x, Horse 5
6a. Ox 3, Tiger 3
7b. Elephant &6
8a. Rabbit *1, Squid 2
9b. Ox &4
10a. Ram *1, Boar *2
11b. Ox 3, Rooster 4
12a. Fish 3, Squid 3x
13b. Tiger 3, Rat 4
14a. Snake 3, Squid 4
15b. Dog &2
16a. Tiger &2
17b. Tiger 2x, Rat 3
18a. Rooster 2, Squid 5
19b. Boar &5
20a. Rabbit 2, Rooster 3
21b. Boar &3
22a. Dragon &2
23b. Rat 2, Boar 2x
24a. Fish &6
25b. Rat 4, Elephant 5x
26a. Dragon &2
27b. Rooster 3, Boar 1
28a. Monkey 2, Snake 4
29b. Dragon &1
30a. Rooster 4, Snake 5
31b. Horse &2";
        let mut state = initial_state(0);
        for line in HISTORY.lines() {
            let (_, moves) = line.split_once(". ").expect("history line has a prefix");
            for notation in moves.split(", ") {
                state = state
                    .apply(action(notation))
                    .unwrap_or_else(|error| panic!("{line}: {notation} is illegal: {error:?}"));
            }
        }
        state
    }

    #[test]
    fn always_writes_a_legal_complete_ply() {
        let state = state();
        let mut analyzer = AvocadoAnalyzer::new();
        analyzer.set_state(state.clone());
        analyzer.think(4);
        let mut line = Vec::new();
        analyzer.write_optimal_lop(&mut line);
        assert!(!line.is_empty());
        let child = apply_actions(state, &line).unwrap();
        assert!(child.active_player == Player::Alpha || child.winner().is_some());
    }

    #[test]
    fn writes_the_searched_multi_ply_principal_variation() {
        let state = state();
        let root_player = state.active_player;
        let candidate_count = full_plies(&state).len();
        let mut analyzer = AvocadoAnalyzer::new();
        analyzer.set_state(state.clone());

        // Finish the one-ply pass, then search every root candidate deeply
        // enough to choose and retain the opponent's reply.
        analyzer.think(candidate_count * 2);

        let mut line = Vec::new();
        analyzer.write_optimal_lop(&mut line);
        let mut replay = state;
        let mut completed_plies = 0;
        let mut active_player = root_player;
        for action in line {
            replay = replay.apply(action).unwrap();
            if replay.winner().is_some() || replay.active_player != active_player {
                completed_plies += 1;
                active_player = replay.active_player;
            }
        }

        assert!(
            completed_plies >= 2 || replay.winner().is_some(),
            "the line should include a searched reply, not only the root ply"
        );
    }

    #[test]
    fn mate_in_three_score_includes_the_three_ply_mating_line() {
        let state = screenshot_position();
        let candidate_count = full_plies(&state).len();
        let mut analyzer = AvocadoAnalyzer::new();
        analyzer.set_state(state.clone());

        analyzer.think(candidate_count * 20);

        let mate = match analyzer.evaluation() {
            Evaluation::MateInN(mate) => mate,
            Evaluation::Estimate(estimate) => {
                panic!("expected mate-in-3, got estimate {estimate:?}")
            }
        };
        assert_eq!(mate.winner(), Player::Alpha);
        assert_eq!(mate.plies(), 3);

        let mut line = Vec::new();
        analyzer.write_optimal_lop(&mut line);
        let mut replay = state;
        let mut completed_plies = 0;
        let mut active_player = replay.active_player;
        for action in line {
            replay = replay.apply(action).unwrap();
            if replay.winner().is_some() || replay.active_player != active_player {
                completed_plies += 1;
                active_player = replay.active_player;
            }
        }

        assert_eq!(completed_plies, mate.plies());
        assert_eq!(replay.winner(), Some(mate.winner()));
    }

    #[test]
    fn keeps_a_terminal_child_one_ply_away_after_thinking() {
        let state = State {
            active_player: Player::Alpha,
            reserves: CardMultiset::EMPTY,
            r1: cards(&[
                (Card::Snipe, Player::Alpha),
                (Card::Animal(Animal::Mouse), Player::Alpha),
            ]),
            r2: cards(&[
                (Card::Animal(Animal::Rooster), Player::Beta),
                (Card::Animal(Animal::Tiger), Player::Beta),
                (Card::Snipe, Player::Beta),
            ]),
            r3: CardMultiset::EMPTY,
            r4: CardMultiset::EMPTY,
            r5: CardMultiset::EMPTY,
            r6: CardMultiset::EMPTY,
            leading_action: None,
        };
        let expected = MateInN::new(Player::Alpha, 1).unwrap().into();
        let mut analyzer = AvocadoAnalyzer::new();

        analyzer.set_state(state);
        assert_eq!(analyzer.evaluation(), expected);
        analyzer.think(32);
        assert_eq!(analyzer.evaluation(), expected);
    }

    #[test]
    fn public_estimates_are_rounded_and_bounded_millipoints() {
        assert_eq!(
            estimate(1.2346),
            EvaluationEstimate::from_millipoints(1_235).unwrap().into()
        );
        assert_eq!(estimate(1_000.0), EvaluationEstimate::MAX.into());
        assert_eq!(estimate(-1_000.0), EvaluationEstimate::MIN.into());
    }
}
