# Changelog

## 2026-03-10

### Summary
This update moves the project toward the roadmap in `FUTURE_WORK.md`, with immediate emphasis on handling complex Solidity code more reliably and preparing the CLI for future parser backends.

### Parser Architecture
- Introduced a parser abstraction layer in [src/main.rs](./src/main.rs).
- Added `ParserBackend` and `ContractParser` so contract discovery no longer depends directly on a single hard-coded parsing function.
- Implemented `StringWalkerParser` as the current backend and routed discovery through `discover_contracts_with_parser(...)`.
- Added CLI support for selecting a parser backend with `--parser string-walker`.

Why this matters:
- It creates a clean seam for a future AST-based backend without forcing a large rewrite today.
- It also makes side-by-side parser comparison possible later.

### Complex Solidity Handling
- Strengthened coverage for real-world Solidity patterns that are common in production codebases:
  - `abstract contract`
  - inheritance-based constructor modifiers
  - `struct` types in constructor parameters
  - function-typed constructor parameters
  - multi-line constructor declarations
  - `error` declarations
  - inline `assembly` blocks
- Added a dedicated parser regression test to confirm the current string-walking parser can still extract the correct constructor metadata from those shapes.

### Constructor Argument Input
- Improved `--args` parsing so JSON object items can now represent raw Solidity literals.
- Supported forms:
  - `{"raw":"Config({owner: msg.sender, limits: [1, 2]})"}`
  - `{"solidity":"callback"}`
- Preserved existing behavior for booleans, numbers, arrays, and standard JSON strings.
- Rejected `null` explicitly because it is not a valid constructor literal in this CLI flow.

Why this matters:
- Complex constructors often need raw Solidity expressions instead of JSON-only serialized values.
- This closes a practical gap that showed up while testing struct and function-typed arguments.

### CLI and Help Output
- Updated help text to document:
  - the new `--parser` option
  - raw literal usage for `--args`
- Refreshed the `Examples:` section so the live `--help` output now shows:
  - `forge-scriptgen --parser string-walker --list`
  - a complex constructor example using `{"raw":"..."}` literals

Why this matters:
- The installed CLI help now reflects the actual supported workflows instead of only the earlier minimal examples.
- Users can discover the complex-constructor path directly from `forge-scriptgen --help` without opening the README.

### Test Coverage Added
- Added unit tests in [src/main.rs](./src/main.rs) for:
  - parser backend parsing
  - raw Solidity literal support in `--args`
  - complex contract parsing regression
- Added an integration test in [tests/cli.rs](./tests/cli.rs) that:
  - lists a contract with a complex constructor signature
  - generates a deployment script from mixed raw and JSON string arguments
  - verifies the final constructor call in the generated script

### Reproducible CLI fixtures
- Added persistent Solidity fixtures under [tests/fixtures/repro/src](./tests/fixtures/repro/src) so the manual complex-contract validation can be reproduced without relying on temporary files.
- Added expected generated script outputs under [tests/fixtures/repro/expected](./tests/fixtures/repro/expected).
- Added an end-to-end reproduction script at [scripts/reproduce_complex_cli_demo.sh](./scripts/reproduce_complex_cli_demo.sh).
- Added fixture usage notes in [README.md](./tests/fixtures/repro/README.md).

What the reproduction script does:
- prints the complex Solidity source fixtures
- runs `forge-scriptgen` against them
- prints the generated `.s.sol` files
- compares generated output against the expected fixtures with `diff`

Why this matters:
- It turns the ad-hoc manual validation into a repeatable regression check.
- It preserves concrete examples of the Solidity patterns the current parser is expected to support.

### Validation
- Ran `cargo fmt`
- Ran `cargo test`
- Ran `./scripts/reproduce_complex_cli_demo.sh`
- Reinstalled the global CLI with `cargo install --path . --force`
- Verified the live `forge-scriptgen --help` output after reinstall
- Result: all tests passed, including unit and CLI integration tests

### Example
```bash
cargo run -- \
  --parser string-walker \
  --args '[{"raw":"Config({owner: msg.sender, limits: [1, 2]})"},{"raw":"callback"},"primary"]' \
  --private-key 0x9999 \
  AdvancedCounter
```
