# typewire-derive

Proc-macro crate providing `#[derive(Typewire)]`.

## Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | Entry point: `#[proc_macro_derive(Typewire, attributes(serde, diffable, typewire))]` |
| `src/attr.rs` | Attribute parsing: `ContainerAttrs`, `VariantAttrs`, `FieldAttrs` with sub-structs (`DiffableOpts`, `SkipOpts`, `EncodingOpts`) |
| `src/case.rs` | `RenameAll` enum — identifier case conversion (camelCase, snake_case, etc.) |
| `src/expand.rs` | `Codegen` trait + analysis: dispatches struct/enum/transparent/proxy to platform codegen |
| `src/wasm.rs` | `WasmCodegen` — generates `to_js`, `from_js`, `patch_js` for wasm32 |

## How It Works

1. `expand::analyze()` parses the `DeriveInput` + serde/diffable/typewire attributes into a `Schema`
2. `encode::generate_schema_and_section()` (from `typewire-schema`) emits the link-section record (gated behind `schemas` feature)
3. `WasmCodegen` implements `Codegen` trait to emit wasm32-specific conversion code

## Features

| Feature | What it enables |
|---------|----------------|
| `schemas` | Emits link-section records for TypeScript codegen (opt-in, propagated from `typewire/schemas`) |

## Attributes

All attributes below work under `#[serde(...)]`, `#[diffable(...)]`, or `#[typewire(...)]`. The `typewire` namespace is a superset — users can use it exclusively without needing `#[serde]` or `#[diffable]`.

**Container:** `rename_all`, `rename_all_fields`, `tag`, `content`, `untagged`, `transparent`, `default`, `deny_unknown_fields`, `from`, `try_from`, `into`, `atomic`, `visit_transparent`

**Variant:** `rename`, `alias`, `rename_all`, `skip`, `skip_serializing`, `skip_deserializing`, `other`, `untagged`

**Field:** `rename`, `alias`, `skip`, `default`, `flatten`, `skip_serializing_if`, `with = "serde_bytes"`, `base64`, `display`, `lenient`
