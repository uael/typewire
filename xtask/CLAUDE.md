# xtask

Dev automation. Single binary with subcommands for formatting, linting, documentation, testing, and benchmarking.

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
| `cargo xtask bench` | Run all benchmarks (size + perf) |
| `cargo xtask bench size` | Bundle size comparison (raw/gzip) |
| `cargo xtask bench perf` | Performance benchmarks (wasm, requires Node.js) |
| `cargo xtask bench --json` | Machine-readable JSON output (used by CI) |
| `cargo xtask bench check A.json B.json` | Compare two bench JSONs for size regressions (>1.5% = fail) |

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
