//! Iceberg is a tactic-aware Snipe Hunt agent.
//!
//! It reasons in complete plies and treats pressure, safe snipe exits, captures,
//! and pressure-building moves as first-class search facts. Its scout is
//! deliberately selective when looking for a mate, but its shortestness search
//! considers every legal attack and defense. Any reported mate has a concrete
//! proof; a fully solved result also proves that no shorter mate exists.

mod position;
mod search;
mod tactics;

use crate::{
    position::Position,
    search::{ExactSearch, PreferredTurn, ProofSearch},
    tactics::Tactics,
};
use snipe_core::{
    Action, ActionWriter, Analyzer, Evaluation, EvaluationEstimate, MateInN, OptimalOutcome,
    Player, State,
};
use std::{collections::HashMap, sync::Arc};

const FIRST_BOUND: u8 = 4;
const MAX_BOUND: u8 = 40;

struct ProvenMate {
    bound: u8,
    distance: u8,
    line: Arc<[Action]>,
    preferences: Arc<HashMap<Position, PreferredTurn>>,
}

struct MateLane {
    target: Player,
    lower: u8,
    upper: Option<ProvenMate>,
    search: ExactSearch,
    exact: bool,
}

impl MateLane {
    fn new(root: Position, target: Player, _tactics: &mut Tactics) -> Self {
        Self {
            target,
            lower: 0,
            upper: None,
            search: ExactSearch::new(root, target, FIRST_BOUND, None),
            exact: false,
        }
    }

    fn consume_resolution(&mut self, root: Position, _tactics: &mut Tactics) {
        if !self.search.is_resolved() {
            return;
        }
        if let Some((distance, line)) = self.search.proved_line() {
            if self
                .upper
                .as_ref()
                .is_none_or(|old| distance < old.distance)
            {
                let preferences = self.upper.as_ref().map_or_else(
                    || Arc::new(HashMap::new()),
                    |old| Arc::clone(&old.preferences),
                );
                self.upper = Some(ProvenMate {
                    bound: self.search.bound,
                    distance,
                    line,
                    preferences,
                });
            }
        } else {
            self.lower = self.lower.max(self.search.bound);
        }

        let Some(upper) = self.upper.as_ref() else {
            if self.search.bound == MAX_BOUND {
                return;
            }
            let bound = self.search.bound.saturating_mul(2).min(MAX_BOUND);
            self.search.restart(root, bound, None);
            return;
        };
        if upper.distance <= self.lower.saturating_add(1) {
            self.exact = true;
            return;
        }
        let bound = self.lower + (upper.distance - self.lower) / 2;
        self.search.restart(
            root,
            bound,
            self.upper
                .as_ref()
                .map(|upper| Arc::clone(&upper.preferences)),
        );
    }

    fn diagnostics(&self) -> String {
        format!(
            "{:?}:lo{} hi{:?} exact{} {}",
            self.target,
            self.lower,
            self.upper
                .as_ref()
                .map(|upper| (upper.distance, upper.bound)),
            self.exact,
            self.search.diagnostics(),
        )
    }

    fn retained_entries(&self) -> usize {
        self.search.retained_entries()
    }
}

pub struct IcebergAnalyzer {
    root_state: Option<State>,
    root: Option<Position>,
    tactics: Tactics,
    searches: Option<[MateLane; 2]>,
    scouts: Option<[ProofSearch; 2]>,
    shortest_probe: Option<ProofSearch>,
    next_lane: usize,
    proved: Option<(Player, u8, Arc<[Action]>)>,
    solved: Option<OptimalOutcome>,
    fallback: Vec<Action>,
}

impl IcebergAnalyzer {
    pub fn new() -> Self {
        Self {
            root_state: None,
            root: None,
            tactics: Tactics::default(),
            searches: None,
            scouts: None,
            shortest_probe: None,
            next_lane: 0,
            proved: None,
            solved: None,
            fallback: Vec::new(),
        }
    }

