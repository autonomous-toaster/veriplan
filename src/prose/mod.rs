//! Prose-guidance: run a curated subset of steve's writing rules over the
//! prose veriplan actually cares about, and correlate findings with grounding.
//!
//! veriplan validates *semantic* ambiguity (task refs, temporal keywords,
//! grounding). This module adds cheap, deterministic *lexical/stylistic*
//! ambiguity detection via steve's clarity rules: passive voice, pronoun
//! ambiguity, hedging, one-instruction-per-sentence, synonym consistency, and
//! sentence length.
//!
//! Why a curated subset? Most of steve's rules fight OpenSpec's required
//! grammar and vocabulary: `shall`/`may`/`should` (RFC 2119), `**GIVEN**`/
//! `**WHEN**`/`**THEN**` scenario scaffolding, temporal predicates, and
//! technical vocabulary. So we enable only the rules that serve *unambiguity*
//! and correlate with grounding trouble (a passive requirement names no task
//! agent, so it grounds poorly).
//!
//! Prose findings are advisory only — they never block a plan.

use std::path::Path;

use steve::{RuleId, Severity, Ste, SteBuilder};

use crate::input::StrictnessProfile;
use crate::ir::{PlanIR, Rfc2119Strength};

/// A prose-guidance finding from steve's curated rules.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProseFinding {
    /// "blocker", "warning", or "info" (advisory — never used to block).
    pub severity: String,
    /// The steve rule id string (e.g. "verb/passive").
    pub rule: String,
    /// Source file the prose came from.
    pub file: String,
    /// 1-based line within the checked snippet (requirement body / task desc).
    pub line: usize,
    /// 1-based column (byte offset within the line).
    pub column: usize,
    /// The requirement or task this prose belongs to, if any.
    pub element: String,
    /// Human-readable finding message.
    pub message: String,
    /// Optional suggested replacement.
    pub suggestion: Option<String>,
    /// The offending snippet text.
    pub snippet: String,
    /// Byte offset of the start of the offending text.
    pub start: usize,
    /// Byte offset of the end of the offending text.
    pub end: usize,
    /// A structured replacement for the offending span, when a deterministic
    /// fix exists (`Local` findings). `None` for `Structural` findings.
    pub replacement: Option<String>,
    /// Whether this finding is safely auto-appliable (`Local`) or needs
    /// judgment (`Structural`).
    pub fixability: crate::ir::Fixability,
    /// The official ASD-STE100 (Issue 9) rule number(s), if any.
    pub ste_rule: Option<String>,
}

/// A combined rephrase directive that fixes both style and grounding.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CombinedDirective {
    /// The requirement id.
    pub requirement_id: String,
    /// Severity ("blocker" in Strict, "warning" in Moderate, "info" in Lax).
    pub severity: String,
    /// The actionable combined directive text.
    pub directive: String,
}

/// Every steve rule. We start with all rules off, then enable only the
/// curated set, so the policy is explicit and auditable.
fn all_rules() -> Vec<RuleId> {
    vec![
        RuleId::DictionaryNotApprovedWord,
        RuleId::SentenceLength,
        RuleId::ParagraphLength,
        RuleId::PassiveVoice,
        RuleId::Progressive,
        RuleId::PerfectTense,
        RuleId::Contraction,
        RuleId::Semicolon,
        RuleId::PhrasalVerb,
        RuleId::OneInstructionPerSentence,
        RuleId::SynonymConsistency,
        RuleId::Hedging,
        RuleId::ListsForSequences,
        RuleId::OneWordOnePos,
        RuleId::NounCluster,
        RuleId::TechnicalNounAsVerb,
        RuleId::ConditionBeforeCommand,
        RuleId::ModalLadder,
        RuleId::SlopWord,
        RuleId::AntiTerseness,
        RuleId::ImperativeMood,
        RuleId::PronounAmbiguity,
        RuleId::TopicSentence,
        RuleId::UnitsFormat,
    ]
}

/// The full curated set applied to spec.md requirement prose.
fn curated_rules() -> Vec<RuleId> {
    vec![
        RuleId::PassiveVoice,
        RuleId::PronounAmbiguity,
        RuleId::Hedging,
        RuleId::OneInstructionPerSentence,
        RuleId::SynonymConsistency,
        RuleId::SentenceLength,
        // SlopWord is a steve-`Local` rule: it replaces AI-slop ("leverage" →
        // "use") deterministically, so prose can produce machine-applicable
        // findings for `--fix` (design D7 / F2).
        RuleId::SlopWord,
    ]
}

/// Which rules apply to which OpenSpec artifact (design D2).
fn rules_for(artifact: &str) -> Vec<RuleId> {
    match artifact {
        // tasks.md: minimal set — descriptions become grounding aliases.
        "tasks" => vec![RuleId::OneInstructionPerSentence, RuleId::Hedging],
        // design/proposal: light set, more human-oriented.
        "design" => vec![RuleId::PassiveVoice, RuleId::PronounAmbiguity, RuleId::Hedging],
        // spec.md: full curated set.
        _ => curated_rules(),
    }
}

