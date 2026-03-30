# typewire-schema

Schema metadata crate. Defines the type metadata pipeline: encode → binary → decode → codegen.

## Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | `Schema` enum, `Field`, `Struct`, `Enum`, `Variant`, flags, `repr` module |
| `src/scalar.rs` | `Scalar` enum — 24 leaf type identifiers |
| `src/coded.rs` | Binary format: `#[repr(C, packed)]` types for link-section embedding |
| `src/encode.rs` | (feature `encode`) `Schema` → `TokenStream` for link-section records (`generate_schema_and_section(schema, emit_section)`) |
| `src/decode.rs` | (feature `decode`) Link-section bytes → `Vec<Schema>` with owned data |
| `src/typescript.rs` | (feature `typescript`) `impl IntoIterator<Item = &Schema>` → `.d.ts` string |

## Three-Stage Pipeline

```
syn::Schema  ──encode──▶  coded::Record<T>  ──decode──▶  Schema (owned)
 (derive)                  (link section)                  (codegen)
feature="encode"           always available              feature="decode"
```

## Features (Mutually Exclusive)

- `encode` — enables `syn`, `quote`, `proc-macro2` deps. Used by `typewire-derive` at compile time.
- `decode` — enables `thiserror`. Used at runtime for schema extraction.
- `typescript` — implies `decode`. Adds TypeScript emitter.

**`encode` and `decode` cannot be combined** in the same compilation — the `repr` module provides different type aliases depending on which is active (`syn::Type` vs `Box<Schema>`).

## coded Module

All types are `#[repr(C, packed)]`, `Copy`, const-constructible. Derives `zerocopy::IntoBytes` + `zerocopy::Immutable` for safe byte casting.

Key types: `Record<T>`, `Ident<N>`, `PrimitiveIdent`, `OptionIdent`, `SeqIdent`, `MapIdent`, `TupleIdent`, `Tag`, `FlatStruct`, `FlatEnum`, etc.

The `typewire_schemas` link section (Apple: `__DATA,typewire_schemas`) contains concatenated `Record<T>` entries.

## Tests

`tests/roundtrip.rs` — 43 tests gated on `feature = "typescript"`. Run via `cargo xtask test unit` (which passes `--features typescript`).
