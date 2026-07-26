//! Blueberry: an aggressive, policy-guided Monte Carlo Snipe Hunt player.
//!
//! This crate deliberately shares no representation, search, or evaluation
//! code with any other agent.

use snipe_core::{
    Action, ActionWriter, Analyzer, Animal, Card, Evaluation, EvaluationEstimate, Player, State,
};

#[derive(Clone)]
struct Arm {
    actions: Vec<Action>,
    state: State,
    visits: u32,
    value: f64,
    prior: f64,
}

/// A forcing, opportunistic Monte Carlo analyzer.
pub struct BlueberryAnalyzer {
    root: Option<State>,
    arms: Vec<Arm>,
    best: usize,
    rng: u64,
    evaluation: Evaluation,
}

impl Default for BlueberryAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl BlueberryAnalyzer {
    pub fn new() -> Self {
        Self {
            root: None,
            arms: Vec::new(),
            best: 0,
            rng: 0xB10E_BEEF_51A7_E5E5,
            evaluation: estimate(0.0),
        }
    }

    fn random(&mut self, upper: usize) -> usize {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        (self.rng as usize) % upper.max(1)
    }

    fn select_arm(&self) -> usize {
        let total = self.arms.iter().map(|arm| arm.visits).sum::<u32>().max(1);
        let maximizing = self
            .root
            .as_ref()
            .is_none_or(|state| state.active_player == Player::Alpha);
        (0..self.arms.len())
            .max_by(|&left, &right| {
                let utility = |arm: &Arm| {
                    let mean = if arm.visits == 0 {
                        0.0
                    } else {
                        arm.value / f64::from(arm.visits)
                    };
                    let signed = if maximizing { mean } else { -mean };
                    signed + arm.prior * (f64::from(total).sqrt() / f64::from(arm.visits + 1))
                };
                utility(&self.arms[left]).total_cmp(&utility(&self.arms[right]))
            })
            .unwrap_or(0)
    }

    fn refresh(&mut self) {
        let Some(root) = &self.root else { return };
        if self.arms.is_empty() {
            self.evaluation = root.winner().map_or_else(
                || estimate(instinct(root)),
                |winner| Evaluation::MateInN { winner, plies: 0 },
            );
            return;
        }
        let maximizing = root.active_player == Player::Alpha;
        self.best = (0..self.arms.len())
            .max_by(|&left, &right| {
                let strength = |arm: &Arm| {
                    let mean = if arm.visits == 0 {
                        instinct(&arm.state)
                    } else {
                        arm.value / f64::from(arm.visits)
                    };
                    if maximizing { mean } else { -mean }
                };
                strength(&self.arms[left])
                    .total_cmp(&strength(&self.arms[right]))
                    .then_with(|| self.arms[left].visits.cmp(&self.arms[right].visits))
            })
            .unwrap_or(0);
        let arm = &self.arms[self.best];
        let value = if arm.visits == 0 {
            instinct(&arm.state)
        } else {
            arm.value / f64::from(arm.visits)
        };
        self.evaluation = arm.state.winner().map_or_else(
            || estimate(value),
            |winner| Evaluation::MateInN {
                winner,
                plies: arm.actions.len(),
            },
        );
    }
}

impl Analyzer for BlueberryAnalyzer {
    fn set_state(&mut self, state: State) {
        self.rng = fingerprint(&state) ^ 0xB10E_BEEF_51A7_E5E5;
        self.arms = turns(&state)
            .into_iter()
            .filter_map(|actions| {
                let child = execute(state.clone(), &actions)?;
                let prior = forcing_prior(&state, &child, &actions);
                Some(Arm {
                    actions,
                    state: child,
                    visits: 0,
                    value: 0.0,
                    prior,
                })
            })
            .collect();
        self.root = Some(state);
        self.best = 0;
        self.evaluation = estimate(0.0);
        self.refresh();
    }

    fn think_for_one_tick(&mut self) {
        if self.arms.is_empty() {
            return;
        }
        let index = self.select_arm();
        let mut state = self.arms[index].state.clone();
        let mut horizon = 0;
        while state.winner().is_none() && horizon < 5 {
            let moves = turns(&state);
            if moves.is_empty() {
                break;
            }
            let sample = if horizon < 2 {
                moves
                    .iter()
                    .enumerate()
                    .max_by(|(_, left), (_, right)| {
                        let left_state = execute(state.clone(), left).expect("generated move");
                        let right_state = execute(state.clone(), right).expect("generated move");
                        forcing_prior(&state, &left_state, left).total_cmp(&forcing_prior(
                            &state,
                            &right_state,
                            right,
                        ))
                    })
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            } else {
                self.random(moves.len())
            };
            state = execute(state, &moves[sample]).expect("generated move is legal");
            horizon += 1;
        }
        let outcome = match state.winner() {
            Some(Player::Alpha) => 100_000.0 - horizon as f64,
            Some(Player::Beta) => -100_000.0 + horizon as f64,
            None => instinct(&state),
        };
        self.arms[index].visits += 1;
        self.arms[index].value += outcome;
        self.refresh();
    }

