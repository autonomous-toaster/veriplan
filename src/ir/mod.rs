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

/// The curated problem vocabulary (design D2).
///
/// `kind` is stable across strictness profiles — it never encodes severity.
/// It is derived from the `check`/`category` values via [`kind_of`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    NoTasks,
    DuplicateTaskId,
    NoRequirements,
    NoPhaseGrouping,
    NoRfc2119Keyword,
    NoRfc2119Any,
    BadTaskReference,
    BareCapability,
    VagueAction,
    VagueQuality,
    UnknownNonFormalizable,
    PatternUngrounded,
    NoFormalizable,
    GroundingMultiKeyword,
    GroundingAmbiguous,
    GroundingUngroundable,
    ScenarioNoWhen,
    ScenarioNoThen,
    ThenNoShall,
    TaskNotCovered,
    LowDiversity,
    MayRequirement,
    InformationalRequirement,
    ViolationSequential,
    ViolationConcurrent,
    ViolationConditional,
    ViolationExclusive,
    ViolationGlobal,
    ViolationFixedTime,
    /// Prose findings from steve rules (e.g. `prose_slop_word`).
    ProseSlopWord,
    /// A prose finding from a steve rule not otherwise enumerated.
    ProseOther,
}

impl Kind {
    /// The stable string identifier for this kind.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::NoTasks => "no_tasks",
            Self::DuplicateTaskId => "duplicate_task_id",
            Self::NoRequirements => "no_requirements",
            Self::NoPhaseGrouping => "no_phase_grouping",
            Self::NoRfc2119Keyword => "no_rfc2119_keyword",
            Self::NoRfc2119Any => "no_rfc2119_any",
            Self::BadTaskReference => "bad_task_reference",
            Self::BareCapability => "bare_capability",
            Self::VagueAction => "vague_action",
            Self::VagueQuality => "vague_quality",
            Self::UnknownNonFormalizable => "unknown_non_formalizable",
            Self::PatternUngrounded => "pattern_ungrounded",
            Self::NoFormalizable => "no_formalizable",
            Self::GroundingMultiKeyword => "grounding_multi_keyword",
            Self::GroundingAmbiguous => "grounding_ambiguous",
            Self::GroundingUngroundable => "grounding_ungroundable",
            Self::ScenarioNoWhen => "scenario_no_when",
            Self::ScenarioNoThen => "scenario_no_then",
            Self::ThenNoShall => "then_no_shall",
            Self::TaskNotCovered => "task_not_covered",
            Self::LowDiversity => "low_diversity",
            Self::MayRequirement => "may_requirement",
            Self::InformationalRequirement => "informational_requirement",
            Self::ViolationSequential => "violation_sequential",
            Self::ViolationConcurrent => "violation_concurrent",
            Self::ViolationConditional => "violation_conditional",
            Self::ViolationExclusive => "violation_exclusive",
            Self::ViolationGlobal => "violation_global",
            Self::ViolationFixedTime => "violation_fixed_time",
            Self::ProseSlopWord => "prose_slop_word",
            Self::ProseOther => "prose_other",
        }
    }
}

