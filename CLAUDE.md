# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build & Development Commands

```sh
cargo build                          # Build all crates
cargo xtask test                     # Run ALL tests (unit + wasm + e2e) — always use this
cargo xtask test --coverage          # Unit + wasm coverage (skips e2e)
cargo xtask test unit --coverage     # Coverage for unit tests only
cargo xtask test wasm --coverage     # Coverage for wasm tests only (nightly)
cargo xtask fmt                      # Format code (requires nightly rustfmt)
cargo xtask fmt --check              # Check formatting
cargo xtask lint                     # Clippy + format check (-D warnings)
cargo xtask lint --fix               # Auto-fix clippy + formatting
cargo xtask doc                      # Build documentation (-D warnings in CI)
cargo deny check                     # License/advisory/ban auditing
```

**Always run `cargo xtask test` (no subcommand) to validate changes.** Subcommands (`unit`, `wasm`, `e2e`) exist for targeted debugging only — never run them separately as a substitute for the full suite.

Requires **stable** Rust (pinned in `rust-toolchain`). Formatting uses nightly rustfmt via `cargo +nightly fmt`.

## Architecture

typewire is a derive-based cross-language type bridging framework. Define types once in Rust, get type-safe foreign-language bindings automatically. The derive macro generates platform-specific conversion methods, while the schema pipeline produces typed declarations for the target language.

Currently supported targets: **WebAssembly** (wasm32) with TypeScript codegen. **Kotlin** and **Swift** are planned — adding a new target means implementing the internal `Codegen` trait in `typewire-derive` (see `src/expand.rs`) and a new emitter module in `typewire-schema`.

### Pipeline

```
#[derive(Typewire)]  →  encode (link section)  →  decode  →  language-specific declarations
     (derive)              (typewire-schema)       (CLI)       (codegen)
```

1. `#[derive(Typewire)]` analyzes types and emits platform-gated conversion methods + schema records in link sections (when `schemas` feature is enabled)
2. The `typewire` CLI extracts schema records from compiled binaries, generates typed declarations, and strips the schema section
3. The generated declarations match the wire format of the Rust types

### Workspace Crates

| Crate | Role |
|-------|------|
| `typewire/` | Main library: `Typewire` trait, primitive/compound impls, error types, CLI binary |
| `typewire-derive/` | Proc-macro: `#[derive(Typewire)]` with platform-specific code generation |
| `typewire-schema/` | Schema metadata: coded binary format, encode/decode, language emitters (TypeScript, more planned) |
| `xtask/` | Dev automation: fmt, lint (9 clippy passes), test (unit/wasm/e2e) |
| `examples/todo-app/` | End-to-end example: wasm cdylib with TypeScript type-checking |

### Feature Matrix

`typewire-schema` has mutually exclusive feature sets:
- `encode` — derive-time (uses `syn`), enabled by `typewire-derive`
- `decode` — runtime codegen (uses `thiserror`), enabled by `typewire[cli]`
- `typescript` — implies `decode`, adds TypeScript emitter

These cannot be combined in a single compilation. The xtask lint command handles this by running separate clippy passes per feature combination.

`typewire` and `typewire-derive` share a `schemas` feature (opt-in):
- When enabled, `#[derive(Typewire)]` embeds schema records in a `typewire_schemas` link section
- When disabled, only the `Typewire` trait impl is generated (no link section overhead)
- Required for TypeScript codegen; the todo-app example enables it

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

## Documentation

- **Never leak implementation details in public docs.** Public rustdoc (`///`, `//!`) must not mention crate-internal types, traits, modules, or functions that are not part of the public API (e.g. private traits, internal module paths, codegen helpers). If a concept is only relevant to contributors, document it in `CLAUDE.md` or code comments (`//`), not in rustdoc.
- Use the README as the source of truth for what the project *is*. Crate-level docs should be consistent with it.

## Code Style

- 2-space indentation everywhere (including inside `macro_rules!` and `quote!` blocks), max 100 columns
- Imports: reordered, grouped by `StdExternalCrate`, granularity `Crate`
- Clippy: `pedantic` + `nursery` at warn; `dbg_macro`, `allow_attributes`, `missing_safety_doc`, `undocumented_unsafe_blocks` denied
- No `#[allow]` — use `#[expect(lint, reason = "...")]` when suppression is necessary
- Dual-licensed: Apache-2.0 OR MIT

## Workflow

- When working on a PR, **always keep the PR description synced** with the current state of the changes. Update the title and body whenever the scope or content of the PR evolves.

## Maintenance

When making changes that affect a crate's public API, features, file structure, or conventions, **always update the corresponding `CLAUDE.md`** files (root and per-crate) to keep them accurate.
