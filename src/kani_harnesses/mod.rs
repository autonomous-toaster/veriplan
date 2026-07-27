//! Kani proof harnesses for veriplan's core translation and verification logic.
//!
//! These harnesses are only compiled when running `cargo kani`.
//! They verify:
//!   - BFS LTL evaluator correctly handles all LTL patterns
//!   - Naming convention between translator and Promela generator is consistent
//!   - Generated LTL formulas are syntactically valid
//!   - Generated Promela has balanced structure
//!   - Convertibility check severity invariants hold

mod bfs_evaluator;
mod convertibility;
mod naming_convention;
mod promela_generator;
mod translator;
