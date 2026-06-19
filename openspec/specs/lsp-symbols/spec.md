# lsp-symbols

## Purpose

Provides document symbol hierarchies for tasks.md (phases containing tasks) and spec.md (requirements containing scenarios), enabling editor outline views and symbol navigation.

## Task Reference

| T ID | Description |
|------|-------------|
| T6.1 | Implement document symbols for tasks.md |
| T6.2 | Implement document symbols for spec.md |

## Requirements

### Requirement: Document symbols for tasks.md

T6.1 SHALL ALWAYS implement `textDocument/documentSymbol` for `tasks.md` files, returning a hierarchical symbol tree where:

- Each phase is a `DocumentSymbol` with kind `SymbolKind::Namespace` containing child symbols
- Each task is a `DocumentSymbol` with kind `SymbolKind::Function`, with its checked/unchecked status in the detail field
- Phase names use the `[concurrent]` tag indicator when applicable

#### Scenario: tasks.md shows phases containing tasks

- **WHEN** T6.1 is invoked on tasks.md with tasks grouped under "## Detect Changes" and "## Parse"
- **THEN** T6.1 SHALL return two top-level symbols: "Detect Changes" containing its task children, and "Parse" containing its task children

### Requirement: Document symbols for spec.md

T6.2 SHALL ALWAYS implement `textDocument/documentSymbol` for spec files, returning a hierarchical symbol tree where:

- Each requirement heading is a `DocumentSymbol` with kind `SymbolKind::Interface`
- Each scenario is a child `DocumentSymbol` with kind `SymbolKind::Event`
- The detail field SHOW the temporal category classification (e.g., "SequentialOrder", "Global") when available

#### Scenario: spec.md shows requirements containing scenarios

- **WHEN** T6.2 is invoked on a spec.md with two requirements, each having scenarios
- **THEN** T6.2 SHALL return the requirements as top-level symbols, each containing their scenario children
