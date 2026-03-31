# typewire

[![CI](https://github.com/uael/typewire/workflows/CI/badge.svg)](https://github.com/uael/typewire/actions)
[![docs.rs](https://img.shields.io/docsrs/typewire)](https://docs.rs/typewire)
[![crates.io](https://img.shields.io/crates/v/typewire)](https://crates.io/crates/typewire)
[![license](https://img.shields.io/crates/l/typewire)](LICENSE-MIT)

> **Work in progress** -- not production ready. API and schema format may change.

**Derive-based cross-language type bridging for Rust.**

`#[derive(Typewire)]` generates bidirectional conversion methods and compile-time
schema records from your Rust types. Define types once in Rust, get type-safe
foreign-language bindings and declarations automatically.

Currently supported targets:

- **WebAssembly** (wasm32) -- generates `to_js`, `from_js`, `patch_js` via
  `wasm-bindgen`, with TypeScript `.d.ts` generation
- **Kotlin** and **Swift** -- planned

## Relationship with wasm-bindgen

Typewire does not replace `wasm-bindgen` -- it builds on top of it. Where
`wasm-bindgen` handles the low-level ABI boundary (function exports, memory
management, JS glue code), typewire adds support for richer type shapes that
`wasm-bindgen` alone cannot express: enums with data, tagged unions, nested
structs, optional fields, generic types, `HashMap`/`BTreeMap`, and more. The
generated code uses `wasm-bindgen`'s `JsValue` and `js-sys` primitives under
the hood.

## Quick look

```rust
use typewire::Typewire;

#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct Todo {
  pub id: u32,
  pub title: String,
  pub completed: bool,
  pub description: Option<String>,
  pub priority: Priority,
  pub tags: Vec<String>,
}

#[derive(Clone, Typewire)]
#[typewire(rename_all = "lowercase")]
pub enum Priority {
  Low,
  Medium,
  High,
}

#[derive(Clone, Typewire)]
#[typewire(tag = "type", content = "data")]
pub enum Command {
  Add(Todo),
  Toggle { id: u32 },
  Remove { id: u32 },
  SetPriority { id: u32, priority: Priority },
}
```

Build to wasm, run the CLI, and get TypeScript declarations:

```ts
export type Command =
  | { type: "Add"; data: Todo }
  | { type: "Toggle"; data: { id: number } }
  | { type: "Remove"; data: { id: number } }
  | { type: "SetPriority"; data: { id: number; priority: Priority } };

export type Priority = "low" | "medium" | "high";

export interface Todo {
  id: number;
  title: string;
  completed: boolean;
  description: string | null;
  priority: Priority;
  tags: string[];
}
```

## How it works

```text
#[derive(Typewire)]  -->  encode (link section)  -->  decode  -->  TypeScript .d.ts
     (derive)              (typewire-schema)         (CLI)        (codegen)
```

1. **Derive** -- `#[derive(Typewire)]` analyzes your types and generates
   target-specific conversion methods. With the `schemas` feature enabled,
   it also embeds schema records in a binary link section.

2. **Extract** -- The `typewire` CLI reads schema records from compiled
   binaries and generates typed declarations for the target language.

3. **Strip** -- The CLI strips the schema section from the binary so it
   doesn't ship to production.

## Features

| Feature | What it enables |
|---------|----------------|
| `derive` (default) | Re-exports `#[derive(Typewire)]` |
| `schemas` | Embeds schema records in link sections for codegen |
| `cli` | Binary target for schema extraction and declaration generation |
| `uuid` | `Typewire` impl for `uuid::Uuid` |
| `chrono` | `Typewire` impl for `chrono::DateTime`, `NaiveDate`, etc. |
| `url` | `Typewire` impl for `url::Url` |
| `indexmap` | `Typewire` impl for `IndexMap` and `IndexSet` |
| `bytes` | `Typewire` impl for `bytes::Bytes` |
| `base64` | Base64 encoding for `Vec<u8>` fields via `#[typewire(base64)]` |
| `serde_json` | `Typewire` impl for `serde_json::Value` |

## Serde compatibility

`#[typewire(...)]` supports the same attributes as serde:

- **Container**: `rename_all`, `tag`, `content`, `untagged`, `transparent`,
  `default`, `deny_unknown_fields`, `from`, `try_from`, `into`
- **Variant**: `rename`, `alias`, `skip`, `other`, `untagged`
- **Field**: `rename`, `alias`, `skip`, `default`, `flatten`,
  `skip_serializing_if`, `with = "serde_bytes"`, `base64`, `display`, `lenient`

When a type also derives `Serialize`/`Deserialize`, typewire reads `#[serde(...)]`
attributes too, so you don't need to duplicate them. But when only deriving
`Typewire`, prefer `#[typewire(...)]`.

## Efficient patching

`patch_js` performs structural diffing -- it only touches the JS properties that
actually changed. For collections, it uses LCS-based diffing to emit minimal
splice operations instead of replacing the entire array.

## CLI usage

```sh
# Install
cargo install typewire --features cli

# Generate TypeScript from a wasm binary
typewire target/wasm32-unknown-unknown/release/my_app.wasm -o types.d.ts

# The schema section is stripped automatically (use --no-strip to keep it)
```

## Workspace

| Crate | Role |
|-------|------|
| `typewire` | Main library: trait, primitive/compound impls, CLI binary |
| `typewire-derive` | Proc-macro: `#[derive(Typewire)]` |
| `typewire-schema` | Schema metadata: binary format, encode/decode, emitters |

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or
  <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or
  <http://opensource.org/licenses/MIT>)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
