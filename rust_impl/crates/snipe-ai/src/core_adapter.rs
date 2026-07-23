//! Concrete integration with the authoritative Snipe Hunt rules.

use crate::{GamePosition, SnipeFeatures, SnipeWeights, MATE_SCORE};
use snipe_core::{activates_triplet, Animal, Location, Move, Player, Row, State};

impl GamePosition for State {
    type Move = Move;

    #[inline]
    fn legal_moves(&self, moves: &mut Vec<Self::Move>) {
        State::legal_moves_into(*self, moves);
    }

    #[inline]
    fn apply_move(&self, mv: Self::Move) -> Self {
        State::apply_move(*self, mv).expect("the rules engine generated an illegal move")
    }

    #[inline]
    fn position_hash(&self) -> u64 {
        State::position_hash(*self)
    }

    #[inline]
    fn repetition_hash(&self) -> u64 {
        State::repetition_hash(*self)
    }

    #[inline]
    fn convergence_hash(&self) -> u64 {
        State::convergence_hash(*self)
    }

    fn terminal_score(&self) -> Option<i32> {
        State::winner(*self).map(|winner| {
            if winner == self.side_to_move() {
                MATE_SCORE
            } else {
                -MATE_SCORE
            }
        })
    }

    #[inline]
    fn evaluate(&self) -> i32 {
        evaluate_state(*self)
    }

    #[inline]
    fn move_ordering_score(&self, mv: Self::Move, child: &Self) -> i32 {
        tactical_move_score(*self, mv, *child)
    }

    fn is_tactical(&self, _mv: Self::Move, child: &Self) -> bool {
        if child.captured_snipe_winner().is_some() {
            return true;
        }
        let player = self.side_to_move();
        let reserve = Location::reserve_of(player);
        let before = self.animal_bits(reserve, player);
        let after = child.animal_bits(reserve, player);
        after & !before != 0
    }

    fn creates_direct_snipe_threat(&self, _mv: Self::Move, child: &Self) -> bool {
        let player = self.side_to_move();
        snipe_threats(*child, player) > snipe_threats(*self, player)
    }

    fn is_snipe_step(&self, mv: Self::Move) -> bool {
        matches!(mv, Move::Snipe { .. })
    }

    fn has_immediate_snipe_capture_threat(&self) -> bool {
        let defender = self.side_to_move();
        let attacker = defender.opponent();
        let mut data = self.to_data();
        data.side_to_move = attacker as u8;
        data.pending_animal = u8::MAX;
        data.pending_destination = 0;
        let attacker_to_move =
            State::from_data(data).expect("changing only side-to-move preserves state validity");
        attacker_to_move.has_winning_snipe_capture()
    }

    fn side_to_move_has_snipe_capture(&self) -> bool {
        self.has_winning_snipe_capture()
    }
}

pub fn evaluate_state(state: State) -> i32 {
    SnipeWeights::default().evaluate(extract_features(state))
}

pub fn extract_features(state: State) -> SnipeFeatures {
    let me = state.side_to_move();
    let them = me.opponent();
    let my_opportunities = triplet_opportunities(state, me);
    let their_opportunities = triplet_opportunities(state, them);
    SnipeFeatures {
        material: owned_animals(state, me) - owned_animals(state, them),
        reserve: state.reserve_count(me) as i32 - state.reserve_count(them) as i32,
        mobility: pseudo_mobility(state, me) - pseudo_mobility(state, them),
        progress: progress(state, me) - progress(state, them),
        retreaters: retreaters(state, me) - retreaters(state, them),
        near_triplets: my_opportunities.0 - their_opportunities.0,
        capture_pressure: my_opportunities.1 - their_opportunities.1,
        snipe_pressure: snipe_threats(state, me) - snipe_threats(state, them),
        snipe_liberties: snipe_liberties(state, me) - snipe_liberties(state, them),
        row_freedom: row_freedom(state, me) - row_freedom(state, them),
    }
}

/// Fast ordering score. Capturing the snipe is overwhelmingly first, followed
/// by triplet captures, newly-created snipe threats, and forward development.
pub fn tactical_move_score(parent: State, mv: Move, child: State) -> i32 {
    let player = parent.side_to_move();
    if child.captured_snipe_winner() == Some(player) {
        return 390_000;
    }

    let reserve = Location::reserve_of(player);
    let captured = (child.animal_bits(reserve, player) & !parent.animal_bits(reserve, player))
        .count_ones() as i32;
    let positional = match mv {
        Move::Snipe { destination } => {
            // Prefer central escape squares until tactics say otherwise.
            6 - (destination.number() as i32 * 2 - 7).abs()
        }
        Move::Drop { destination, .. } => forward_value(destination, player) * 3,
        Move::Animals { first, second } => {
            forward_value(first.destination, player) * 4
                + second
                    .map(|step| forward_value(step.destination, player) * 2)
                    .unwrap_or(0)
        }
    };
    // The child is evaluated for the opponent. This breaks the many quiet-move
    // ties before selective search discards low-promise alternatives.
    captured * 12_000 + positional - evaluate_state(child) / 4
}

fn owned_animals(state: State, player: Player) -> i32 {
    Location::ALL
        .into_iter()
        .map(|location| state.animal_count(location, player) as i32)
        .sum()
}

fn progress(state: State, player: Player) -> i32 {
    Row::ALL
        .into_iter()
        .map(|row| state.animal_count(row.location(), player) as i32 * forward_value(row, player))
        .sum()
}

