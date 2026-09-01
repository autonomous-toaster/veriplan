## Context

`veriplan check` currently supports `--format json` for JSON output, but the JSON drops the `convertibility_report` (blockers, warnings, infos) unless `--verbose` is passed. The human output is compact by default (blockers only) but `--verbose` balloons to 13KB, dominated by 43 repetitive "task not referenced" warnings. AI agents consuming this output waste context on noise.

## Goals / Non-Goals

**Goals:**
- Add `--json` shorthand flag for `--format json`
- Add `--compact` flag for AI-optimized output (minified, short keys, counts)
- Fix JSON to always include `convertibility_report`
- Add warning/info summary counts to default human output
- Keep `--verbose` output unchanged (backward compat)

**Non-Goals:**
- No changes to the verification engine or IR types
- No new output formats beyond JSON (no YAML, XML, etc.)
- No changes to exit code behavior

## Decisions

### 1. `--json` as shorthand for `--format json`
`--json` sets format to "json". If both `--json` and `--format` are given, `--json` wins (last flag wins in clap).

### 2. `--compact` as a general modifier
`--compact` changes the output schema to be AI-optimized:
- Short field names (`plan` not `plan_name`, `ok` not `convertible`, `blk` not `blockers`)
- Warnings/info as counts, not arrays
- Minified (no extra whitespace)
- TOON-friendly for `| toon` piping

Only meaningful with `--json`. Ignored for human output (human default is already compact).

### 3. Full JSON schema (unchanged except fix)
The existing `format_json` keeps its schema but always includes `convertibility_report`. This is the "full data" mode for scripts and humans reading JSON.

### 4. Compact JSON schema (new)
```json
{"plan":"name","ok":false,"skip":"reason",
 "blk":[{"req":"...","loc":"file:line","err":"...","fix":"..."}],
 "warn":43,"info":8}
```

### 5. Human default output
Add warning/info summary counts after blockers:
```
  7 blocker(s): [full detail]
  43 warning(s): tasks not referenced by any SHALL requirement
  8 info(s)
```

## Risks / Trade-offs

- [Backward compat] `--format json` without `--verbose` will now include `convertibility_report`. Consumers that relied on its absence will break. Mitigation: this is a bug fix, not a breaking change — the report should always have been included.
- [Schema coupling] Compact JSON uses short keys. If TOON format evolves, the schema may need updates. Mitigation: short keys are a convention, not a dependency on TOON.