    pub fn diagnostics(&self) -> String {
        let searches = self
            .searches
            .as_ref()
            .map(|searches| {
                format!(
                    "{}; {}",
                    searches[0].diagnostics(),
                    searches[1].diagnostics()
                )
            })
            .unwrap_or_else(|| "terminal".to_owned());
        let scouts = self
            .scouts
            .as_ref()
            .map(|scouts| format!("{}; {}", scouts[0].diagnostics(), scouts[1].diagnostics()))
            .unwrap_or_else(|| "terminal".to_owned());
        let shortest = self
            .shortest_probe
            .as_ref()
            .map_or_else(|| "none".to_owned(), ProofSearch::diagnostics);
        format!(
            "search[{searches}] scouts[{scouts}] shortest[{shortest}] tactics={} retained={} proved={:?}",
            self.tactics.retained_entries(),
            self.retained_entries(),
            self.proved
                .as_ref()
                .map(|(winner, distance, _)| (winner, distance)),
        )
    }

    pub fn retained_entries(&self) -> usize {
        self.tactics.retained_entries()
            + self
                .searches
                .iter()
                .flatten()
                .map(MateLane::retained_entries)
                .sum::<usize>()
            + self
                .scouts
                .iter()
                .flatten()
                .map(ProofSearch::retained_entries)
                .sum::<usize>()
            + self
                .shortest_probe
                .as_ref()
                .map_or(0, ProofSearch::retained_entries)
    }

    fn notice_lane(&mut self, lane: usize) {
        let Some(root) = self.root else { return };
        self.searches.as_mut().unwrap()[lane].consume_resolution(root, &mut self.tactics);
        let lane_state = &self.searches.as_ref().unwrap()[lane];
        let Some(upper) = lane_state.upper.as_ref() else {
            return;
        };
        let replacement_probe = self.shortest_probe.as_ref().and_then(|probe| {
            (!lane_state.exact
                && probe.target == lane_state.target
                && probe.bound <= lane_state.lower)
                .then(|| {
                    let bound = lane_state.lower + (upper.distance - lane_state.lower) / 2;
                    (bound, Arc::clone(&upper.line))
                })
        });
        if self.validate_line(&upper.line, lane_state.target, u32::from(upper.distance)) {
            self.proved = Some((lane_state.target, upper.distance, Arc::clone(&upper.line)));
            if lane_state.exact {
                let mate = MateInN::new(lane_state.target, u32::from(upper.distance)).unwrap();
                self.solved = Some(OptimalOutcome::MateInN(mate));
                self.shortest_probe = None;
            }
        }
        if let Some((bound, line)) = replacement_probe {
            self.shortest_probe = Some(ProofSearch::new(
                root,
                lane_state.target,
                bound,
                &mut self.tactics,
                false,
                Some(&line),
            ));
        }
    }

    fn notice_scout(&mut self, lane: usize) {
        let Some((distance, line)) = self.scouts.as_ref().unwrap()[lane].proved_line() else {
            return;
        };
        let target = self.scouts.as_ref().unwrap()[lane].target;
        let preferences = self.scouts.as_ref().unwrap()[lane].proven_preferences();
        if !self.validate_line(&line, target, u32::from(distance)) {
            return;
        }
        let exact_lane = self
            .searches
            .as_mut()
            .unwrap()
            .iter_mut()
            .find(|candidate| candidate.target == target)
            .unwrap();
        if exact_lane
            .upper
            .as_ref()
            .is_none_or(|old| distance < old.distance)
        {
            exact_lane.upper = Some(ProvenMate {
                bound: self.scouts.as_ref().unwrap()[lane].bound,
                distance,
                line: Arc::clone(&line),
                preferences: Arc::clone(&preferences),
            });
            let bound = exact_lane.search.bound;
            exact_lane
                .search
                .restart(self.root.unwrap(), bound, Some(preferences));
        }
        let probe_bound = exact_lane.search.bound.min(distance.saturating_sub(1));
        let shortest_probe = ProofSearch::new(
            self.root.unwrap(),
            target,
            probe_bound,
            &mut self.tactics,
            false,
            Some(&line),
        );
        self.proved = Some((target, distance, line));
        self.shortest_probe = Some(shortest_probe);
        self.scouts = None;
        self.tactics = Tactics::default();
    }

