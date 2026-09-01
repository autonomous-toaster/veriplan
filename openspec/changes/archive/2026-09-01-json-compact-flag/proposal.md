## Why

`veriplan check` output is too verbose for AI agent context (13KB with `--verbose`, JSON drops convertibility report). Agents need compact, actionable output they can parse quickly without wasting tokens on repetitive warnings.

## What Changes

- Add `--json` flag as shorthand for `--format json`
- Add `--compact` flag to produce AI-optimized output (minified, short keys, warnings/info as counts)
- Fix JSON output to always include `convertibility_report` (not gated behind `--verbose`)
- Add warning/info summary counts to default human output
- Collapse repetitive warnings to summary in verbose mode

## Capabilities

### New Capabilities
- `cli-output-format`: Structured output modes for `veriplan check` — full JSON, compact JSON, and compact human

### Modified Capabilities
None — no existing spec-level behavior changes

## Impact

- `src/main.rs`: Add `--json` and `--compact` CLI flags
- `src/annotator/mod.rs`: New `format_json_compact()` function, fix `format_json` gate, add summary counts to `format_human`
- `src/cli.rs`: Update `check_all_changes` to support compact mode
