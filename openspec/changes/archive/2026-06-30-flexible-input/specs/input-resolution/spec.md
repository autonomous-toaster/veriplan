Task Reference

| T ID | Description |
|------|-------------|
| T1.1 | Add InputSource enum and InputResolver to CLI |
| T1.2 | Implement single-file input detection and parsing |
| T1.3 | Implement stdin input detection and parsing |
| T1.4 | Implement loose-directory input detection |
| T1.5 | Wire InputResolver into check and visualize subcommands |

### Requirement: Input resolution detects OpenSpec directories

T1.1 SHALL define an `InputSource` enum BEFORE T1.5 dispatches to the parser.
The enum SHALL have variants for OpenSpec directories, loose directories, single files, and stdin.

### Requirement: Single-file input detection

T1.2 SHALL detect when the CLI argument is a `.md` file path BEFORE T1.5 dispatches to the parser.
A `.md` file path SHALL be resolved relative to the current working directory.
If the file does not exist, veriplan SHALL exit with an error showing the path.

### Requirement: Stdin input detection

T1.3 SHALL accept `-` or `--stdin` as a CLI argument BEFORE T1.5 dispatches to the parser.
When stdin mode is active, veriplan SHALL read the full content from stdin and parse it as markdown.
The source location label for stdin-parsed items SHALL be `<stdin>`.

### Requirement: Loose-directory input detection

T1.4 SHALL detect when the CLI argument is a directory that contains `tasks.md` or `specs/` but not the full OpenSpec layout BEFORE T1.5 dispatches to the parser.
A directory with only `tasks.md` SHALL produce a PlanIR with empty requirements and an INFO item "no specs found".
A directory with only `specs/` SHALL produce a PlanIR with empty tasks and an INFO item "no tasks found".
A directory with neither SHALL cause veriplan to exit with error "no verifiable content found in directory".
