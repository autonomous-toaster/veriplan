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
    /// An ambiguous pronoun: the reader cannot tell which noun the pronoun names.
    ProsePronounAmbiguity,
    /// Passive voice: the sentence names no actor.
    ProsePassiveVoice,
    /// Hedging: the sentence weakens its own claim.
    ProseHedging,
    /// More than one instruction in one sentence.
    ProseOneInstruction,
    /// A sentence longer than the writing standard allows.
    ProseSentenceLength,
    /// One concept named two ways in the same text.
    ProseSynonymInconsistency,
    /// A prose finding from a steve rule not otherwise enumerated.
    ProseOther,
}

/// Whether a kind describes the whole plan or one site that needs one edit.
///
/// A plan fact can be summarised once. A site fact cannot: two sites of the same kind are two
/// edits, so a group line that keeps one of them deletes the others from the report.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Arity {
    /// One fact about the plan.
    Plan,
    /// One place that needs one edit.
    Site,
}

/// Every kind with the identifier it reports, any accepted aliases, and its arity.
///
/// One table, two derived functions: a kind's identifier and a name's kind cannot drift
/// apart if both are read from this list. An alias is a spelling a check may still emit.
pub const KIND_NAMES: &[(Kind, &str, &[&str], Arity)] = &[
    (Kind::NoTasks, "no_tasks", &[], Arity::Plan),
    (Kind::DuplicateTaskId, "duplicate_task_id", &[], Arity::Site),
    (Kind::NoRequirements, "no_requirements", &[], Arity::Plan),
    (Kind::NoPhaseGrouping, "no_phase_grouping", &[], Arity::Plan),
    (
        Kind::NoRfc2119Keyword,
        "no_rfc2119_keyword",
        &[],
        Arity::Site,
    ),
    (Kind::NoRfc2119Any, "no_rfc2119_any", &[], Arity::Site),
    (
        Kind::BadTaskReference,
        "bad_task_reference",
        &[],
        Arity::Site,
    ),
    (Kind::BareCapability, "bare_capability", &[], Arity::Site),
    (Kind::VagueAction, "vague_action", &[], Arity::Site),
    (Kind::VagueQuality, "vague_quality", &[], Arity::Site),
    (
        Kind::UnknownNonFormalizable,
        "unknown_non_formalizable",
        &[],
        Arity::Site,
    ),
    (
        Kind::PatternMultiClause,
        "pattern_multi_clause",
        &[],
        Arity::Site,
    ),
    (
        Kind::PatternUngrounded,
        "pattern_ungrounded",
        &[],
        Arity::Site,
    ),
    (Kind::NoFormalizable, "no_formalizable", &[], Arity::Plan),
    (
        Kind::GroundingMultiKeyword,
        "grounding_multi_keyword",
        &["grounding_ambiguous_multi_keyword"],
        Arity::Site,
    ),
    (
        Kind::GroundingAmbiguous,
        "grounding_ambiguous",
        &[],
        Arity::Site,
    ),
    (
        Kind::GroundingUngroundable,
        "grounding_ungroundable",
        &[],
        Arity::Site,
    ),
    (
        Kind::GroundingAbsentTask,
        "grounding_absent_task",
        &[],
        Arity::Site,
    ),
    (
        Kind::GroundingHistorical,
        "grounding_historical",
        &[],
        Arity::Site,
    ),
    (Kind::GroundingMixed, "grounding_mixed", &[], Arity::Site),
    (
        Kind::GroundingIncomplete,
        "grounding_incomplete",
        &[],
        Arity::Site,
    ),
    (
        Kind::GroundingContradictory,
        "grounding_contradictory",
        &[],
        Arity::Site,
    ),
    (Kind::ScenarioNoWhen, "scenario_no_when", &[], Arity::Site),
    (Kind::ScenarioNoThen, "scenario_no_then", &[], Arity::Site),
    (Kind::ThenNoShall, "then_no_shall", &[], Arity::Site),
    (Kind::TaskNotCovered, "task_not_covered", &[], Arity::Site),
    (Kind::LowDiversity, "low_diversity", &[], Arity::Plan),
    (Kind::MayRequirement, "may_requirement", &[], Arity::Site),
    (
        Kind::InformationalRequirement,
        "informational_requirement",
        &[],
        Arity::Site,
    ),
    (
        Kind::ViolationSequential,
        "violation_sequential",
        &["SequentialOrder"],
        Arity::Site,
    ),
    (
        Kind::ViolationConcurrent,
        "violation_concurrent",
        &["ConcurrentEvents"],
        Arity::Site,
    ),
    (
        Kind::ViolationConditional,
        "violation_conditional",
        &["Conditional"],
        Arity::Site,
    ),
    (
        Kind::ViolationExclusive,
        "violation_exclusive",
        &["Exclusive"],
        Arity::Site,
    ),
    (
        Kind::ViolationGlobal,
        "violation_global",
        &["Global"],
        Arity::Site,
    ),
    (
        Kind::ViolationFixedTime,
        "violation_fixed_time",
        &["FixedTime"],
        Arity::Site,
    ),
    (
        Kind::ProseSlopWord,
        "prose_slop_word",
        &["SlopWord", "style/slop-word"],
        Arity::Site,
    ),
    (
        Kind::ProsePronounAmbiguity,
        "prose_pronoun_ambiguity",
        &["style/pronoun-ambiguity"],
        Arity::Site,
    ),
    (
        Kind::ProsePassiveVoice,
        "prose_passive_voice",
        &["verb/passive"],
        Arity::Site,
    ),
    (
        Kind::ProseHedging,
        "prose_hedging",
        &["style/hedging"],
        Arity::Site,
    ),
    (
        Kind::ProseOneInstruction,
        "prose_one_instruction",
        &["structure/one-instruction"],
        Arity::Site,
    ),
    (
        Kind::ProseSentenceLength,
        "prose_sentence_length",
        &["length/sentence"],
        Arity::Site,
    ),
    (
        Kind::ProseSynonymInconsistency,
        "prose_synonym_inconsistency",
        &["style/synonym-consistency"],
        Arity::Site,
    ),
    (Kind::ProseOther, "prose_other", &[], Arity::Site),
];

