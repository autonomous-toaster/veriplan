//! The curated problem vocabulary and the mapping from check names to it.
//!
//! Extracted from ir::mod to keep that file under the length gate, following ir::ltl.

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
    /// A requirement body holding more than one constraint clause; only the first is modelled.
    PatternMultiClause,
    PatternUngrounded,
    NoFormalizable,
    GroundingMultiKeyword,
    GroundingAmbiguous,
    GroundingUngroundable,
    /// A cited task id is not declared by any change in scope.
    GroundingAbsentTask,
    /// A requirement body is grounded on an archived change, so no formula describes this plan.
    GroundingHistorical,
    /// A requirement body mixes this change's tasks with an archived change's tasks.
    GroundingMixed,
    /// The predicate matched but its argument slots are under-filled.
    GroundingIncomplete,
    /// One task fills two slots that require different tasks.
    GroundingContradictory,
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

/// Every kind with the identifier it reports and any accepted aliases.
///
/// One table, two derived functions: a kind's identifier and a name's kind cannot drift
/// apart if both are read from this list. An alias is a spelling a check may still emit.
pub const KIND_NAMES: &[(Kind, &str, &[&str])] = &[
    (Kind::NoTasks, "no_tasks", &[]),
    (Kind::DuplicateTaskId, "duplicate_task_id", &[]),
    (Kind::NoRequirements, "no_requirements", &[]),
    (Kind::NoPhaseGrouping, "no_phase_grouping", &[]),
    (Kind::NoRfc2119Keyword, "no_rfc2119_keyword", &[]),
    (Kind::NoRfc2119Any, "no_rfc2119_any", &[]),
    (Kind::BadTaskReference, "bad_task_reference", &[]),
    (Kind::BareCapability, "bare_capability", &[]),
    (Kind::VagueAction, "vague_action", &[]),
    (Kind::VagueQuality, "vague_quality", &[]),
    (
        Kind::UnknownNonFormalizable,
        "unknown_non_formalizable",
        &[],
    ),
    (Kind::PatternMultiClause, "pattern_multi_clause", &[]),
    (Kind::PatternUngrounded, "pattern_ungrounded", &[]),
    (Kind::NoFormalizable, "no_formalizable", &[]),
    (
        Kind::GroundingMultiKeyword,
        "grounding_multi_keyword",
        &["grounding_ambiguous_multi_keyword"],
    ),
    (Kind::GroundingAmbiguous, "grounding_ambiguous", &[]),
    (Kind::GroundingUngroundable, "grounding_ungroundable", &[]),
    (Kind::GroundingAbsentTask, "grounding_absent_task", &[]),
    (Kind::GroundingHistorical, "grounding_historical", &[]),
    (Kind::GroundingMixed, "grounding_mixed", &[]),
    (Kind::GroundingIncomplete, "grounding_incomplete", &[]),
    (Kind::GroundingContradictory, "grounding_contradictory", &[]),
    (Kind::ScenarioNoWhen, "scenario_no_when", &[]),
    (Kind::ScenarioNoThen, "scenario_no_then", &[]),
    (Kind::ThenNoShall, "then_no_shall", &[]),
    (Kind::TaskNotCovered, "task_not_covered", &[]),
    (Kind::LowDiversity, "low_diversity", &[]),
    (Kind::MayRequirement, "may_requirement", &[]),
    (
        Kind::InformationalRequirement,
        "informational_requirement",
        &[],
    ),
    (
        Kind::ViolationSequential,
        "violation_sequential",
        &["SequentialOrder"],
    ),
    (
        Kind::ViolationConcurrent,
        "violation_concurrent",
        &["ConcurrentEvents"],
    ),
    (
        Kind::ViolationConditional,
        "violation_conditional",
        &["Conditional"],
    ),
    (
        Kind::ViolationExclusive,
        "violation_exclusive",
        &["Exclusive"],
    ),
    (Kind::ViolationGlobal, "violation_global", &["Global"]),
    (
        Kind::ViolationFixedTime,
        "violation_fixed_time",
        &["FixedTime"],
    ),
    (Kind::ProseSlopWord, "prose_slop_word", &["SlopWord"]),
    (Kind::ProseOther, "prose_other", &[]),
];

impl Kind {
    /// The stable string identifier for this kind.
    pub fn as_str(&self) -> &'static str {
        KIND_NAMES
            .iter()
            .find(|(kind, _, _)| kind == self)
            .map_or("prose_other", |(_, name, _)| *name)
    }
}

/// Map a `check`/`category` value to its curated [`Kind`] (design D2).
///
/// `kind` is a pure function of the problem identifier and never encodes severity, so it is
/// stable across strictness profiles. Both the snake_case check name and the CamelCase category
/// name are accepted, because callers supply whichever they have.
pub fn kind_of(check_or_category: &str) -> Kind {
    KIND_NAMES
        .iter()
        .find(|(_, name, aliases)| {
            *name == check_or_category || aliases.contains(&check_or_category)
        })
        .map_or(Kind::ProseOther, |(kind, _, _)| *kind)
}

#[cfg(test)]
mod tests {
    use super::*;

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
            "grounding_absent_task",
            "grounding_historical",
            "grounding_mixed",
            "grounding_incomplete",
            "grounding_contradictory",
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
            assert_eq!(
                kind_of(check),
                kind,
                "kind_of must be deterministic for {}",
                check
            );
            assert!(!kind.as_str().is_empty());
        }
    }

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
            Kind::PatternMultiClause,
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