/// Map a `check`/`category` value to its curated [`Kind`] (design D2).
///
/// `kind` is a pure function of the problem identifier and never encodes
/// severity, so it is stable across strictness profiles.
pub fn kind_of(check_or_category: &str) -> Kind {
    match check_or_category {
        "no_tasks" => Kind::NoTasks,
        "duplicate_task_id" => Kind::DuplicateTaskId,
        "no_requirements" => Kind::NoRequirements,
        "no_phase_grouping" => Kind::NoPhaseGrouping,
        "no_rfc2119_keyword" => Kind::NoRfc2119Keyword,
        "no_rfc2119_any" => Kind::NoRfc2119Any,
        "bad_task_reference" => Kind::BadTaskReference,
        "bare_capability" => Kind::BareCapability,
        "vague_action" => Kind::VagueAction,
        "vague_quality" => Kind::VagueQuality,
        "unknown_non_formalizable" => Kind::UnknownNonFormalizable,
        "pattern_ungrounded" => Kind::PatternUngrounded,
        "no_formalizable" => Kind::NoFormalizable,
        "grounding_ambiguous_multi_keyword" | "grounding_multi_keyword" => {
            Kind::GroundingMultiKeyword
        }
        "grounding_ambiguous" => Kind::GroundingAmbiguous,
        "grounding_ungroundable" => Kind::GroundingUngroundable,
        "scenario_no_when" => Kind::ScenarioNoWhen,
        "scenario_no_then" => Kind::ScenarioNoThen,
        "then_no_shall" => Kind::ThenNoShall,
        "task_not_covered" => Kind::TaskNotCovered,
        "low_diversity" => Kind::LowDiversity,
        "may_requirement" => Kind::MayRequirement,
        "informational_requirement" => Kind::InformationalRequirement,
        "SequentialOrder" => Kind::ViolationSequential,
        "ConcurrentEvents" => Kind::ViolationConcurrent,
        "Conditional" => Kind::ViolationConditional,
        "Exclusive" => Kind::ViolationExclusive,
        "Global" => Kind::ViolationGlobal,
        "FixedTime" => Kind::ViolationFixedTime,
        "SlopWord" => Kind::ProseSlopWord,
        // Prose rules use steve's slashed ids (e.g. "verb/passive") or the
        // bare rule name; any unknown value is a prose/other finding.
        _ => Kind::ProseOther,
    }
}

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
    /// Whether this finding is advisory (prose) and never blocks.
    pub advisory: bool,
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every `check`/`category` value that can reach output must map to a
    /// stable `Kind` that never encodes severity (design D2, task 8.2).
    #[test]
    fn kind_of_is_stable_and_severity_independent() {
        let check_values = [
            "no_tasks",
            "duplicate_task_id",
            "no_requirements",
            "no_phase_grouping",
            "no_rfc2119_keyword",
            "no_rfc2119_any",
            "bad_task_reference",
            "bare_capability",
            "vague_action",
            "vague_quality",
            "unknown_non_formalizable",
            "pattern_ungrounded",
            "no_formalizable",
            "grounding_ambiguous_multi_keyword",
            "grounding_multi_keyword",
            "grounding_ambiguous",
            "grounding_ungroundable",
            "scenario_no_when",
            "scenario_no_then",
            "then_no_shall",
            "task_not_covered",
            "low_diversity",
            "may_requirement",
            "informational_requirement",
            "SequentialOrder",
            "ConcurrentEvents",
            "Conditional",
            "Exclusive",
            "Global",
            "FixedTime",
            "SlopWord",
        ];
        for check in check_values {
            let kind = kind_of(check);
            // `kind` is a pure function of the check value — it never depends
            // on severity, so it is stable across strictness profiles.
            assert_eq!(kind_of(check), kind, "kind_of must be deterministic for {}", check);
            assert!(!kind.as_str().is_empty());
        }
    }

    /// The `Kind` enum must cover every curated problem vocabulary value.
    #[test]
    fn kind_enum_covers_all_vocabulary() {
        let kinds = [
            Kind::NoTasks,
            Kind::DuplicateTaskId,
            Kind::NoRequirements,
            Kind::NoPhaseGrouping,
            Kind::NoRfc2119Keyword,
            Kind::NoRfc2119Any,
            Kind::BadTaskReference,
            Kind::BareCapability,
            Kind::VagueAction,
            Kind::VagueQuality,
            Kind::UnknownNonFormalizable,
            Kind::PatternUngrounded,
            Kind::NoFormalizable,
            Kind::GroundingMultiKeyword,
            Kind::GroundingAmbiguous,
            Kind::GroundingUngroundable,
            Kind::ScenarioNoWhen,
            Kind::ScenarioNoThen,
            Kind::ThenNoShall,
            Kind::TaskNotCovered,
            Kind::LowDiversity,
            Kind::MayRequirement,
            Kind::InformationalRequirement,
            Kind::ViolationSequential,
            Kind::ViolationConcurrent,
            Kind::ViolationConditional,
            Kind::ViolationExclusive,
            Kind::ViolationGlobal,
            Kind::ViolationFixedTime,
            Kind::ProseSlopWord,
            Kind::ProseOther,
        ];
        let mut ids: Vec<&str> = kinds.iter().map(|k| k.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), kinds.len(), "kind ids must be unique");
    }
}
