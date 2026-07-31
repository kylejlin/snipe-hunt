//! Dumpling v1: fixed-work iterative deepening alpha-beta with killzone
//! prepruning, move ordering, a transposition table, and a hand-tuned
//! material/pressure/snipe-safety evaluation.

use crate::search::{
    SearchConfig, Searcher, analyzer_evaluation, analyzer_fully_solved, analyzer_set_state,
    analyzer_think, analyzer_write_lop, assert_analyzer,
};
use snipe_core::{ActionWriter, Analyzer, Evaluation, OptimalOutcome, State};

const CONFIG: SearchConfig = SearchConfig {
    nodes_per_tick: 262_144,
    table_bits: 20,
    material_major: 1_200,
    material_minor: 350,
    alpha_material_minor: 350,
    beta_material_minor: 350,
    reserve_penalty: 40,
    breakthrough: 150,
    infiltration: 50,
    pressure: 70,
    control: 110,
    sanctuary: 3_000,
    trench: 1_000,
    snipe_pressure: 20_000,
    repetition: 0,
    late_move_reduction: true,
    aspiration_window: 2_000,
    maximum_full_moves: 24,
    move_count_pruning_depth: 2,
    beta_maximum_full_moves: 24,
    beta_move_count_pruning_depth: 2,
    beta_opening_completed_depth: 4,
    beta_opening_moves: 0,
    capture_threat_per_mille: 700,
    sanctuary_space: 200,
    snipe_safe_exit: 2_500,
    snipe_support: 2_000,
    snipe_setup_drop: 20_000,
    snipe_invader: 0,
    alpha_snipe_invader: 2_000,
    beta_snipe_invader: 2_000,
    snipe_near_pressure: 3_500,
    snipe_near_attacker: 1_200,
    snipe_home_distance: 500,
    snipe_proximity: 100,
};

pub struct DumplingV1Analyzer {
    searcher: Searcher,
}

impl DumplingV1Analyzer {
    pub fn new() -> Self {
        let analyzer = Self {
            searcher: Searcher::new(CONFIG),
        };
        assert_analyzer::<Self>();
        analyzer
    }

    pub fn completed_depth(&self) -> i8 {
        self.searcher.completed_depth()
    }

    pub fn nodes_per_tick(&self) -> u32 {
        self.searcher.nodes_per_tick()
    }
}

impl Default for DumplingV1Analyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl Analyzer for DumplingV1Analyzer {
    fn set_state(&mut self, state: State) {
        analyzer_set_state(&mut self.searcher, state);
    }

    fn think_for_one_tick(&mut self) {
        analyzer_think(&mut self.searcher);
    }

    fn is_fully_solved(&self) -> Option<OptimalOutcome> {
        analyzer_fully_solved(&self.searcher)
    }

    fn evaluation(&self) -> Evaluation {
        analyzer_evaluation(&self.searcher)
    }

    fn write_optimal_lop<W: ActionWriter>(&self, writer: &mut W) {
        analyzer_write_lop(&self.searcher, writer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_core::{Card, CardMultiset, MateInN, Player};

    fn cards(entries: &[(Card, Player)]) -> CardMultiset {
        entries
            .iter()
            .fold(CardMultiset::EMPTY, |cards, &(card, player)| {
                cards
                    .checked_add(CardMultiset::singleton(card, player))
                    .expect("test position has legal multiplicities")
            })
    }

    #[test]
    fn terminal_positions_are_stably_solved_without_thinking() {
        let state = State {
            active_player: Player::Beta,
            reserves: cards(&[(Card::Snipe, Player::Beta)]),
            r1: cards(&[(Card::Snipe, Player::Alpha)]),
            r2: CardMultiset::EMPTY,
            r3: CardMultiset::EMPTY,
            r4: CardMultiset::EMPTY,
            r5: CardMultiset::EMPTY,
            r6: CardMultiset::EMPTY,
            leading_action: None,
        };
        let mate = MateInN::new(Player::Alpha, 0).unwrap();
        let solved = Some(OptimalOutcome::MateInN(mate));
        let mut analyzer = DumplingV1Analyzer::new();
        analyzer.set_state(state);

        assert_eq!(analyzer.is_fully_solved(), solved);
        assert_eq!(analyzer.evaluation(), mate.into());
        let mut line = Vec::new();
        analyzer.write_optimal_lop(&mut line);
        assert!(line.is_empty());

        analyzer.think_for_one_tick();
        assert_eq!(analyzer.is_fully_solved(), solved);
        assert_eq!(analyzer.evaluation(), mate.into());
        let mut unchanged_line = Vec::new();
        analyzer.write_optimal_lop(&mut unchanged_line);
        assert_eq!(unchanged_line, line);
    }
}