/// Map a strictness profile to per-rule severities (design D3).
fn severity(mode: &StrictnessProfile, rule: RuleId) -> Severity {
    match mode {
        StrictnessProfile::Strict => match rule {
            RuleId::PassiveVoice | RuleId::OneInstructionPerSentence => Severity::Hard,
            _ => Severity::Soft,
        },
        // Moderate and Lax: steve has no "info", so we use soft and then
        // downgrade Lax findings to info at the CheckItem boundary.
        _ => Severity::Soft,
    }
}

/// Build the curated `Ste` for a given artifact kind and strictness mode.
fn curated_ste(mode: &StrictnessProfile, artifact: &str) -> Result<Ste, steve::ConfigError> {
    let mut b: SteBuilder = Ste::builder();
    for r in all_rules() {
        b = b.rule(r, Severity::Off);
    }
    for r in rules_for(artifact) {
        b = b.rule(r, severity(mode, r));
    }
    // OpenSpec requirement bodies run several clauses (e.g.
    // "T2.1 SHALL complete BEFORE T3.2 SHALL run"); use a 30-word cap rather
    // than the fixed 20-word procedural default.
    if artifact == "spec" {
        b = b.max_sentence_words(30);
    }
    b.build()
}

/// The safe STE subset applied to scenario step content (spec R1.3).
///
/// Only PronounAmbiguity and SentenceLength. PassiveVoice and
/// OneInstructionPerSentence are excluded because they false-positive on
/// legitimate state assertions ("**THEN** the plan SHALL be marked VALID").
/// SynonymConsistency is a document-level rule that does not fit the per-step
/// model and fires on meta-scenarios that cite multiple words.
fn scenario_rules() -> Vec<RuleId> {
    vec![RuleId::PronounAmbiguity, RuleId::SentenceLength]
}

/// Build the `Ste` for scenario step content with the safe subset.
fn scenario_ste(mode: &StrictnessProfile) -> Result<Ste, steve::ConfigError> {
    let mut b: SteBuilder = Ste::builder();
    for r in all_rules() {
        b = b.rule(r, Severity::Off);
    }
    for r in scenario_rules() {
        b = b.rule(r, severity(mode, r));
    }
    // Scenario steps are typically one assertion; use the 30-word cap so a
    // longer but valid step is not spuriously flagged.
    b = b.max_sentence_words(30);
    b.build()
}

/// Strip `**GIVEN**`/`**WHEN**`/`**THEN**`/`**AND**` markers and inline code
/// spans from a scenario step before prose checking (spec R1.2).
fn strip_scenario_step(text: &str) -> String {
    let mut s = text.trim().to_string();
    for m in ["**GIVEN**", "**WHEN**", "**THEN**", "**AND**"] {
        s = s.replace(m, "");
    }
    let mut out = String::new();
    let mut in_code = false;
    for c in s.chars() {
        if c == '`' {
            in_code = !in_code;
            continue;
        }
        if !in_code {
            out.push(c);
        }
    }
    out.trim().to_string()
}

/// Convert a steve severity to a veriplan CheckItem severity, downgrading to
/// "info" in Lax mode (design D3 / D5: advisory only, never blocking).
fn veriplan_severity(sev: &steve::Severity, mode: &StrictnessProfile) -> &'static str {
    if *mode == StrictnessProfile::Lax {
        return "info";
    }
    match sev {
        Severity::Hard => "blocker",
        _ => "warning",
    }
}

/// Run steve's curated rules over one snippet, returning findings.
fn check_snippet(
    ste: &Ste,
    snippet: &str,
    file: &str,
    element: &str,
    mode: &StrictnessProfile,
) -> Vec<ProseFinding> {
    let Ok(report) = ste.check_text(snippet) else {
        return Vec::new();
    };
    report
        .iter()
        .map(|f| ProseFinding {
            severity: veriplan_severity(&f.severity(), mode).to_string(),
            rule: f.rule().as_str().to_string(),
            file: file.to_string(),
            line: f.line(),
            column: f.column(),
            element: element.to_string(),
            message: f.message().to_string(),
            suggestion: f.suggestion().map(|s| s.to_string()),
            snippet: f.snippet().to_string(),
            start: f.start(),
            end: f.end(),
            replacement: f.replacement().map(|r| r.to_string()),
            // steve derives `fixability` from the rule, so `SlopWord` is
            // `Structural` even when it carries a plain replacement. The
            // design (D7) requires SlopWord-with-replacement to be `Local`
            // (machine-applicable), so we override it here: a deterministic
            // replacement makes the edit byte-recoverable.
            fixability: if f.rule() == steve::RuleId::SlopWord && f.replacement().is_some() {
                crate::ir::Fixability::Local
            } else {
                match f.fixability() {
                    steve::Fixability::Local => crate::ir::Fixability::Local,
                    steve::Fixability::Structural => crate::ir::Fixability::Structural,
                }
            },
            ste_rule: f.ste_rule().map(|s| s.to_string()),
        })
        .collect()
}

