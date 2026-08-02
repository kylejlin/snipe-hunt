//! Kiwi: Fajita's policy/value network and MCTS with continuously trained weights.
//!
//! The implementation deliberately delegates inference and search to Fajita's
//! public types so the two agents cannot drift architecturally. Kiwi owns only
//! its independently trained and published checkpoint.

use snipe_core::{ActionWriter, Analyzer, Evaluation, OptimalOutcome, State};

pub use agent_fajita::{
    ACTION_SIZE, INITIAL_SEED, INPUT_SIZE, Model, PARAM_COUNT, RESIDUAL_LAYERS, Search, WIDTH,
    action_index, encode_state, state_key,
};

#[cfg(feature = "training")]
pub use agent_fajita::training;

pub struct KiwiAnalyzer {
    inner: agent_fajita::FajitaAnalyzer,
}

impl Default for KiwiAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl KiwiAnalyzer {
    pub fn new() -> Self {
        let model = Model::from_bytes(include_bytes!(concat!(env!("OUT_DIR"), "/kiwi.bin")))
            .unwrap_or_else(|_| Model::seeded(INITIAL_SEED));
        Self::with_model(model)
    }

    pub fn with_model(model: Model) -> Self {
        Self {
            inner: agent_fajita::FajitaAnalyzer::with_model(model),
        }
    }
}

impl Analyzer for KiwiAnalyzer {
    fn set_state(&mut self, state: State) {
        self.inner.set_state(state);
    }

    fn think_for_one_tick(&mut self) {
        self.inner.think_for_one_tick();
    }

    fn is_fully_solved(&self) -> Option<OptimalOutcome> {
        self.inner.is_fully_solved()
    }

    fn evaluation(&self) -> Evaluation {
        self.inner.evaluation()
    }

    fn write_optimal_lop<W: ActionWriter>(&self, writer: &mut W) {
        self.inner.write_optimal_lop(writer);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use snipe_core::{MateInN, Player};
    use snipe_prng::initial_state;

    #[test]
    fn kiwi_uses_the_shared_model_and_search_contract() {
        let model = Model::seeded(INITIAL_SEED);
        let state = initial_state(7_071);
        let mut kiwi = KiwiAnalyzer::with_model(model.clone());
        let mut fajita = agent_fajita::FajitaAnalyzer::with_model(model);
        kiwi.set_state(state.clone());
        fajita.set_state(state);
        for _ in 0..32 {
            kiwi.think_for_one_tick();
            fajita.think_for_one_tick();
        }
        assert_eq!(kiwi.evaluation(), fajita.evaluation());
        let mut kiwi_line = Vec::new();
        let mut fajita_line = Vec::new();
        kiwi.write_optimal_lop(&mut kiwi_line);
        fajita.write_optimal_lop(&mut fajita_line);
        assert_eq!(kiwi_line, fajita_line);
    }

    #[test]
    fn unpublished_kiwi_starts_from_fresh_seeded_weights() {
        let published = include_bytes!(concat!(env!("OUT_DIR"), "/kiwi.bin"));
        if !published.is_empty() {
            return;
        }
        let state = initial_state(7_071);
        let mut kiwi = KiwiAnalyzer::new();
        let mut fresh = agent_fajita::FajitaAnalyzer::with_model(Model::seeded(INITIAL_SEED));
        kiwi.set_state(state.clone());
        fresh.set_state(state);
        for _ in 0..32 {
            kiwi.think_for_one_tick();
            fresh.think_for_one_tick();
        }
        assert_eq!(kiwi.evaluation(), fresh.evaluation());
        let mut kiwi_line = Vec::new();
        let mut fresh_line = Vec::new();
        kiwi.write_optimal_lop(&mut kiwi_line);
        fresh.write_optimal_lop(&mut fresh_line);
        assert_eq!(kiwi_line, fresh_line);
    }

    #[test]
    fn kiwi_recognizes_terminal_positions_without_search() {
        let mut state = initial_state(11);
        state.reserves = snipe_core::CardMultiset::EMPTY;
        state.r1 = snipe_core::CardMultiset::EMPTY;
        state.r2 = snipe_core::CardMultiset::EMPTY;
        state.r3 = snipe_core::CardMultiset::EMPTY;
        state.r4 = snipe_core::CardMultiset::EMPTY;
        state.r5 = snipe_core::CardMultiset::EMPTY;
        state.r6 = snipe_core::CardMultiset::EMPTY;
        state.r1 = snipe_core::CardMultiset::singleton(snipe_core::Card::Snipe, Player::Alpha);
        let mut kiwi = KiwiAnalyzer::with_model(Model::seeded(INITIAL_SEED));
        kiwi.set_state(state);
        assert_eq!(
            kiwi.evaluation(),
            MateInN::new(Player::Alpha, 0).unwrap().into()
        );
    }
}
