# xtask

Dev automation. Single binary with subcommands for formatting, linting, documentation, and testing.

## Key Files

| File | Purpose |
|------|---------|
| `src/main.rs` | CLI entry point, fmt/lint/doc/test commands |
| `src/coverage.rs` | Coverage collection, per-line LCOV merging, delta enforcement, git notes |

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
| `cargo xtask test e2e` | Build wasm -> typegen (strips section) -> snapshot diff -> assert stripped -> tsc -> node |
| `cargo xtask test --coverage` | Unit + wasm coverage via `cargo-llvm-cov` (skips e2e) |
| `cargo xtask test unit --coverage` | Unit tests only with coverage |
| `cargo xtask test wasm --coverage` | Wasm tests only with coverage (nightly) |
| `cargo xtask test --coverage --coverage-output path.json` | Write per-crate coverage JSON to a custom path |
| `cargo xtask coverage-delta target/coverage.json` | Compare coverage against parent commit's git note |

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

The `--coverage` flag uses `cargo-llvm-cov` (LLVM instrument-coverage) to collect line coverage. When run without a suite selector (`cargo xtask test --coverage`), both native unit tests and wasm tests are instrumented and combined into a single per-crate report. E2e tests are skipped in coverage mode (they are validated by the main CI workflow).

### Native coverage

1. `cargo llvm-cov --no-report --all` runs workspace tests under coverage instrumentation
2. `cargo llvm-cov --no-report -p typewire-schema --features typescript` runs the typescript-feature tests separately (mutually exclusive features)

### Wasm coverage

Uses `wasm-bindgen-test`'s experimental coverage support (requires nightly >= 1.87.0 and wasm-bindgen-test >= 0.3.57):
1. `cargo +nightly llvm-cov --no-report -p typewire --target wasm32-unknown-unknown` with env vars:
   - `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER=wasm-bindgen-test-runner`
   - `CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS="-Cinstrument-coverage -Zno-profiler-runtime -Clink-args=--no-gc-sections --cfg=wasm_bindgen_unstable_test_coverage"`

### Cross-target merging

The `typewire` crate compiles different code for native vs wasm32 (`#[cfg(target_arch = "wasm32")]`). LCOV reports (`cargo llvm-cov report --lcov`) are collected per-target and merged at the per-line level: for each source file, `DA:line,count` records are combined, taking the max execution count when the same line appears in both targets (covered if *either* target covered it). Lines present in only one report are taken as-is. Totals are recomputed from the merged per-line map.

### Reporting

`cargo llvm-cov report --lcov --package <crate>` generates per-crate LCOV reports from all accumulated profdata. These are parsed into per-line coverage data internally.

Output:
- Human-readable summary printed to stdout
- Machine-readable JSON written to `target/coverage.json` (or `--coverage-output <path>`)

CI stores per-crate percentages as git notes on `refs/notes/coverage` and enforces a max 1.0 percentage point regression threshold.
