//! PlanIR — Intermediate Representation bridging parsing and verification.
//!
//! Every element carries a `SourceLocation` for bidirectional trace↔markdown
//! projection during counterexample annotation.

use std::collections::BTreeMap;

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
    /// A normative requirement explicitly marked as informational / human-review-only
    /// (e.g. body contains "human review only"). Not a temporal constraint; it is
    /// surfaced as INFO and does not block the plan.
    Informational,
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

/// The history a requirement's body was written against.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Grounding {
    /// The archived change that defines the cited tasks.
    pub change: String,
    /// The task ids that change defines, in checklist order.
    pub tasks: Vec<String>,
}

/// A requirement parsed from spec.md.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Requirement {
    pub id: String,
    /// The archived change that grounds this requirement, when one can be resolved.
    ///
    /// Filled during load so that grounding and translation both read one value, and so that a
    /// published body's ids can be told apart from this change's tasks without either of them
    /// knowing about the filesystem.
    #[serde(default)]
    pub grounded_in: Option<Grounding>,
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
    /// The offending text, when the constructor had it in hand. The formatter renders it and
    /// never reconstructs it from a location.
    pub excerpt: Option<String>,
    /// The curated problem kind (stable across strictness).
    pub kind: Kind,
    /// The remedy this finding prescribes (drives `--fix`).
    pub op: Op,
    /// Whether this finding is safely auto-appliable (`Local`) or needs
    /// judgment (`Structural`).
    pub fixability: Fixability,
    /// Byte offset of the start of the offending text (for span edits).
    pub start: usize,
    /// Byte offset of the end of the offending text (for span edits).
    pub end: usize,
    /// A structured replacement for the offending span, when a deterministic
    /// fix exists (`Local` findings). `None` for `Structural` findings.
    pub replacement: Option<String>,
}

/// Whether a finding can be auto-applied deterministically (design D3).
///
/// Mirrors steve's `Fixability` and clippy's applicability tiers.
/// `Local` findings have a data-driven, byte-recoverable replacement that
/// `--fix` may apply. `Structural` findings need judgment (an AI or human
/// rewrite) and are surfaced as suggestions only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Fixability {
    /// A deterministic, data-driven replacement (e.g. `duplicate_task_id`
    /// rename, prose `SlopWord` with a plain replacement).
    Local,
    /// A finding that needs judgment (e.g. `split_requirement`, passive
    /// voice). Never auto-applied by `--fix`.
    Structural,
}

/// The remedy a finding prescribes (design D2).
///
/// `op` is orthogonal to `kind`: two different `kind`s may share one `op`,
/// and `op` is what drives `--fix` eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Op {
    /// Split one requirement into several, one temporal keyword each.
    SplitRequirement,
    /// Rename a task ID (e.g. resolving a duplicate).
    RenameTask,
    /// Replace a requirement body.
    ReplaceBody,
    /// Add a task-ID reference to a requirement.
    AddTaskReference,
    /// Remove a requirement.
    RemoveRequirement,
    /// Add a scenario step.
    AddScenarioStep,
    /// Fix a task reference (e.g. a bad `T99`).
    FixReference,
    /// Add a temporal keyword to a requirement.
    AddTemporalKeyword,
    /// Informational only — no edit prescribed.
    InformationalOnly,
    /// No remedy.
    None,
}

mod kind;
pub use kind::{Arity, Kind, kind_of};
/// A canonical, machine-readable finding (design D1).
///
/// This is the unified output contract shared by convertibility blockers,
/// model-check violations, and prose findings. It mirrors steve's `Finding`
/// shape and adds veriplan-specific fields (`kind`, `op`, `requirement_id`,
/// `advisory`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Finding {
    /// The curated problem kind (stable across strictness).
    pub kind: String,
    /// "blocker", "warning", or "info" (a strictness-mutable projection).
    pub severity: String,
    /// Source file.
    pub file: String,
    /// 1-based line.
    pub line: usize,
    /// 1-based column (byte offset within the line).
    pub column: usize,
    /// Byte offset of the start of the offending text.
    pub start: usize,
    /// Byte offset of the end of the offending text.
    pub end: usize,
    /// Human-readable description of the violation.
    pub message: String,
    /// A concrete fix suggestion, if available.
    pub suggestion: Option<String>,
    /// A structured replacement for the offending span, when a deterministic
    /// fix exists (`Local` findings). `None` for `Structural` findings.
    pub replacement: Option<String>,
    /// Whether this finding is safely auto-appliable (`Local`) or needs
    /// judgment (`Structural`).
    pub fixability: Fixability,
    /// The remedy this finding prescribes (drives `--fix`).
    pub op: Op,
    /// The requirement this finding belongs to, if any.
    pub requirement_id: Option<String>,
    /// Whether no model formula backs this finding. This flag describes the model, not the
    /// consequence: a prose finding that blocks the verdict still reports `advisory: true`.
    pub advisory: bool,
    /// The element that owns the finding (requirement, scenario step, task). Empty for a
    /// plan-scoped finding.
    pub element: String,
    /// The offending text, when the constructor had it in hand. The formatter renders it and
    /// never reconstructs it from a location.
    pub excerpt: Option<String>,
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
    pub tasks: BTreeMap<String, SourceLocation>,
    pub requirements: BTreeMap<String, SourceLocation>,
    pub scenarios: BTreeMap<(String, String), SourceLocation>,
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

pub mod ltl;
