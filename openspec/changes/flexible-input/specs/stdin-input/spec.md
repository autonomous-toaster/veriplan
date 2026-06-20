Task Reference

| T ID | Description |
|------|-------------|
| T1.3 | Implement stdin input detection and parsing |
| T4.1 | Add parse_content() function that tries both parsers on any content |

### Requirement: Stdin reads from standard input

T1.3 SHALL read the entire standard input stream as a single string BEFORE parsing.
The `-` argument and `--stdin` flag SHALL be equivalent.
Stdin mode SHALL use `parse_content()` with the label `<stdin>` for source locations.
Stdin SHALL NOT require a temporary file — content SHALL be read directly from `std::io::stdin()`.
