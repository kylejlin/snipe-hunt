//! Dumpling: allocation-free, aggressively pruned minimax for Snipe Hunt.
//!
//! Each promoted engine remains available as a versioned module so a new
//! challenger can be tested directly against the frozen predecessor.

mod packed;
mod search;

pub mod v1;

pub use v1::DumplingV1Analyzer;

/// The strongest accepted Dumpling version.
pub type DumplingAnalyzer = DumplingV1Analyzer;
