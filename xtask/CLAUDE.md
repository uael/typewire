# xtask

Dev automation. Single binary with subcommands for formatting, linting, documentation, and testing.

## Commands

| Command | What it does |
|---------|-------------|
| `cargo xtask fmt` | `cargo +nightly fmt --all` |
| `cargo xtask fmt --check` | Check-only formatting |
| `cargo xtask lint` | 9 clippy passes (all feature combos + wasm32) + fmt check |
| `cargo xtask lint --fix` | Auto-fix mode |
| `cargo xtask doc` | Build docs per-crate with correct feature sets |
| `cargo xtask test` | All test suites |
| `cargo xtask test unit` | Native tests + schema roundtrips (`--features typescript`) |
| `cargo xtask test wasm` | wasm32 tests via `wasm-bindgen-test` |
| `cargo xtask test e2e` | Build wasm → typegen (strips section) → snapshot diff → assert stripped → tsc → node |
| `cargo xtask test --coverage` | All tests + per-crate line coverage via `cargo-llvm-cov` |
| `cargo xtask test unit --coverage` | Unit tests only with coverage |
| `cargo xtask test --coverage --coverage-output path.json` | Write per-crate coverage JSON to a custom path |

## Lint Passes

The lint command runs separate clippy invocations because `typewire-schema`'s `encode` and `decode` features are mutually exclusive:

1. Workspace default features
2. `typewire-schema` no features
3. `typewire-schema --features encode`
4. `typewire-schema --features typescript`
5. `typewire --no-default-features`
6. `typewire` all type features
7. `typewire --no-default-features --features cli`
8. `typewire-derive`
9. `typewire + todo-app --target wasm32-unknown-unknown`

## Coverage

The `--coverage` flag uses `cargo-llvm-cov` (LLVM instrument-coverage) to collect line coverage for native unit tests. Wasm and e2e tests are excluded because LLVM instrument-coverage does not support the wasm32 target.

Coverage is collected via a two-pass approach:
1. `cargo llvm-cov --no-report --all` runs workspace tests under coverage instrumentation
2. `cargo llvm-cov --no-report -p typewire-schema --features typescript` runs the typescript-feature tests separately (mutually exclusive features)
3. `cargo llvm-cov report --json --package <crate>` generates per-crate JSON reports

Output:
- Human-readable summary printed to stdout
- Machine-readable JSON written to `target/coverage.json` (or `--coverage-output <path>`)

CI stores per-crate percentages as git notes on `refs/notes/coverage` and enforces a max 1.0 percentage point regression threshold.