    fn evaluation(&self) -> Evaluation {
        self.evaluation
    }

    fn write_optimal_lop<W: ActionWriter>(&self, writer: &mut W) {
        if let Some(arm) = self.arms.get(self.best) {
            writer.reserve(arm.actions.len());
            for &action in &arm.actions {
                writer.push(action);
            }
        }
    }
}

fn turns(state: &State) -> Vec<Vec<Action>> {
    let player = state.active_player;
    let mut choices = Vec::new();
    let mut first_actions = Vec::new();
    state.write_legal_actions(&mut first_actions);
    for first in first_actions {
        let Ok(after) = state.clone().apply(first) else {
            continue;
        };
        if after.active_player != player || after.winner().is_some() {
            choices.push(vec![first]);
            continue;
        }
        let mut second_actions = Vec::new();
        after.write_legal_actions(&mut second_actions);
        for second in second_actions {
            if after.clone().apply(second).is_ok() {
                choices.push(vec![first, second]);
            }
        }
    }
    choices
}

fn execute(mut state: State, actions: &[Action]) -> Option<State> {
    for &action in actions {
        state = state.apply(action).ok()?;
    }
    Some(state)
}

fn forcing_prior(before: &State, after: &State, actions: &[Action]) -> f64 {
    if after.winner() == Some(before.active_player) {
        return 1000.0;
    }
    let enemy = before.active_player.opponent();
    let reserve_gain = reserve_animals(after, before.active_player) as f64
        - reserve_animals(before, before.active_player) as f64;
    let enemy_snipe_pressure = snipe_progress(after, enemy);
    1.0 + reserve_gain.max(0.0) * 8.0 + enemy_snipe_pressure * 2.5 + actions.len() as f64 * 0.2
}

fn instinct(state: &State) -> f64 {
    if let Some(winner) = state.winner() {
        return if winner == Player::Alpha {
            100_000.0
        } else {
            -100_000.0
        };
    }
    let alpha_hunt = snipe_progress(state, Player::Beta) * 12.0;
    let beta_hunt = snipe_progress(state, Player::Alpha) * 12.0;
    let captures = (reserve_animals(state, Player::Alpha) as f64
        - reserve_animals(state, Player::Beta) as f64)
        * 5.0;
    alpha_hunt - beta_hunt + captures
}

fn reserve_animals(state: &State, player: Player) -> u32 {
    animals()
        .into_iter()
        .map(|animal| u32::from(state.reserves.count(Card::Animal(animal), player)))
        .sum()
}

fn snipe_progress(state: &State, prey: Player) -> f64 {
    let ranks = [
        &state.r1, &state.r2, &state.r3, &state.r4, &state.r5, &state.r6,
    ];
    for (index, cards) in ranks.into_iter().enumerate() {
        if cards.count(Card::Snipe, prey) != 0 {
            return match prey {
                Player::Alpha => (5 - index) as f64,
                Player::Beta => index as f64,
            };
        }
    }
    8.0
}

fn fingerprint(state: &State) -> u64 {
    let ranks = [
        &state.reserves,
        &state.r1,
        &state.r2,
        &state.r3,
        &state.r4,
        &state.r5,
        &state.r6,
    ];
    let mut hash = if state.active_player == Player::Alpha {
        17
    } else {
        31
    };
    for (location, cards) in ranks.into_iter().enumerate() {
        for (animal_index, animal) in animals().into_iter().enumerate() {
            let count = u64::from(cards.count(Card::Animal(animal), Player::Alpha))
                + 3 * u64::from(cards.count(Card::Animal(animal), Player::Beta));
            hash ^= count.wrapping_add((location * 37 + animal_index) as u64);
            hash = hash.rotate_left(9).wrapping_mul(0x9E37_79B9_7F4A_7C15);
        }
    }
    hash
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
    use snipe_core::InitialStateBuilder;

    #[test]
    fn recommendation_is_legal_before_and_after_thinking() {
        let a = animals();
        let state = InitialStateBuilder {
            alpha_reserve: [a[0]],
            r1: [a[1], a[2]],
            r2: [
                a[3], a[4], a[5], a[6], a[7], a[8], a[9], a[10], a[11], a[12], a[13], a[14],
            ],
            r3: [a[15]],
            r4: [a[15]],
            r5: [
                a[3], a[4], a[5], a[6], a[7], a[8], a[9], a[10], a[11], a[12], a[13], a[14],
            ],
            r6: [a[1], a[2]],
            beta_reserve: [a[0]],
        }
        .build()
        .unwrap();
        let mut analyzer = BlueberryAnalyzer::new();
        analyzer.set_state(state.clone());
        analyzer.think(8);
        let mut line = Vec::new();
        analyzer.write_optimal_lop(&mut line);
        assert!(execute(state, &line).is_some());
    }
}