/// Run prose-guidance over a plan's requirement bodies and task descriptions.
///
/// `spec_dir` is the change directory (used for design.md/proposal.md prose);
/// pass `None` when it is unavailable (e.g. stdin/single-file input) to skip
/// design/proposal checking.
pub fn check_prose(
    plan: &PlanIR,
    spec_dir: Option<&Path>,
    mode: &StrictnessProfile,
) -> Vec<ProseFinding> {
    let mut findings = Vec::new();

    // spec.md: requirement body prose (full curated set).
    let ste_spec = match curated_ste(mode, "spec") {
        Ok(s) => s,
        Err(_) => return findings,
    };
    for req in &plan.requirements {
        if req.strength == Rfc2119Strength::May
            || crate::translator::classify(&req.statement)
                == crate::ir::ConstraintCategory::Informational
        {
            continue; // informational; skip prose guidance.
        }
        let file = req.source.file.clone();
        let element = format!("Requirement '{}'", req.id);
        findings.extend(check_snippet(&ste_spec, &req.statement, &file, &element, mode));
    }

    // spec.md: scenario step content (safe subset). Scenarios are parsed and
    // available at the plan level as standalone scenarios (spec R1.1, R3.1).
    let ste_scenario = match scenario_ste(mode) {
        Ok(s) => s,
        Err(_) => return findings,
    };
    for scenario in &plan.scenarios {
        for step in &scenario.steps {
            let stripped = strip_scenario_step(&step.text);
            if stripped.is_empty() {
                continue;
            }
            let element = format!("Scenario '{}' ({:?} step)", scenario.name, step.kind);
            findings.extend(check_snippet(
                &ste_scenario,
                &stripped,
                &scenario.source.file,
                &element,
                mode,
            ));
        }
    }

    // tasks.md: task descriptions (minimal set).
    let ste_tasks = match curated_ste(mode, "tasks") {
        Ok(s) => s,
        Err(_) => return findings,
    };
    for task in &plan.tasks {
        let file = task.source.file.clone();
        let element = format!("Task {}", task.id);
        findings.extend(check_snippet(&ste_tasks, &task.description, &file, &element, mode));
    }

    // design.md / proposal.md: light set on body paragraphs.
    if let Some(dir) = spec_dir {
        let ste_design = match curated_ste(mode, "design") {
            Ok(s) => s,
            Err(_) => return findings,
        };
        for artifact in ["design.md", "proposal.md"] {
            let path = dir.join(artifact);
            if let Ok(content) = std::fs::read_to_string(&path) {
                findings.extend(check_snippet(
                    &ste_design,
                    &content,
                    artifact,
                    artifact,
                    mode,
                ));
            }
        }
    }

    findings
}

/// Correlate steve style findings with grounding outcomes per requirement.
///
/// When a requirement has both a steve style finding (passive/pronoun/hedging)
/// AND a grounding outcome that is `Ungroundable` or `Ambiguous`, produce a
/// single combined directive telling the assistant to name the task agent.
///
/// Runs in all strictness modes including Lax (where ungrounded is info-only
/// and the steve hint may be the sole weak-requirement signal).
pub fn correlate_with_grounding(
    findings: &[ProseFinding],
    plan: &PlanIR,
    mode: &StrictnessProfile,
) -> Vec<CombinedDirective> {
    // Compute grounding outcomes per requirement.
    let (_, _, _, outcomes) = crate::grounding::check_grounding(plan, mode);

    // Style-finding requirements: passive, pronoun, or hedging on a requirement.
    let style_ids: std::collections::HashSet<String> = findings
        .iter()
        .filter(|f| {
            f.element.starts_with("Requirement '")
                && matches!(
                    f.rule.as_str(),
                    "verb/passive" | "style/pronoun-ambiguity" | "style/hedging"
                )
        })
        .filter_map(|f| f.element.strip_prefix("Requirement '").map(|s| s.trim_end_matches('\'').to_string()))
        .collect();

    let mut directives = Vec::new();
    for outcome in &outcomes {
        let failed = outcome.failed || matches!(outcome.status, crate::grounding::GroundingStatus::Ungroundable | crate::grounding::GroundingStatus::Ambiguous);
        if failed && style_ids.contains(&outcome.requirement_id) {
            let sev = match mode {
                StrictnessProfile::Strict => "blocker",
                _ => "warning",
            };
            directives.push(CombinedDirective {
                requirement_id: outcome.requirement_id.clone(),
                severity: sev.to_string(),
                directive: format!(
                    "Requirement '{}' is passive/ambiguous AND ungrounded — name the agent as a task ID, e.g. 'T1.2 SHALL resolve ...'",
                    outcome.requirement_id
                ),
            });
        }
    }
    directives
}

#[cfg(test)]
mod tests;
