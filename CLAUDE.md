# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```sh
cargo build                          # Build all crates
cargo xtask test                     # Run all tests (unit + wasm + e2e)
cargo xtask test unit                # Native unit + integration + schema roundtrips
cargo xtask test wasm                # wasm32 tests (needs wasm-bindgen-cli)
cargo xtask test e2e                 # Build wasm → typegen → tsc → node
cargo xtask fmt                      # Format code (requires nightly rustfmt)
cargo xtask fmt --check              # Check formatting
cargo xtask lint                     # Clippy + format check (-D warnings)
cargo xtask lint --fix               # Auto-fix clippy + formatting
cargo doc --no-deps --all            # Build documentation (-D warnings in CI)
cargo deny check                     # License/advisory/ban auditing
```

Requires **stable** Rust (pinned in `rust-toolchain`). Formatting uses nightly rustfmt via `cargo +nightly fmt`.

## Architecture

typewire provides bidirectional Rust↔JavaScript type conversion for wasm32 targets, with compile-time schema embedding and TypeScript declaration generation.

### Pipeline

```
#[derive(Typewire)]  →  encode (link section)  →  decode  →  TypeScript .d.ts
     (derive)              (typewire-schema)       (CLI)       (codegen)
```

1. `#[derive(Typewire)]` analyzes types and emits `to_js`/`from_js`/`patch_js` + schema records in link sections
2. The `typewire` CLI extracts schema records from compiled binaries and generates TypeScript declarations
3. The generated `.d.ts` types match the wire format of the Rust types

### Workspace Crates

| Crate | Role |
|-------|------|
| `typewire/` | Main library: `Typewire` trait, primitive/compound impls, error types, CLI binary |
| `typewire-derive/` | Proc-macro: `#[derive(Typewire)]` with full serde attribute support |
| `typewire-schema/` | Schema metadata: coded binary format, encode/decode, TypeScript emitter |
| `xtask/` | Dev automation: fmt, lint (8 clippy passes), test (unit/wasm/e2e) |
| `examples/todo-app/` | End-to-end example: wasm cdylib with TypeScript type-checking |

### Feature Matrix

`typewire-schema` has mutually exclusive feature sets:
- `encode` — derive-time (uses `syn`), enabled by `typewire-derive`
- `decode` — runtime codegen (uses `thiserror`), enabled by `typewire[cli]`
- `typescript` — implies `decode`, adds TypeScript emitter

These cannot be combined in a single compilation. The xtask lint command handles this by running separate clippy passes per feature combination.

## Releasing

Releases are automated with [release-plz](https://release-plz.dev/). On every push to `main`, the GitHub Action opens (or updates) a release PR that bumps versions and updates `CHANGELOG.md`. Merging that PR publishes to crates.io and creates a GitHub release with a `v<version>` tag.

- Config: `release-plz.toml`
- Workflow: `.github/workflows/release-plz.yml`
- Only `typewire`, `typewire-derive`, and `typewire-schema` are published; `xtask` and examples are excluded
- `typewire` owns the changelog and git tags (`v{{ version }}`); `typewire-derive` and `typewire-schema` are bumped in lockstep without their own changelog or tags

### Commit Message Format

Use [Conventional Commits](https://www.conventionalcommits.org/) — release-plz uses them to determine version bumps and generate changelog entries.

```
<type>[optional scope]: <description>

[optional body]
```

| Type | Bump | Example |
|------|------|---------|
| `fix:` | patch | `fix: handle edge case in schema validation` |
| `feat:` | minor | `feat: add new derive attribute` |
| `feat!:` / `BREAKING CHANGE:` | major | `feat!: rename core trait` |
| `docs:`, `ci:`, `chore:`, `refactor:`, `test:` | none | `ci: add deny check to CI` |

## Code Style

- 2-space indentation, max 100 columns
- Imports: reordered, grouped by `StdExternalCrate`, granularity `Crate`
- Clippy: `pedantic` + `nursery` at warn; `dbg_macro`, `allow_attributes`, `missing_safety_doc`, `undocumented_unsafe_blocks` denied
- No `#[allow]` — use `#[expect(lint, reason = "...")]` when suppression is necessary
- Dual-licensed: Apache-2.0 OR MIT
