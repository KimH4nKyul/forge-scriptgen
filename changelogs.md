# Changelogs

## 2026-03-10

### Summary
This update advances the repository along the direction proposed in `FUTURE_WORK.md`, with an immediate focus on handling more complex Solidity code safely and preparing the parser for future backend upgrades.

### Parser Architecture
- Introduced a parser abstraction layer in [src/main.rs](/Users/kimh4nkyul/Projects/forge-scriptgen/src/main.rs).
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

### Test Coverage Added
- Added unit tests in [src/main.rs](/Users/kimh4nkyul/Projects/forge-scriptgen/src/main.rs) for:
  - parser backend parsing
  - raw Solidity literal support in `--args`
  - complex contract parsing regression
- Added an integration test in [tests/cli.rs](/Users/kimh4nkyul/Projects/forge-scriptgen/tests/cli.rs) that:
  - lists a contract with a complex constructor signature
  - generates a deployment script from mixed raw and JSON string arguments
  - verifies the final constructor call in the generated script

### Validation
- Ran `cargo fmt`
- Ran `cargo test`
- Result: all tests passed, including unit and CLI integration tests

### Example
```bash
cargo run -- \
  --parser string-walker \
  --args '[{"raw":"Config({owner: msg.sender, limits: [1, 2]})"},{"raw":"callback"},"primary"]' \
  --private-key 0x9999 \
  AdvancedCounter
```

### Files Changed
- [src/main.rs](/Users/kimh4nkyul/Projects/forge-scriptgen/src/main.rs)
- [tests/cli.rs](/Users/kimh4nkyul/Projects/forge-scriptgen/tests/cli.rs)
- [AGENTS.md](/Users/kimh4nkyul/Projects/forge-scriptgen/AGENTS.md)
