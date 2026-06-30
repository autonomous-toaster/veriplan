# flexible-input

## Problem

veriplan only accepts OpenSpec change directories as input. The parser requires `openspec/changes/<name>/tasks.md` AND `specs/<capability>/spec.md` — two separate files in a strict directory layout. This blocks several real use cases:

1. **Single spec files** — "I have one spec.md, tell me if it's formalizable"
2. **Piped input** — CI pipelines that want to `cat spec.md | veriplan check --stdin`
3. **Ad-hoc directories** — a folder with tasks.md but no specs/, or vice versa
4. **Strictness mismatch** — the same SHALL statement is either a blocker or not depending on context, but there's no way to control this

Additionally, the checker has two correctness issues:

- MAY requirements are silently dropped (violates "never silently drop" principle)
- SHALL statements with temporal keywords but no task IDs are marked `NonFormalizable`, even though the pattern IS recognized — they're just not model-verifiable

## Proposal

Add flexible input modes and strictness profiles to veriplan:

1. **Single-file mode**: `veriplan check path/to/spec.md` or `path/to/tasks.md`
2. **Stdin mode**: `veriplan check --stdin` or `veriplan check -`
3. **Directory mode**: `veriplan check path/to/folder` (with tasks.md and/or specs/)
4. **Three-tier classification**: NonFormalizable / Pattern-formalizable-but-ungrounded / Fully-formalizable
5. **Strictness profiles**: `--strict` (default) / `--moderate` / `--lax` controlling severity, not visibility

OpenSpec remains the first-class input format. No changes to the OpenSpec parsing path.
