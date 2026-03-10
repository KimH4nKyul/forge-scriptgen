# Contributing

Thanks for contributing to `forge-scriptgen`.

## Development setup

Install Rust, then work from the repository root.

```bash
cargo build
cargo test
```

For local CLI runs:

```bash
cargo run -- --help
cargo run -- --parser string-walker --list
```

## Recommended development loop

Run these before opening a pull request:

```bash
cargo fmt
cargo test
./scripts/reproduce_complex_cli_demo.sh
```

`cargo test` covers parser and CLI behavior. The reproduction script validates complex Solidity fixtures and compares generated scripts against committed expected output.

## Where to contribute

High-value contribution areas:
- Solidity parsing accuracy and edge cases
- new parser backends behind the existing abstraction
- constructor argument validation and UX
- safer secret-handling workflows
- generated script quality and template flexibility
- test fixtures for real-world Solidity patterns

## Parser changes

If you change parsing behavior:
- add or update unit tests in `src/main.rs`
- add or update CLI coverage in `tests/cli.rs` when user-facing behavior changes
- update `tests/fixtures/repro/` if the complex-case reproduction output changes

Prefer improving behavior through the parser abstraction instead of coupling more logic directly into the CLI path.

## Fixtures and expected output

Reproducible fixtures live under `tests/fixtures/repro/`.

- `src/` contains Solidity input files
- `expected/` contains expected generated `.s.sol` output

If a change intentionally modifies generated output, update the expected files and explain why in the pull request.

## Pull requests

A good PR should include:
- a short problem statement
- the implementation approach
- commands used for verification
- fixture or test updates for new Solidity edge cases

Keep changes scoped. Separate parser logic, documentation, and unrelated cleanup into different commits when practical.