    fn notice_shortest_probe(&mut self) {
        let Some(probe) = self.shortest_probe.as_ref() else {
            return;
        };
        if !probe.is_resolved() {
            return;
        }
        let target = probe.target;
        let bound = probe.bound;
        let result = probe.proved_line();
        let found = if let Some((distance, line)) = result {
            if !self.validate_line(&line, target, u32::from(distance)) {
                return;
            }
            let preferences = probe.proven_preferences();
            Some((distance, line, preferences))
        } else {
            None
        };

        let (next_bound, preferred_line, exact_distance) = {
            let exact_lane = self
                .searches
                .as_mut()
                .unwrap()
                .iter_mut()
                .find(|candidate| candidate.target == target)
                .unwrap();
            if let Some((distance, line, preferences)) = found.as_ref() {
                exact_lane.upper = Some(ProvenMate {
                    bound,
                    distance: *distance,
                    line: Arc::clone(line),
                    preferences: Arc::clone(preferences),
                });
            } else {
                exact_lane.lower = exact_lane.lower.max(bound);
            }
            let upper = exact_lane
                .upper
                .as_ref()
                .expect("a shortestness probe starts from a proved mate");
            if upper.distance <= exact_lane.lower.saturating_add(1) {
                exact_lane.exact = true;
                (None, Arc::clone(&upper.line), Some(upper.distance))
            } else {
                let next_bound = exact_lane.lower + (upper.distance - exact_lane.lower) / 2;
                if exact_lane.search.bound <= exact_lane.lower {
                    exact_lane.search.restart(
                        self.root.unwrap(),
                        next_bound,
                        Some(Arc::clone(&upper.preferences)),
                    );
                }
                (Some(next_bound), Arc::clone(&upper.line), None)
            }
        };

        if let Some((distance, line, _)) = found {
            self.proved = Some((target, distance, line));
        }
        if let Some(distance) = exact_distance {
            let mate = MateInN::new(target, u32::from(distance)).unwrap();
            self.solved = Some(OptimalOutcome::MateInN(mate));
            self.shortest_probe = None;
        } else {
            self.shortest_probe = Some(ProofSearch::new(
                self.root.unwrap(),
                target,
                next_bound.unwrap(),
                &mut self.tactics,
                false,
                Some(&preferred_line),
            ));
        }
    }

    fn validate_line(&self, line: &[Action], winner: Player, expected_plies: u32) -> bool {
        let Some(mut state) = self.root_state.clone() else {
            return false;
        };
        // Completing a ply that was already in progress at the root still
        // counts as one completed ply in the Analyzer contract.
        let mut plies = u32::from(state.leading_action.is_some());
        for &action in line {
            if state.leading_action.is_none() {
                plies += 1;
            }
            let Ok(next) = state.apply(action) else {
                return false;
            };
            state = next;
            if state.winner().is_some() {
                break;
            }
        }
        state.winner() == Some(winner) && plies == expected_plies
    }

    fn choose_fallback(&mut self, root: Position) -> Vec<Action> {
        let attacker = root.active;
        let turns = self.tactics.attacking_turns(root, attacker);
        let turn = turns
            .first()
            .copied()
            .or_else(|| self.tactics.turns(root).first().copied());
        let Some(turn) = turn else {
            return Vec::new();
        };
        let mut line = vec![turn.first];
        if let Some(second) = turn.second {
            line.push(second);
        }
        line
    }
}

