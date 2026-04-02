# xtask

Dev automation. Single binary with subcommands for formatting, linting, documentation, testing, and benchmarking.

## Commands

| Command | What it does |
|---------|-------------|
| `cargo xtask fmt` | `cargo +nightly fmt --all` |
| `cargo xtask fmt --check` | Check-only formatting |
| `cargo xtask lint` | 4 clippy passes (all feature combos + wasm32) + fmt check |
| `cargo xtask lint --fix` | Auto-fix mode |
| `cargo xtask doc` | Build docs per-crate with correct feature sets |
| `cargo xtask test` | All test suites |
| `cargo xtask test unit` | Native tests + schema roundtrips (`--features typescript`) |
| `cargo xtask test wasm` | wasm32 tests via `wasm-bindgen-test` |
| `cargo xtask test e2e` | Build wasm → typegen (strips section) → snapshot diff → assert stripped → tsc → node |
| `cargo xtask bench` | Run all benchmarks (size + perf) |
| `cargo xtask bench size` | Bundle size comparison (raw/gzip) |
| `cargo xtask bench perf` | Performance benchmarks (wasm, requires Node.js) |
| `cargo xtask bench --json` | Machine-readable JSON output (used by CI) |
| `cargo xtask bench check A.json B.json` | Compare two bench JSONs for size regressions (>1.5% = fail) |

## Lint Passes

The lint command runs separate clippy invocations because `typewire-schema`'s `encode` and `decode` features are mutually exclusive. Package-prefixed features (`typewire/uuid`, etc.) let us check optional type impls without extra passes:

1. Workspace + `typewire` optional type features (covers `typewire-derive`, `typewire-schema[encode]`, all type impls)
2. `typewire-schema --features typescript` (decode path + tests)
3. `typewire --no-default-features --features cli` (codegen binary)
4. `typewire + todo-app --target wasm32` + optional type features
