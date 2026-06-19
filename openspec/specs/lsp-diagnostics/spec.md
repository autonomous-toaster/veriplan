# lsp-diagnostics

## Purpose

Publishes real-time convertibility diagnostics to LSP clients on file save, mapping CheckItem severity levels to LSP DiagnosticSeverity and covering all files in an affected change.

## Task Reference

| T ID | Description |
|------|-------------|
| T3.1 | Implement did_save diagnostics handler |
| T3.4 | Publish diagnostics for all files in the affected change |
| T3.5 | Handle files outside any change |

## Requirements

### Requirement: Convertibility diagnostics on save

T3.1 SHALL ALWAYS publish `textDocument/publishDiagnostics` notifications mapping the result of `checker::check_convertibility()` to LSP Diagnostic structs on every `didSave` notification for files belonging to an OpenSpec change.

The mapping SHALL be:

- CheckItem.severity "blocker" → DiagnosticSeverity::ERROR
- CheckItem.severity "warning" → DiagnosticSeverity::WARNING
- CheckItem.severity "info" → DiagnosticSeverity::INFORMATION
- CheckItem.check string → Diagnostic.code as a String code
- CheckItem.detail → Diagnostic.message
- CheckItem.location ("file:line") → Diagnostic.range with 0-based line number (parsed from "file:42" → Position { line: 41, character: 0 }...line 41, character: 999)
- CheckItem.fix → stored in Diagnostic.data for use by code actions

T3.4 SHALL ALWAYS publish diagnostics for ALL files in the affected change, not just the saved file — because a change to one spec file can invalidate diagnostics in another.

#### Scenario: Blocker diagnostic on bad task reference

- **WHEN** T3.1 processes a saved spec file containing `T9.9 SHALL complete BEFORE T8.1` and no task T9.9 exists
- **THEN** T3.2 SHALL publish an ERROR diagnostic with code "bad_task_reference"

#### Scenario: Warning on missing RFC 2119 keyword

- **WHEN** T3.1 processes a saved spec file with a requirement heading but no SHALL in its paragraph
- **THEN** T3.2 SHALL publish a WARNING diagnostic with code "no_rfc2119_keyword"

#### Scenario: Diagnostics cleared on fix

- **WHEN** T3.1 processes a file that was previously erroring but now passes convertibility
- **THEN** T3.4 SHALL publish an empty diagnostics array for that file

### Requirement: Graceful handling outside changes

T3.5 SHALL ALWAYS publish an empty diagnostics array for files that cannot be resolved to any OpenSpec change, without attempting to reparse or fail.
