# Architecture

## Overview

`forge-scriptgen` is a small Rust CLI that turns Solidity contracts in a Foundry project into starter deployment scripts.

Current flow:
1. Parse CLI arguments in `src/main.rs`
2. Resolve the contracts directory and output directory
3. Discover Solidity files under `src/**/*.sol`
4. Parse contracts and constructor metadata through a parser backend
5. Select the target contract
6. Collect constructor arguments from JSON or interactive input
7. Render and write `script/<ContractName>.s.sol`

## Main components

### CLI layer

The CLI currently lives in `src/main.rs`. It is responsible for:
- argument parsing
- user-facing errors
- interactive prompts
- file discovery
- script rendering and file writing

### Parser abstraction

The project already has a parser boundary:
- `ParserBackend`
- `ContractParser`
- `StringWalkerParser`

This keeps the CLI flow independent from a specific parsing implementation and allows an AST-based backend to be added later.

### Current parser

`StringWalkerParser` is a source-walking parser that relies on:
- comment stripping
- delimiter tracking
- string tracking
- contract and constructor keyword scanning

It is intentionally lightweight, but it is expected to preserve behavior across a range of Solidity patterns covered by tests and fixtures.

## Data flow

Important structures:
- `Options`: parsed CLI inputs
- `ContractInfo`: selected contract metadata used for generation
- `ConstructorParam`: constructor parameter text plus detected display name
- `ParsedContract`: parser output before file-level metadata is attached

`discover_contracts_with_parser(...)` is the main bridge between file discovery and parser output.

## Testing strategy

There are three layers of validation:

1. Unit tests in `src/main.rs`
   - parser helpers
   - constructor extraction
   - argument parsing behavior

2. CLI integration tests in `tests/cli.rs`
   - end-to-end CLI behavior against temporary projects

3. Reproducible fixture demo in `tests/fixtures/repro/` and `scripts/reproduce_complex_cli_demo.sh`
   - complex Solidity source examples
   - committed expected generated output

## Design constraints

Current priorities:
- generated script correctness over aggressive automation
- explicit handling of complex constructor literals
- minimal friction for Foundry users
- backward-compatible room for a future AST parser

Known limitation:
- the current parser is not semantic AST parsing yet, so literal correctness still depends on user input for complex types