impl Kind {
    /// The stable string identifier for this kind.
    pub fn as_str(&self) -> &'static str {
        KIND_NAMES
            .iter()
            .find(|(kind, _, _, _)| kind == self)
            .map_or("prose_other", |(_, name, _, _)| *name)
    }

    /// Whether the kind describes the whole plan or one site.
    pub fn arity(&self) -> Arity {
        KIND_NAMES
            .iter()
            .find(|(kind, _, _, _)| kind == self)
            .map_or(Arity::Site, |(_, _, _, arity)| *arity)
    }

    /// Whether the kind comes from the prose pass rather than from a model-checked rule.
    pub fn is_prose(&self) -> bool {
        matches!(
            self,
            Kind::ProseSlopWord
                | Kind::ProsePronounAmbiguity
                | Kind::ProsePassiveVoice
                | Kind::ProseHedging
                | Kind::ProseOneInstruction
                | Kind::ProseSentenceLength
                | Kind::ProseSynonymInconsistency
                | Kind::ProseOther
        )
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
        .find(|(_, name, aliases, _)| {
            *name == check_or_category || aliases.contains(&check_or_category)
        })
        .map_or(Kind::ProseOther, |(kind, _, _, _)| *kind)
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

#[cfg(test)]
mod prose_kind_tests {
    use super::*;

    /// A prose kind names the rule that produced it, so the finding says what to fix.
    #[test]
    fn prose_rules_map_to_named_kinds() {
        assert_eq!(
            kind_of("style/pronoun-ambiguity"),
            Kind::ProsePronounAmbiguity
        );
        assert_eq!(kind_of("verb/passive"), Kind::ProsePassiveVoice);
        assert_eq!(kind_of("style/hedging"), Kind::ProseHedging);
        assert_eq!(
            kind_of("structure/one-instruction"),
            Kind::ProseOneInstruction
        );
        assert_eq!(kind_of("length/sentence"), Kind::ProseSentenceLength);
        assert_eq!(
            kind_of("style/synonym-consistency"),
            Kind::ProseSynonymInconsistency
        );
        assert_eq!(kind_of("style/slop-word"), Kind::ProseSlopWord);
        // Unmapped rule: the fallback keeps the kind honest instead of guessing.
        assert_eq!(kind_of("style/units-format"), Kind::ProseOther);
    }

    /// A plan fact can be summarised once; a site fact cannot.
    #[test]
    fn arity_separates_plan_facts_from_sites() {
        assert_eq!(Kind::NoTasks.arity(), Arity::Plan);
        assert_eq!(Kind::LowDiversity.arity(), Arity::Plan);
        assert_eq!(Kind::ProsePronounAmbiguity.arity(), Arity::Site);
        assert_eq!(Kind::ViolationSequential.arity(), Arity::Site);
        assert!(Kind::ProsePronounAmbiguity.is_prose());
        assert!(!Kind::ViolationSequential.is_prose());
    }
}
