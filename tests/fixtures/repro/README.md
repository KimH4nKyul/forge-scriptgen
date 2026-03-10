# Repro Fixture

This fixture preserves the complex Solidity sources and expected generated scripts used to validate `forge-scriptgen` against non-trivial constructor signatures.

Contents:
- `src/`: Solidity contracts used as test input
- `expected/`: expected generated `.s.sol` output
- `scripts/reproduce_complex_cli_demo.sh`: end-to-end reproduction script

Run:

```bash
./scripts/reproduce_complex_cli_demo.sh
```

The script prints the source contracts, generates scripts with `forge-scriptgen`, prints the generated output, and verifies both files against `tests/fixtures/repro/expected/`.