impl Default for IcebergAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for IcebergAnalyzer {
    fn set_state(&mut self, state: State) {
        self.root_state = Some(state.clone());
        self.tactics = Tactics::default();
        self.next_lane = 0;
        self.proved = None;
        self.shortest_probe = None;
        if let Some(winner) = state.winner() {
            let mate = MateInN::new(winner, 0).unwrap();
            self.solved = Some(OptimalOutcome::MateInN(mate));
            self.root = None;
            self.searches = None;
            self.scouts = None;
            self.fallback.clear();
            return;
        }
        self.solved = None;
        let root = Position::from_core(&state);
        self.root = Some(root);
        self.fallback = self.choose_fallback(root);
        let checked = self.tactics.profile(root, root.active.opponent()).pressed;
        let first = if checked {
            root.active.opponent()
        } else {
            root.active
        };
        self.searches = Some([
            MateLane::new(root, first, &mut self.tactics),
            MateLane::new(root, first.opponent(), &mut self.tactics),
        ]);
        self.scouts = Some([
            ProofSearch::new(root, first, 20, &mut self.tactics, true, None),
            ProofSearch::new(root, first.opponent(), 20, &mut self.tactics, true, None),
        ]);
    }

    fn think_for_one_tick(&mut self) {
        if self.solved.is_some() {
            return;
        }
        if self.proved.is_none() {
            let phase = self.next_lane % 10;
            self.next_lane = self.next_lane.wrapping_add(1);
            let lane = usize::from(phase == 9);
            let Some(scouts) = self.scouts.as_mut() else {
                return;
            };
            scouts[lane].tick(&mut self.tactics);
            if scouts[lane].target == self.root.unwrap().active
                && let Some(turn) = scouts[lane].leading_turn()
            {
                self.fallback = turn_actions(turn);
            }
            self.notice_scout(lane);
            return;
        }
        if self.shortest_probe.is_some() && self.next_lane.is_multiple_of(4) {
            self.next_lane = self.next_lane.wrapping_add(1);
            self.shortest_probe
                .as_mut()
                .unwrap()
                .tick(&mut self.tactics);
            self.notice_shortest_probe();
            return;
        }
        self.next_lane = self.next_lane.wrapping_add(1);
        let lane = self
            .searches
            .as_ref()
            .and_then(|lanes| {
                lanes
                    .iter()
                    .position(|lane| lane.upper.is_some() && !lane.exact)
            })
            .unwrap_or(0);
        {
            let Some(searches) = self.searches.as_mut() else {
                return;
            };
            searches[lane].search.tick(&mut self.tactics);
        }
        self.notice_lane(lane);
    }

    fn is_fully_solved(&self) -> Option<OptimalOutcome> {
        self.solved
    }

    fn evaluation(&self) -> Evaluation {
        if let Some(outcome) = self.solved {
            return outcome.as_evaluation();
        }
        if let Some((winner, distance, _)) = self.proved.as_ref() {
            return MateInN::new(*winner, u32::from(*distance)).unwrap().into();
        }
        EvaluationEstimate::ZERO.into()
    }

    fn write_optimal_lop<W>(&self, writer: &mut W)
    where
        W: ActionWriter,
    {
        let line: &[Action] = self
            .proved
            .as_ref()
            .map_or(self.fallback.as_slice(), |(_, _, line)| line);
        writer.reserve(line.len());
        for &action in line {
            writer.push(action);
        }
    }
}

