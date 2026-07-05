## Context

Veriplan's convertibility check currently validates task structure (T4.1) and requirement references (T4.2) using exact task ID matching. It cannot detect when a requirement uses vague natural language like "the migration step" instead of the explicit task ID "T2.1". The `groundcontrol` crate provides a rule-based grounding engine that fills this gap.

groundcontrol is a sibling crate in the same GitHub org (`autonomous-toaster/groundcontrol`). It is not published on crates.io — integration is via git dependency. Its core is ~600 LOC across three modules: types, RuleGrounder, and parse helpers. The RuleGrounder is stateless and uses keyword matching + positional heuristics — no ML dependency.

## Goals / Non-Goals

**Goals:**

- Add groundcontrol as a git dependency in Cargo.toml
- Build a Signature from PlanIR tasks (task IDs + descriptions as aliases)
- Ground each requirement's SHALL statement against the Signature using groundcontrol's RuleGrounder
- Populate the existing `PatternUngrounded` ConstraintCategory variant
- Surface grounding results in the convertibility report (blocker for ungroundable/ambiguous by default)
- Support strictness profile downgrading (Moderate → warning, Lax → info)
- Fold grounding rules into `veriplan bootstrap` config generation

**Non-Goals:**

- Replace veriplan's existing temporal classification (T4.4) — grounding is complementary, not a replacement
- Add ML-based grounding — the rule-based approach is sufficient for structured OpenSpec specs
- Modify groundcontrol itself — integration is consumer-side only
- Support stdin/single-file mode grounding — scoped to OpenSpec change directories

## Decisions

### Decision: Library dependency over subprocess

groundcontrol is consumed as a Rust library (git dependency), not invoked as a subprocess CLI.

**Rationale:** The RuleGrounder is stateless and its API is a single function call (`ground(nl, sig) → GroundingResult`). A subprocess would add serialization overhead, error handling complexity, and a runtime dependency on the groundcontrol binary being installed. Library integration is simpler, faster, and type-safe.

### Decision: Signature built from PlanIR, not re-parsed

A `Signature::from_planir(plan: &PlanIR)` builder converts veriplan's existing PlanIR tasks into groundcontrol's Signature type, rather than re-parsing tasks.md with groundcontrol's parser.

**Rationale:** Veriplan already has PlanIR fully populated after parsing. Re-parsing would duplicate work and risk inconsistency. The mapping is straightforward: each PlanIR Task → ConstantDef (name = "T{id}", aliases from description), plus the 6 fixed predicate definitions.

### Decision: Grounding integrated into convertibility check, not separate phase

The grounding check runs as part of the existing convertibility pipeline (between T4.2 and T4.4), not as a separate CLI phase.

**Rationale:** The convertibility report already has blockers/warnings/rephrase_directives — grounding results map directly to these slots. A separate phase would add pipeline complexity (new CLI flags, new report sections, new state) for what's essentially one more check in the existing sequence.

### Decision: Ambiguous = blocker by default, downgradable via strictness

Ambiguous grounding (confidence < 0.8) produces a blocker by default. The existing strictness profile (Strict/Moderate/Lax) controls whether ambiguous is downgraded to warning.

**Rationale:** If the grounder can't confidently map NL to task IDs, the spec is ambiguous and the user should fix it. Strictness profiles provide an escape hatch for quick iteration.

### Decision: New module at src/grounding/

A new `src/grounding/` module wraps groundcontrol's API, hiding the dependency behind a veriplan-internal interface.

**Rationale:** Keeps the dependency change isolated. If groundcontrol's API changes, only `src/grounding/` needs updating. The rest of veriplan interacts with `CheckItem` results, not groundcontrol types directly.

## Risks / Trade-offs

- **[Dependency on unpublished crate]** groundcontrol is a git dependency from a sibling repo. If the repo is unavailable or the API changes incompatibly, veriplan won't build. → Mitigation: pin to a specific commit SHA in Cargo.toml; both repos are in the same GitHub org.
- **[False positives]** The rule-based grounder may flag valid NL as ungroundable if it uses uncommon phrasing. → Mitigation: strictness profiles allow downgrading; users can also add aliases to task descriptions.
- **[Performance]** Grounding runs for every requirement in every check. For plans with hundreds of requirements, this adds latency. → Mitigation: the RuleGrounder is O(n*m) where n=constants, m=predicates — cheap. No IO or ML inference.
- **[Overlap with T4.2]** Both T4.2 (exact ID check) and grounding (fuzzy check) validate task references. → Mitigation: they're complementary — T4.2 catches missing IDs, grounding catches vague NL. Both run; neither is redundant.
