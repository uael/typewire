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

1. `expand::analyze()` parses the `DeriveInput` + serde/typewire attributes into a `Schema`
2. `encode::generate_schema_and_section()` (from `typewire-schema`) emits the link-section record
3. `WasmCodegen` implements `Codegen` trait to emit wasm32-specific conversion code

## Supported Serde Attributes

Container: `rename_all`, `tag`, `content`, `untagged`, `transparent`, `default`, `deny_unknown_fields`, `from`, `try_from`, `into`

Variant: `rename`, `alias`, `rename_all`, `skip`, `skip_serializing`, `skip_deserializing`, `other`, `untagged`

Field: `rename`, `alias`, `skip`, `default`, `flatten`, `skip_serializing_if`, `with = "serde_bytes"`

## Typewire-Specific Attributes

- `#[typewire(base64)]` — `Vec<u8>` as base64 string
- `#[typewire(display)]` — use `Display`/`FromStr` for string conversion
- `#[typewire(lenient)]` — skip errors during `from_js` instead of propagating
- `#[diffable(atomic)]` — compare as a whole, no field-level patching
- `#[diffable(visit_transparent)]` — delegate `patch_js` to inner field
