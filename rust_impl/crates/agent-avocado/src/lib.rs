//! Avocado: a deterministic, patient Snipe Hunt calculator.
//!
//! The search and evaluation in this crate are intentionally private and
//! independent. Only `snipe-core` types cross its public boundary.

use snipe_core::{
    Action, ActionWriter, Analyzer, Animal, Card, Evaluation, EvaluationEstimate, Player, State,
};
use std::collections::HashMap;

const MATE: f64 = 1_000_000.0;
const MAX_DEPTH: usize = 4;

#[derive(Clone)]
struct Candidate {
    actions: Vec<Action>,
    state: State,
    score: f64,
}

/// A deterministic, positionally conservative analyzer.
pub struct AvocadoAnalyzer {
    root: Option<State>,
    candidates: Vec<Candidate>,
    best: usize,
    cursor: usize,
    depth: usize,
    evaluation: Evaluation,
    transpositions: HashMap<(u64, usize, usize), f64>,
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
        self.candidates[index].score = minimax(
            &state,
            self.depth.saturating_sub(1),
            1,
            -MATE,
            MATE,
            &mut visited,
            384,
            &mut self.transpositions,
        );
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
            writer.reserve(candidate.actions.len());
            for &action in &candidate.actions {
                writer.push(action);
            }
        }
    }
}

fn minimax(
    state: &State,
    depth: usize,
    plies_from_root: usize,
    mut alpha: f64,
    mut beta: f64,
    visited: &mut usize,
    budget: usize,
    transpositions: &mut HashMap<(u64, usize, usize), f64>,
) -> f64 {
    *visited += 1;
    if let Some(score) = terminal_score(state, plies_from_root) {
        return score;
    }
    if depth == 0 || *visited >= budget {
        return evaluate(state);
    }
    let key = (fingerprint(state), depth, plies_from_root);
    if let Some(&cached) = transpositions.get(&key) {
        return cached;
    }
    let moves = full_plies(state);
    if moves.is_empty() {
        return evaluate(state);
    }
    if state.active_player == Player::Alpha {
        let mut value = -MATE;
        for actions in moves {
            let Some(child) = apply_actions(state.clone(), &actions) else {
                continue;
            };
            value = value.max(minimax(
                &child,
                depth - 1,
                plies_from_root + 1,
                alpha,
                beta,
                visited,
                budget,
                transpositions,
            ));
            alpha = alpha.max(value);
            if alpha >= beta || *visited >= budget {
                break;
            }
        }
        transpositions.insert(key, value);
        value
    } else {
        let mut value = MATE;
        for actions in moves {
            let Some(child) = apply_actions(state.clone(), &actions) else {
                continue;
            };
            value = value.min(minimax(
                &child,
                depth - 1,
                plies_from_root + 1,
                alpha,
                beta,
                visited,
                budget,
                transpositions,
            ));
            beta = beta.min(value);
            if alpha >= beta || *visited >= budget {
                break;
            }
        }
        transpositions.insert(key, value);
        value
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

fn terminal_score(state: &State, distance: usize) -> Option<f64> {
    state.winner().map(|winner| {
        let score = MATE - distance as f64;
        if winner == Player::Alpha {
            score
        } else {
            -score
        }
    })
}

fn terminal_evaluation(state: &State, distance: usize) -> Option<Evaluation> {
    state.winner().map(|winner| Evaluation::MateInN {
        winner,
        plies: distance,
    })
}

fn score_to_evaluation(score: f64) -> Evaluation {
    if score.abs() >= MATE - 128.0 {
        Evaluation::MateInN {
            winner: if score > 0.0 {
                Player::Alpha
            } else {
                Player::Beta
            },
            plies: (MATE - score.abs()).max(0.0) as usize,
        }
    } else {
        estimate(score)
    }
}

fn estimate(value: f64) -> Evaluation {
    Evaluation::Estimate(EvaluationEstimate::new(value).expect("finite evaluation"))
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
    use snipe_core::{CardMultiset, InitialStateBuilder};

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
        let expected = Evaluation::MateInN {
            winner: Player::Alpha,
            plies: 1,
        };
        let mut analyzer = AvocadoAnalyzer::new();

        analyzer.set_state(state);
        assert_eq!(analyzer.evaluation(), expected);
        analyzer.think(32);
        assert_eq!(analyzer.evaluation(), expected);
    }
}
