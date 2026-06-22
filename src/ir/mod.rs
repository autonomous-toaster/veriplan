//! PlanIR — Intermediate Representation bridging parsing and verification.
//!
//! Every element carries a `SourceLocation` for bidirectional trace↔markdown
//! projection during counterexample annotation.

use std::collections::HashMap;

/// Byte-precise location in a source file from tree-sitter AST.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SourceLocation {
    pub file: String,
    pub start_byte: usize,
    pub end_byte: usize,
    pub start_line: usize,
    pub end_line: usize,
}

/// RFC 2119 keyword indicating requirement strength.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Rfc2119Strength {
    /// MUST / SHALL — hard constraint, blocks plan if violated
    Must,
    /// SHOULD — soft constraint, flagged but doesn't block
    Should,
    /// MAY — informational, not checked by model
    May,
    /// MUST NOT / SHALL NOT — hard prohibition, blocks plan if condition is true
    MustNot,
    /// No RFC 2119 keyword found
    None,
}

impl Rfc2119Strength {
    pub fn is_hard(&self) -> bool {
        matches!(self, Self::Must | Self::MustNot)
    }

    pub fn is_checked(&self) -> bool {
        !matches!(self, Self::May | Self::None)
    }
}

/// VeriPlan temporal constraint categories (from VeriPlan Table 1).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConstraintCategory {
    /// Fixed time blocks (e.g., "within 2-4 AM window")
    FixedTime,
    /// Sequential order (e.g., "X before Y")
    SequentialOrder,
    /// Concurrent events (e.g., "X and Y run together")
    ConcurrentEvents,
    /// Conditional (e.g., "if X fails then Y")
    Conditional,
    /// Exclusive (e.g., "at most one active at a time")
    Exclusive,
    /// Global invariant (e.g., "always available")
    Global,
    /// SHALL statement that doesn't match any category
    NonFormalizable,
    /// SHALL statement with a temporal pattern but no task references to ground it
    /// (e.g., "X SHALL complete before Y" where X and Y are not task IDs)
    PatternUngrounded,
}

/// A single task/action from tasks.md.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Task {
    /// N.M identifier (e.g., "1.3")
    pub id: String,
    /// Description text
    pub description: String,
    /// Phase name from section heading
    pub phase: String,
    /// Whether the task is checked (completed) in the checklist
    pub checked: bool,
    /// Source location in tasks.md
    pub source: SourceLocation,
}

/// A scenario step type.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum StepKind {
    Given,
    When,
    Then,
    And,
}

/// A single step within a scenario.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScenarioStep {
    pub kind: StepKind,
    pub text: String,
    pub source: SourceLocation,
}

/// A scenario attached to a requirement.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Scenario {
    pub name: String,
    pub steps: Vec<ScenarioStep>,
    pub source: SourceLocation,
}

/// A requirement parsed from spec.md.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Requirement {
    pub id: String,
    pub statement: String,
    pub strength: Rfc2119Strength,
    pub category: ConstraintCategory,
    /// Generated LTL formula (None if NonFormalizable).
    pub ltl: Option<String>,
    pub scenarios: Vec<Scenario>,
    pub source: SourceLocation,
}

/// Phase execution mode.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PhaseMode {
    /// Tasks execute one after another (default).
    Sequential,
    /// All tasks start simultaneously; intra-phase CONCURRENTLY is structurally guaranteed.
    Concurrent,
}

/// A phase grouping (from section headings in tasks.md).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Phase {
    pub name: String,
    pub task_ids: Vec<String>,
    pub mode: PhaseMode,
}

/// Convertibility check result status.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ConvertibilityStatus {
    /// Plan passes all checks — proceed to model checking
    Convertible,
    /// Plan is convertible but has warnings
    ConvertibleWithWarnings,
    /// Plan has blocking issues — must rephrase before model checking
    Blocking,
}

/// A single check result item.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CheckItem {
    pub severity: String, // "blocker", "warning", "info"
    pub check: String,
    pub element: String,
    pub location: String,
    pub detail: String,
    pub fix: Option<String>,
}

/// Feedback report from the convertibility check.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ConvertibilityReport {
    pub status: ConvertibilityStatus,
    pub blockers: Vec<CheckItem>,
    pub warnings: Vec<CheckItem>,
    pub info: Vec<CheckItem>,
    pub rephrase_directives: Vec<String>,
}

/// Bidirectional mapping from element IDs to source locations.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SourceMap {
    pub tasks: HashMap<String, SourceLocation>,
    pub requirements: HashMap<String, SourceLocation>,
    pub scenarios: HashMap<(String, String), SourceLocation>,
}

/// The full plan intermediate representation.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PlanIR {
    /// All tasks from tasks.md in checklist order.
    pub tasks: Vec<Task>,
    /// SHALL requirements from spec.md files.
    pub requirements: Vec<Requirement>,
    /// Scenarios from spec.md files.
    pub scenarios: Vec<Scenario>,
    /// Phase groupings from task sections.
    pub phases: Vec<Phase>,
    /// Bidirectional source location mapping.
    pub source_map: SourceMap,
}