fn forward_value(row: Row, player: Player) -> i32 {
    match player {
        Player::Alpha => row.number() as i32,
        Player::Beta => 7 - row.number() as i32,
    }
}

fn retreaters(state: State, player: Player) -> i32 {
    Animal::ALL
        .into_iter()
        .filter(|&animal| animal.can_retreat() && state.owner_of_animal(animal) == Some(player))
        .count() as i32
}

fn pseudo_mobility(state: State, player: Player) -> i32 {
    let mut count = snipe_liberties(state, player);
    let reserve = state.reserve_count(player) as i32;
    if reserve > 1 {
        for animal in Animal::ALL {
            if state.owner_of_animal(animal) == Some(player)
                && state.location_of_animal(animal) == Some(Location::reserve_of(player))
            {
                count += if animal.can_retreat() { 4 } else { 6 };
            }
        }
    }
    for animal in Animal::ALL {
        if state.owner_of_animal(animal) != Some(player) {
            continue;
        }
        let Some(row) = state.location_of_animal(animal).and_then(Location::row) else {
            continue;
        };
        let source_has_support = state.cell(row.location()).card_count() > 1;
        if source_has_support && row.forward(player).is_some() {
            count += 1;
        }
        if source_has_support && animal.can_retreat() && row.backward(player).is_some() {
            count += 1;
        }
    }
    count
}

fn row_freedom(state: State, player: Player) -> i32 {
    Row::ALL
        .into_iter()
        .map(|row| {
            let cell = state.cell(row.location());
            if cell.animals(player) != 0 && cell.card_count() > 1 {
                cell.animals(player).count_ones() as i32
            } else {
                0
            }
        })
        .sum()
}

fn snipe_liberties(state: State, player: Player) -> i32 {
    let Some(row) = state.snipe_location(player).and_then(Location::row) else {
        return 0;
    };
    let cell = state.cell(row.location());
    if cell.all_animals() == 0 && !cell.has_snipe(player.opponent()) {
        return 0;
    }
    i32::from(row.forward(player).is_some()) + i32::from(row.backward(player).is_some())
}

/// Returns `(activating_steps, capturable_cards)`.
fn triplet_opportunities(state: State, player: Player) -> (i32, i32) {
    let mut activations = 0;
    let mut capturable = 0;
    for animal in Animal::ALL {
        if state.owner_of_animal(animal) != Some(player) {
            continue;
        }
        let Some(source) = state.location_of_animal(animal).and_then(Location::row) else {
            continue;
        };
        let source_cell = state.cell(source.location());
        for destination in [
            source.forward(player),
            if animal.can_retreat() {
                source.backward(player)
            } else {
                None
            },
        ]
        .into_iter()
        .flatten()
        {
            let destination_cell = state.cell(destination.location());
            if activates_triplet(destination_cell.all_animals(), animal)
                && (source_cell.card_count() > 1 || destination_cell.has_snipe(player.opponent()))
            {
                activations += 1;
                capturable += destination_cell.card_count() as i32;
            }
        }
    }
    (activations, capturable)
}

fn snipe_threats(state: State, attacker: Player) -> i32 {
    let Some(target) = state
        .snipe_location(attacker.opponent())
        .and_then(Location::row)
    else {
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
                    || target_cell.has_snipe(attacker.opponent()))
        })
        .count() as i32
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{SearchConfig, SearchEngine, SearchPolicy};
    use std::time::Duration;

    #[test]
    fn reflected_position_has_equal_evaluation_for_reflected_side_to_move() {
        for seed in 0..16 {
            let state = State::initial(seed);
            assert_eq!(evaluate_state(state), evaluate_state(state.reflected()));
        }
    }

    #[test]
    fn every_generated_move_can_be_scored_and_applied() {
        for seed in 0..8 {
            let state = State::initial(seed);
            for mv in state.legal_moves() {
                let child = state.apply_move(mv).unwrap();
                let _ = tactical_move_score(state, mv, child);
                let _ = extract_features(child);
            }
        }
    }

    #[test]
    fn completed_aspiration_move_survives_a_timed_out_research() {
        let config = SearchConfig {
            time_limit: Duration::from_secs(60),
            max_depth: 64,
            deadline_check_interval: 1,
            ..SearchConfig::default()
        };
        let mut baseline = SearchEngine::new_with_policy(
            config.clone(),
            SearchPolicy {
                retain_completed_aspiration_on_timeout: false,
                node_limit: Some(20_000),
                ..SearchPolicy::production()
            },
        );
        let state = State::initial(1);
        let result = baseline.search(&state);
        assert_eq!(
            result.principal_variation.first(),
            result.best_move.as_ref()
        );

        let mut candidate = SearchEngine::new_with_policy(
            config,
            SearchPolicy {
                retain_completed_aspiration_on_timeout: true,
                node_limit: Some(20_000),
                ..SearchPolicy::production()
            },
        );
        let candidate_result = candidate.search(&state);
        let aspiration_child = state
            .apply_move(Move::Animals {
                first: snipe_core::AnimalStep {
                    moved: Animal::Dragon1,
                    destination: Row::Four,
                },
                second: Some(snipe_core::AnimalStep {
                    moved: Animal::Horse1,
                    destination: Row::Four,
                }),
            })
            .unwrap();
        let candidate_child = state
            .apply_move(candidate_result.best_move.unwrap())
            .unwrap();
        assert_eq!(
            candidate_child, aspiration_child,
            "baseline={result:?} candidate={candidate_result:?}"
        );
        assert!(candidate_result.depth > result.depth);
    }
}
