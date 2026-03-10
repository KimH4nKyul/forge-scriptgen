# forge-scriptgen

`forge-scriptgen` is a CLI that generates Foundry deployment scripts (`*.s.sol`) from Solidity source files in a Foundry project. It scans `src/**/*.sol`, finds deployable contracts, extracts constructor signatures, and emits ready-to-edit scripts under `script/`.

## Why this project should exist

Foundry is fast because it keeps the deployment loop close to the code. The missing piece in many teams is the repetitive glue work between contract discovery and script authoring. `forge-scriptgen` removes that manual step.

For global Foundry users, the value is straightforward:
- faster first deployment for new contracts
- fewer copy-paste mistakes in constructor calls
- better consistency across teams and repositories
- a CLI workflow that fits directly into existing Foundry projects

For Foundry maintainers, this project is useful because it explores a practical layer on top of Foundry without changing Foundry itself:
- it validates demand for script generation as a developer workflow
- it provides concrete fixtures for complex Solidity parsing cases
- it offers a small Rust codebase for experimenting with parser backends before deeper ecosystem integration

This project is worth using if you want a lightweight tool that turns contract source into a deployment-script starting point quickly, especially in repositories with many contracts, frequent constructor changes, or multiple contributors.

## What it does

- Discovers deployable contracts from `src/**/*.sol`
- Selects contracts by contract name, relative path, or file name
- Generates `script/<ContractName>.s.sol` by default
- Accepts constructor arguments through JSON or interactive prompts
- Supports raw Solidity literals for complex constructor values
- Lists detected contracts and constructor signatures with `--list`
- Prevents overwriting existing scripts unless `--force` is provided

## Solidity coverage

The current parser backend is `string-walker`. It is structured behind a parser abstraction so an AST backend can be added later without rewriting the CLI flow.

The current test and fixture coverage includes:
- abstract contracts
- inheritance-based constructor modifiers
- multiline constructors
- `struct` constructor arguments
- function-typed constructor arguments
- `error` declarations
- inline `assembly`
- misleading `contract` and `constructor` text inside comments and strings

## Installation

### Install Rust

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
```

### Clone the repository

```bash
git clone https://github.com/kimh4nkyul/forge-scriptgen.git
cd forge-scriptgen
```

### Install the binary

```bash
cargo install --path .
```

Or build and run locally:

```bash
cargo build --release
./target/release/forge-scriptgen --help
```

## Usage

Run the CLI from the root of a Foundry project.

### Show help

```bash
forge-scriptgen --help
```

### List detected contracts

```bash
forge-scriptgen --parser string-walker --list
```

### Generate a simple script

```bash
forge-scriptgen --args '["0xDeAd", 42]' --private-key 0xabc123 Counter
```

### Generate a script for a complex constructor

Use `{"raw":"..."}` or `{"solidity":"..."}` when a value must stay as a Solidity literal rather than a JSON string.

```bash
forge-scriptgen \
  --parser string-walker \
  --args '[{"raw":"Config({owner: msg.sender, limits: [1, 2, 3]})"},{"raw":"callback"},"primary",{"raw":"hex\"1234\""}]' \
  --private-key 0xabc123 \
  ComplexDeployment
```

### Use interactive prompts

```bash
forge-scriptgen Counter
```

### Overwrite an existing script

```bash
forge-scriptgen --force Counter
```

Generated scripts are written to `script/<ContractName>.s.sol` by default. Use `--output-dir` to change the destination. Import paths are computed relative to the generated script location.

## Reproducible complex-case demo

The repository includes a reproducible fixture for complex Solidity parsing and generation:

```bash
./scripts/reproduce_complex_cli_demo.sh
```

This script:
- prints the Solidity fixture contracts
- runs `forge-scriptgen` on them
- prints the generated `.s.sol` files
- verifies the generated output against committed expected files

Reference files:
- `tests/fixtures/repro/src/`
- `tests/fixtures/repro/expected/`
- `scripts/reproduce_complex_cli_demo.sh`

## Contributing

If you want to improve parsing coverage, CLI UX, fixtures, or generation quality, start here:

- [CONTRIBUTING.md](./CONTRIBUTING.md)
- [architecture.md](./docs/architecture.md)

The fastest contributor loop is:

```bash
cargo fmt
cargo test
./scripts/reproduce_complex_cli_demo.sh
```

## Security note

`--private-key` embeds the private key literal directly into the generated script. Do not commit generated scripts that contain real keys. Use disposable values for local testing, or extend the workflow to environment-based key loading before using it in production.

## Development

```bash
cargo fmt
cargo test
cargo run -- --help
cargo run -- --parser string-walker --list
```

After generation, continue with standard Foundry commands such as `forge test` or `forge script`.
