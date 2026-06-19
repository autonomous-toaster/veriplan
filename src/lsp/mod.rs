//! LSP server for veriplan — real-time diagnostics, completions, and navigation
//! for OpenSpec plans.
//!
//! Run with `veriplan lsp --stdio`.

pub mod code_actions;
pub mod completions;
pub mod diagnostics;
pub mod navigation;
pub mod state;
pub mod symbols;
pub mod transport;

pub use transport::run_lsp;
