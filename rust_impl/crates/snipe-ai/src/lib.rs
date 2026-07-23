//! A deterministic, deadline-safe game-tree searcher for Snipe Hunt.
//!
//! The search is deliberately separated from `snipe-core` by [`GamePosition`].
//! This makes the difficult search code independently testable and leaves one
//! small integration point for the rules engine.

mod arena;
mod core_adapter;
mod evaluation;
mod search;

pub use arena::{play_match, ArenaResult, MatchSummary};
pub use core_adapter::{evaluate_state, extract_features, tactical_move_score};
pub use evaluation::{SnipeFeatures, SnipeWeights};
pub use search::{GamePosition, SearchConfig, SearchEngine, SearchResult, SearchStats, MATE_SCORE};
