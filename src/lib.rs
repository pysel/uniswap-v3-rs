//! Uniswap SDK for Rust.

pub mod calltypes;
pub mod client;
pub mod errors;
mod features;
pub mod objects;

// Feature-gated modules keep their top-level paths (`crate::hedger`, `crate::strategies`, ...).
#[allow(unused_imports)] // empty when no features are enabled
pub use features::*;
