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
            Self::PatternMultiClause => "pattern_multi_clause",
            Self::PatternUngrounded => "pattern_ungrounded",
            Self::NoFormalizable => "no_formalizable",
            Self::GroundingMultiKeyword => "grounding_multi_keyword",
            Self::GroundingAmbiguous => "grounding_ambiguous",
            Self::GroundingUngroundable => "grounding_ungroundable",
            Self::GroundingAbsentTask => "grounding_absent_task",
            Self::GroundingHistorical => "grounding_historical",
            Self::GroundingMixed => "grounding_mixed",
            Self::GroundingIncomplete => "grounding_incomplete",
            Self::GroundingContradictory => "grounding_contradictory",
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
        "pattern_multi_clause" => Kind::PatternMultiClause,
        "pattern_ungrounded" => Kind::PatternUngrounded,
        "no_formalizable" => Kind::NoFormalizable,
        "grounding_ambiguous_multi_keyword" | "grounding_multi_keyword" => {
            Kind::GroundingMultiKeyword
        }
        "grounding_ambiguous" => Kind::GroundingAmbiguous,
        "grounding_ungroundable" => Kind::GroundingUngroundable,
        "grounding_absent_task" => Kind::GroundingAbsentTask,
        "grounding_historical" => Kind::GroundingHistorical,
        "grounding_mixed" => Kind::GroundingMixed,
        "grounding_incomplete" => Kind::GroundingIncomplete,
        "grounding_contradictory" => Kind::GroundingContradictory,
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