fn turn_actions(turn: position::Turn) -> Vec<Action> {
    let mut actions = vec![turn.first];
    actions.extend(turn.second);
    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_core::{Animal, Card, CardMultiset};
    use snipe_prng::initial_state;
    use std::collections::HashSet;

    #[test]
    fn packed_actions_match_core_through_seeded_play() {
        for seed in 0..20 {
            let mut state = initial_state(seed);
            for ply in 0..40 {
                if state.winner().is_some() {
                    break;
                }
                let position = Position::from_core(&state);
                let mut core_actions = Vec::new();
                state.write_legal_actions(&mut core_actions);
                let packed_actions = position.legal_actions();
                assert_eq!(
                    core_actions.iter().copied().collect::<HashSet<_>>(),
                    packed_actions.iter().copied().collect::<HashSet<_>>(),
                    "seed {seed}, action {ply}"
                );
                assert_eq!(
                    core_completed_positions(&state),
                    position
                        .turns()
                        .into_iter()
                        .map(|turn| turn.next)
                        .collect::<HashSet<_>>(),
                    "seed {seed}, completed ply {ply}",
                );
                let selected = core_actions[(seed as usize + ply) % core_actions.len()];
                state = state.apply(selected).unwrap();
            }
        }
    }

    #[test]
    fn terminal_positions_are_solved_without_thinking() {
        let mut state = initial_state(4);
        state.r6 = state
            .r6
            .remove_one(snipe_core::Card::Snipe, Player::Beta)
            .unwrap();
        state.reserves = state
            .reserves
            .checked_add(snipe_core::CardMultiset::singleton(
                snipe_core::Card::Snipe,
                Player::Beta,
            ))
            .unwrap();
        let winner = state.winner().unwrap();
        let mut iceberg = IcebergAnalyzer::new();
        iceberg.set_state(state);
        assert_eq!(
            iceberg.evaluation(),
            MateInN::new(winner, 0).unwrap().into()
        );
        assert!(iceberg.is_fully_solved().is_some());
    }

    #[test]
    fn direct_capture_is_reported_as_the_exact_shortest_mate() {
        let state = State {
            active_player: Player::Alpha,
            reserves: cards(&[
                (Card::Animal(Animal::Mouse), Player::Alpha),
                (Card::Animal(Animal::Ox), Player::Alpha),
                (Card::Animal(Animal::Dragon), Player::Alpha),
                (Card::Animal(Animal::Snake), Player::Alpha),
                (Card::Animal(Animal::Ram), Player::Alpha),
                (Card::Animal(Animal::Rooster), Player::Alpha),
                (Card::Animal(Animal::Frog), Player::Alpha),
                (Card::Animal(Animal::Frog), Player::Alpha),
                (Card::Animal(Animal::Tiger), Player::Beta),
                (Card::Animal(Animal::Dragon), Player::Beta),
                (Card::Animal(Animal::Horse), Player::Beta),
                (Card::Animal(Animal::Horse), Player::Beta),
                (Card::Animal(Animal::Rooster), Player::Beta),
                (Card::Animal(Animal::Dog), Player::Beta),
                (Card::Animal(Animal::Fish), Player::Beta),
                (Card::Animal(Animal::Squid), Player::Beta),
            ]),
            r1: cards(&[(Card::Snipe, Player::Alpha)]),
            r2: cards(&[
                (Card::Animal(Animal::Fish), Player::Alpha),
                (Card::Animal(Animal::Squid), Player::Beta),
            ]),
            r3: cards(&[
                (Card::Animal(Animal::Rabbit), Player::Alpha),
                (Card::Animal(Animal::Snake), Player::Alpha),
                (Card::Animal(Animal::Elephant), Player::Alpha),
            ]),
            r4: cards(&[
                (Card::Animal(Animal::Mouse), Player::Alpha),
                (Card::Animal(Animal::Dog), Player::Alpha),
            ]),
            r5: cards(&[
                (Card::Animal(Animal::Ox), Player::Alpha),
                (Card::Animal(Animal::Boar), Player::Beta),
            ]),
            r6: cards(&[
                (Card::Animal(Animal::Tiger), Player::Alpha),
                (Card::Animal(Animal::Rabbit), Player::Beta),
                (Card::Animal(Animal::Ram), Player::Beta),
                (Card::Animal(Animal::Monkey), Player::Alpha),
                (Card::Animal(Animal::Monkey), Player::Beta),
                (Card::Animal(Animal::Boar), Player::Beta),
                (Card::Animal(Animal::Elephant), Player::Beta),
                (Card::Snipe, Player::Beta),
            ]),
            leading_action: None,
        };
        let mut iceberg = IcebergAnalyzer::new();
        iceberg.set_state(state.clone());
        for _ in 0..8 {
            iceberg.think_for_one_tick();
            if iceberg.is_fully_solved().is_some() {
                break;
            }
        }
        let mate = MateInN::new(Player::Alpha, 1).unwrap();
        assert_eq!(
            iceberg.is_fully_solved(),
            Some(OptimalOutcome::MateInN(mate))
        );
        assert_eq!(iceberg.evaluation(), mate.into());
        let mut line = Vec::new();
        iceberg.write_optimal_lop(&mut line);
        let result = line
            .iter()
            .copied()
            .try_fold(state.clone(), |current, action| current.apply(action))
            .unwrap();
        assert_eq!(result.winner(), Some(Player::Alpha));

        let partial_state = state.apply(line[0]).unwrap();
        assert!(partial_state.leading_action.is_some());
        let mut partial_iceberg = IcebergAnalyzer::new();
        partial_iceberg.set_state(partial_state.clone());
        partial_iceberg.think_for_one_tick();
        assert_eq!(partial_iceberg.evaluation(), mate.into());
        let mut partial_line = Vec::new();
        partial_iceberg.write_optimal_lop(&mut partial_line);
        let partial_result = partial_line
            .into_iter()
            .try_fold(partial_state, |current, action| current.apply(action))
            .unwrap();
        assert_eq!(partial_result.winner(), Some(Player::Alpha));
    }

    #[test]
    fn immobilization_is_reported_as_the_exact_shortest_mate() {
        let state = State {
            active_player: Player::Alpha,
            reserves: cards(&[
                (Card::Animal(Animal::Mouse), Player::Alpha),
                (Card::Animal(Animal::Ox), Player::Alpha),
            ]),
            r1: cards(&[(Card::Snipe, Player::Alpha)]),
            r2: CardMultiset::EMPTY,
            r3: CardMultiset::EMPTY,
            r4: CardMultiset::EMPTY,
            r5: CardMultiset::EMPTY,
            r6: cards(&[(Card::Snipe, Player::Beta)]),
            leading_action: None,
        };
        let mut iceberg = IcebergAnalyzer::new();
        iceberg.set_state(state.clone());
        for _ in 0..8 {
            iceberg.think_for_one_tick();
            if iceberg.is_fully_solved().is_some() {
                break;
            }
        }
        let mate = MateInN::new(Player::Alpha, 1).unwrap();
        assert_eq!(
            iceberg.is_fully_solved(),
            Some(OptimalOutcome::MateInN(mate))
        );
        let mut line = Vec::new();
        iceberg.write_optimal_lop(&mut line);
        let result = line
            .into_iter()
            .try_fold(state, |current, action| current.apply(action))
            .unwrap();
        assert_eq!(result.winner(), Some(Player::Alpha));
    }

    fn cards(entries: &[(Card, Player)]) -> CardMultiset {
        entries
            .iter()
            .fold(CardMultiset::EMPTY, |cards, &(card, player)| {
                cards
                    .checked_add(CardMultiset::singleton(card, player))
                    .unwrap()
            })
    }

    fn core_completed_positions(state: &State) -> HashSet<Position> {
        let active = state.active_player;
        let mut first_actions = Vec::new();
        state.write_legal_actions(&mut first_actions);
        let mut positions = HashSet::new();
        for first in first_actions {
            let after_first = state.clone().apply(first).unwrap();
            if after_first.winner().is_some() || after_first.active_player != active {
                positions.insert(Position::from_core(&after_first));
                continue;
            }
            let mut second_actions = Vec::new();
            after_first.write_legal_actions(&mut second_actions);
            if second_actions.is_empty() {
                positions.insert(Position::from_core(&after_first));
            } else {
                positions.extend(second_actions.into_iter().map(|second| {
                    Position::from_core(&after_first.clone().apply(second).unwrap())
                }));
            }
        }
        positions
    }
}
