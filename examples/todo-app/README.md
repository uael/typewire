# todo-app

End-to-end example and practical guide for the typewire pipeline:
**derive → compile → typegen → type-check → runtime test**.

This example showcases the full typewire API surface -- transparent newtypes,
all enum tagging modes, `HashMap`, `base64` encoding, proxy types with
validation, tuple structs, untagged enums, `rename_all`, `skip_serializing_if`,
`default`, and more.

## Quick start

```sh
cargo xtask test e2e
```

## Manual steps

```sh
# 1. Build the wasm module
cargo build -p todo-app --target wasm32-unknown-unknown --release

# 2. Generate TypeScript type declarations
cargo run -p typewire --features cli -- \
  target/wasm32-unknown-unknown/release/todo_app.wasm \
  -o examples/todo-app/types.d.ts

# 3. Generate JS/TS bindings
wasm-bindgen target/wasm32-unknown-unknown/release/todo_app.wasm \
  --out-dir examples/todo-app/pkg --target nodejs

# 4. Install deps, type-check, and run the test
cd examples/todo-app && npm install && npx tsc --noEmit && npx tsx test.ts
```

## Table of contents

- [Quick start](#quick-start)
- [The pipeline](#the-pipeline)
- [Derive attributes](#derive-attributes)
- [Type mapping](#type-mapping)
- [Error handling](#error-handling)
- [patch_js](#patch_js)
- [Generic types](#generic-types)
- [CLI usage](#cli-usage)
- [Feature flags](#feature-flags)
- [Common patterns](#common-patterns)

## Quick start

### 1. Add dependencies

```toml
# Cargo.toml
[package]
name = "my-app"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
typewire = { version = "0.0.2", features = ["schemas"] }
wasm-bindgen = "0.2"
js-sys = "0.3"
```

The `schemas` feature is required for TypeScript declaration generation. Without it, only the `Typewire` trait impl is generated (no link section overhead).

### 2. Define types

```rust
#![cfg(target_arch = "wasm32")]

use typewire::Typewire;
use wasm_bindgen::prelude::*;

#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct User {
  pub user_name: String,
  pub email: Option<String>,
  pub age: u32,
}

#[wasm_bindgen(unchecked_return_type = "User")]
pub fn create_user(
  #[wasm_bindgen(unchecked_param_type = "User")] value: User,
) -> Result<User, typewire::Error> {
  Ok(value)
}
```

### 3. Build to WebAssembly

```sh
cargo build --target wasm32-unknown-unknown --release
```

### 4. Generate TypeScript declarations

```sh
# Install the CLI (one-time)
cargo install typewire --features cli

# Extract schemas and generate .d.ts
typewire target/wasm32-unknown-unknown/release/my_app.wasm -o types.d.ts
```

This produces:

```typescript
export interface User {
  userName: string;
  email: string | null;
  age: number;
}
```

### 5. Generate JS bindings

```sh
wasm-bindgen target/wasm32-unknown-unknown/release/my_app.wasm \
  --out-dir pkg --target nodejs
```

### 6. Use from TypeScript

```typescript
import type { User } from "./types.d.ts";
const { create_user } = require("./pkg/my_app.js");

const user: User = create_user({
  userName: "alice",
  email: null,
  age: 30,
});
```

## The pipeline

```text
#[derive(Typewire)]  -->  encode (link section)  -->  decode  -->  TypeScript .d.ts
     (derive)              (typewire-schema)         (CLI)        (codegen)
```

### Step 1: Derive

`#[derive(Typewire)]` does two things:

1. **Generates platform-specific conversion methods** -- on `wasm32`, this means `to_js()`, `from_js()`, and `patch_js()` methods that convert between Rust types and `JsValue`.

2. **Embeds schema records** (when `schemas` feature is enabled) -- type metadata is serialized into a `typewire_schemas` link section in the compiled binary.

The generated code uses `wasm-bindgen`'s `JsValue` and `js-sys` primitives. typewire does not replace `wasm-bindgen` -- it builds on top of it to support richer type shapes.

### Step 2: Extract

The `typewire` CLI reads the `typewire_schemas` section from the compiled binary, deserializes the schema records, and generates typed declarations for the target language.

### Step 3: Strip

By default, the CLI strips the `typewire_schemas` section from the binary after extraction. This means the schema metadata does not ship to production. Use `--no-strip` to keep it.

### Step 4: Use

The generated `.d.ts` file provides TypeScript types that match the wire format of your Rust types. Import them alongside the `wasm-bindgen` generated JS bindings.

## Derive attributes

All attributes work under the `#[typewire(...)]` namespace. For types that also derive serde's `Serialize`/`Deserialize`, typewire reads `#[serde(...)]` attributes too -- you don't need to duplicate them.

### Container attributes

Applied to structs and enums.

| Attribute | Applies to | Description |
|-----------|-----------|-------------|
| `rename_all = "..."` | struct, enum | Rename all fields/variants: `camelCase`, `snake_case`, `PascalCase`, `SCREAMING_SNAKE_CASE`, `kebab-case`, `SCREAMING-KEBAB-CASE`, `lowercase`, `UPPERCASE` |
| `rename_all_fields = "..."` | enum | Rename fields inside all enum variants |
| `tag = "..."` | enum | Internally tagged: `{ "tag": "Variant", ...fields }` |
| `tag = "...", content = "..."` | enum | Adjacently tagged: `{ "tag": "Variant", "content": payload }` |
| `untagged` | enum | No tag -- variant inferred from content shape |
| `transparent` | struct | Newtype wrapper -- wire format is the inner type |
| `default` | struct | Missing fields use `Default::default()` per-field |
| `deny_unknown_fields` | struct | Reject unrecognized keys during `from_js` |
| `from = "Type"` | struct, enum | Deserialize via `From<Type>` proxy |
| `try_from = "Type"` | struct, enum | Deserialize via `TryFrom<Type>` proxy |
| `into = "Type"` | struct, enum | Serialize via `Into<Type>` proxy |

#### Enum tagging examples

**External (default):**

```rust
#[derive(Typewire)]
enum Shape {
  Circle { radius: f64 },
  Rect { width: f64, height: f64 },
}
```

```typescript
export type Shape =
  | { "Circle": { radius: number } }
  | { "Rect": { width: number; height: number } };
```

**Internal:**

```rust
#[derive(Typewire)]
#[typewire(tag = "kind")]
enum Shape {
  Circle { radius: f64 },
  Rect { width: f64, height: f64 },
}
```

```typescript
export type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Rect"; width: number; height: number };
```

**Adjacent:**

```rust
#[derive(Typewire)]
#[typewire(tag = "type", content = "data")]
enum Command {
  Add(Item),
  Remove { id: u32 },
  Clear,
}
```

```typescript
export type Command =
  | { type: "Add"; data: Item }
  | { type: "Remove"; data: { id: number } }
  | { type: "Clear" };
```

**Untagged:**

```rust
#[derive(Typewire)]
#[typewire(untagged)]
enum Input {
  Num(f64),
  Text(String),
}
```

```typescript
export type Input =
  | number
  | string;
```

### Variant attributes

Applied to individual enum variants.

| Attribute | Description |
|-----------|-------------|
| `rename = "..."` | Override this variant's wire name |
| `alias = "..."` | Accept alternative names during `from_js` (repeatable) |
| `rename_all = "..."` | Rename fields within this variant |
| `skip` | Skip this variant entirely |
| `skip_serializing` | Omit from `to_js` output |
| `skip_deserializing` | Reject during `from_js` |
| `other` | Catch-all for unknown variant names (unit variant only) |
| `untagged` | Per-variant untagged within a tagged enum |

### Field attributes

Applied to struct fields and enum variant fields.

| Attribute | Description |
|-----------|-------------|
| `rename = "..."` | Override this field's wire name |
| `alias = "..."` | Accept alternative names during `from_js` (repeatable) |
| `skip` | Skip this field entirely |
| `skip_serializing` | Omit from `to_js` output |
| `skip_deserializing` | Ignore during `from_js` |
| `default` | Use `Default::default()` when field is absent |
| `default = "path"` | Call function at `path` when field is absent |
| `flatten` | Inline nested struct fields into the parent object |
| `skip_serializing_if = "path"` | Omit field if predicate returns true |
| `base64` | Encode `Vec<u8>` as base64 string (requires `base64` feature) |
| `display` | Use `Display`/`FromStr` for conversion instead of native |
| `with = "serde_bytes"` | Encode `Vec<u8>` as `Uint8Array` |
| `lenient` | Skip invalid elements instead of failing (for `Vec`, `Option`, maps) |

## Type mapping

### Primitive types

| Rust type | TypeScript type | JS representation |
|-----------|----------------|-------------------|
| `bool` | `boolean` | Boolean |
| `u8`, `u16`, `u32` | `number` | Number (exact) |
| `i8`, `i16`, `i32` | `number` | Number (exact) |
| `u64`, `u128`, `usize` | `number` | Number (lossy) or BigInt |
| `i64`, `i128`, `isize` | `number` | Number (lossy) or BigInt |
| `f32`, `f64` | `number` | Number |
| `String`, `Cow<str>` | `string` | String |
| `char` | `string` | String (single char) |
| `()` | `null` | null |

### Compound types

| Rust type | TypeScript type | JS representation |
|-----------|----------------|-------------------|
| `Option<T>` | `T \| null` | Value or null |
| `Vec<T>` | `T[]` | Array |
| `[T; N]` | `T[]` | Array (length checked on `from_js`) |
| `HashMap<K, V>` | `Record<K, V>` | Plain object |
| `BTreeMap<K, V>` | `Record<K, V>` | Plain object |
| `(A, B)` | `[A, B]` | Array (tuple) |
| `(A, B, C, ...)` | `[A, B, C, ...]` | Array (up to 12 elements) |
| `Box<T>` | `T` | Same as inner |
| `Arc<T>`, `Rc<T>` | `T` | Same as inner |

### Feature-gated types

| Rust type | Feature | TypeScript type | JS representation |
|-----------|---------|----------------|-------------------|
| `uuid::Uuid` | `uuid` | `string` | UUID string |
| `chrono::DateTime<Tz>` | `chrono` | `string` | RFC 3339 string |
| `url::Url` | `url` | `string` | URL string |
| `bytes::Bytes` | `bytes` | `Uint8ClampedArray` | Typed array |
| `IndexMap<K, V>` | `indexmap` | `Record<K, V>` | Plain object |
| `IndexSet<T>` | `indexmap` | `T[]` | Array |
| `serde_json::Value` | `serde_json` | `any` | Any JS value |
| `FractionalIndex` | `fractional_index` | `string` | Encoded string |

### Derived types

| Rust pattern | TypeScript output |
|-------------|-------------------|
| Named struct | `export interface Name { ... }` |
| Tuple struct | `export type Name = [T, U, ...]` |
| Unit struct | `export type Name = null` |
| Transparent newtype | `export type Name = T` |
| All-unit enum | `export type Name = "A" \| "B" \| "C"` |
| Enum with data | `export type Name = \| ... \| ...` |

### Field encoding overrides

| Attribute | Rust type | TypeScript type | Wire format |
|-----------|-----------|----------------|-------------|
| `#[typewire(base64)]` | `Vec<u8>` | `string` | Base64 string |
| `#[typewire(display)]` | any `Display + FromStr` | `string` | String |
| `#[serde(with = "serde_bytes")]` | `Vec<u8>` | `Uint8Array` | TypedArray |

## Error handling

### The Error type

`typewire::Error` represents conversion errors. On `wasm32`, it implements `From<Error> for JsValue`, converting to a JS `Error` object.

```rust
pub enum Error {
  UnexpectedType { expected: &'static str },
  MissingField { field: &'static str },
  UnknownVariant { variant: String },
  InvalidValue { message: String },
  OutOfRange,
  NoMatchingVariant,
  Custom(String),
  Context { context: String, source: Box<Error> },
}
```

### Context chaining

Errors automatically gain context as they bubble up through nested types:

```rust
let err = Error::MissingField { field: "name" }
  .in_context("inner")
  .in_context("Outer");
// "in `Outer`: in `inner`: missing field `name`"
```

The derive macro adds context automatically -- struct-level `from_js` wraps any field error with the type name.

### Returning errors from wasm functions

Use `Result<T, typewire::Error>` as the return type of `#[wasm_bindgen]` functions:

```rust
#[wasm_bindgen(unchecked_return_type = "User")]
pub fn create_user(
  #[wasm_bindgen(unchecked_param_type = "User")] value: User,
) -> Result<User, typewire::Error> {
  Ok(value)
}
```

On the JS side, invalid inputs throw a JS `Error` with a descriptive message:

```typescript
try {
  create_user({ id: "not_a_number" } as any);
} catch (e) {
  // Error: in `User`: expected number
}
```

### Proxy type validation errors

Types with `#[typewire(try_from = "...")]` propagate `TryFrom` errors as `Error::Custom`:

```rust
#[derive(Typewire)]
#[typewire(try_from = "String", into = "String")]
pub struct NonEmptyString(String);

impl TryFrom<String> for NonEmptyString {
  type Error = &'static str;
  fn try_from(s: String) -> Result<Self, Self::Error> {
    if s.is_empty() { Err("string must not be empty") } else { Ok(Self(s)) }
  }
}
```

## patch_js

`patch_js` performs structural diffing -- it only touches the JS properties that actually changed. This preserves JS object identity and minimizes DOM-triggering mutations in frameworks.

### How it works

```rust
// On wasm32, every Typewire type has:
fn patch_js(&self, old: &JsValue, set: impl FnOnce(JsValue));
```

- **Structs**: recurse into each field, calling `patch_js` on changed fields. The outer JS object keeps the same reference.
- **Collections**: use LCS-based diffing to emit minimal splice operations instead of replacing the entire array.
- **Primitives and all-unit enums**: compare old and new values; call `set` only if different.
- **Atomic types** (`#[diffable(atomic)]`): replace the entire value instead of patching fields.
- **Transparent wrappers**: delegate to the inner type's `patch_js`.

### Diffable attributes

| Attribute | Applies to | Description |
|-----------|-----------|-------------|
| `#[diffable(atomic)]` | struct, enum | Replace the whole value instead of patching fields |
| `#[diffable(visit_transparent)]` | transparent struct | Delegate patching to the inner type |

### Example

```rust
#[derive(Clone, Typewire)]
struct AppState {
  users: Vec<User>,
  count: u32,
}

// Only the changed field is updated in the JS object.
// The `users` array uses LCS diffing for minimal splices.
new_state.patch_js(&old_js, |new_js| { /* full replace fallback */ });
```

## Generic types

Generic types work with `#[derive(Typewire)]`. The derive adds a `Typewire` bound to each type parameter automatically.

### Basic generic struct

```rust
#[derive(Debug, PartialEq, Clone, Typewire)]
struct Wrapper<T: Clone> {
  value: T,
}
```

Works with any `T` that implements `Typewire`:

```rust
let w = Wrapper { value: 42u32 };
let js = w.to_js();
let back = Wrapper::<u32>::from_js(js).unwrap();
```

### Multiple type parameters

```rust
#[derive(Typewire)]
struct Pair<A, B> {
  left: A,
  right: B,
}
```

### Where clauses

```rust
#[derive(Typewire)]
struct Container<T>
where
  T: Clone + std::fmt::Debug,
{
  inner: T,
}
```

### Generic enums

```rust
#[derive(Typewire)]
#[typewire(tag = "type")]
enum Response<T> {
  Value { data: T },
  Empty,
}
```

### Const generics

```rust
#[derive(Typewire)]
struct FixedArray<const N: usize> {
  items: [u32; N],
}
```

### TypeScript output for generics

The TypeScript codegen produces monomorphized types. Each concrete instantiation of a generic type produces a separate TypeScript definition based on its Rust type name. Generic type parameters are not preserved in the TypeScript output.

## CLI usage

```
typewire [OPTIONS] <BINARY>

Arguments:
  <BINARY>  Path to the compiled binary (WASM, ELF, or Mach-O)

Options:
  -l, --lang <LANG>      Target language [default: typescript] [possible values: typescript]
  -o, --output <OUTPUT>  Output file (stdout if omitted)
      --no-strip         Keep the typewire_schemas section in the binary after extraction
  -h, --help             Print help
```

### Install

```sh
cargo install typewire --features cli
```

### Typical workflow

```sh
# Build your wasm crate
cargo build -p my-app --target wasm32-unknown-unknown --release

# Generate TypeScript declarations (strips schema section by default)
typewire target/wasm32-unknown-unknown/release/my_app.wasm -o types.d.ts

# Generate JS bindings with wasm-bindgen
wasm-bindgen target/wasm32-unknown-unknown/release/my_app.wasm \
  --out-dir pkg --target nodejs
```

### Output to stdout

Omit `-o` to print to stdout:

```sh
typewire target/wasm32-unknown-unknown/release/my_app.wasm
```

### Keep the schema section

By default, the CLI strips the `typewire_schemas` section from the wasm binary after extraction. Use `--no-strip` to keep it (useful during development):

```sh
typewire my_app.wasm -o types.d.ts --no-strip
```

## Feature flags

### typewire crate

| Feature | Default | Description |
|---------|---------|-------------|
| `derive` | yes | Re-exports `#[derive(Typewire)]` from `typewire-derive` |
| `schemas` | no | Embeds schema records in link sections for codegen. Required for TypeScript declaration generation. |
| `uuid` | no | `Typewire` impl for `uuid::Uuid` (serialized as string) |
| `chrono` | no | `Typewire` impl for `chrono::DateTime` (serialized as RFC 3339 string) |
| `url` | no | `Typewire` impl for `url::Url` (serialized as string) |
| `bytes` | no | `Typewire` impl for `bytes::Bytes` (serialized as `Uint8ClampedArray`) |
| `indexmap` | no | `Typewire` impls for `IndexMap` (as object) and `IndexSet` (as array) |
| `base64` | no | Enables `base64_encode`/`base64_decode` helpers and `#[typewire(base64)]` field attribute |
| `serde_json` | no | `Typewire` impl for `serde_json::Value` (serialized as `any`) |
| `fractional_index` | no | `Typewire` impl for `fractional_index::FractionalIndex` (serialized as string) |
| `cli` | no | Binary target for schema extraction + declaration generation |

### schemas feature

The `schemas` feature controls whether `#[derive(Typewire)]` embeds schema records in the compiled binary. Without it:

- The `Typewire` trait impl is still generated (with `to_js`/`from_js`/`patch_js`)
- No link section is emitted
- The `typewire` CLI cannot extract schemas

Enable `schemas` in your wasm cdylib crate:

```toml
[dependencies]
typewire = { version = "0.0.2", features = ["schemas"] }
```

You typically only need `schemas` in your final binary crate, not in library crates.

## Common patterns

### Transparent newtypes

Use `#[typewire(transparent)]` to create type-safe wrappers that serialize as their inner type:

```rust
#[derive(Clone, PartialEq, Eq, Hash, Typewire)]
#[typewire(transparent)]
pub struct UserId(String);

#[derive(Clone, Typewire)]
#[typewire(transparent)]
pub struct Timestamp(f64);
```

TypeScript output:

```typescript
export type UserId = string;
export type Timestamp = number;
```

This gives you type safety in Rust while keeping the wire format simple. The TypeScript type alias provides documentation.

### Proxy types (from/into)

For types with validation or complex construction, use proxy types to control serialization:

**Validated input (try_from + into):**

```rust
#[derive(Clone, Typewire)]
#[typewire(try_from = "String", into = "String")]
pub struct Email(String);

impl TryFrom<String> for Email {
  type Error = &'static str;
  fn try_from(s: String) -> Result<Self, Self::Error> {
    if s.contains('@') { Ok(Self(s)) } else { Err("invalid email") }
  }
}

impl From<Email> for String {
  fn from(e: Email) -> Self { e.0 }
}
```

TypeScript sees `export type Email = string;`. The validation happens in `from_js`.

**Bidirectional proxy (from + into):**

```rust
#[derive(Typewire)]
pub struct WireFormat { pub x: u32, pub y: u32 }

#[derive(Typewire)]
#[typewire(from = "WireFormat", into = "WireFormat")]
pub struct Point { pub x: u32, pub y: u32 }

impl From<WireFormat> for Point { /* ... */ }
impl From<Point> for WireFormat { /* ... */ }
```

### Optional fields

`Option<T>` has an implicit default of `None` via `or_default()`. This means optional fields don't require `#[typewire(default)]` to be absent-tolerant at runtime.

However, in the TypeScript schema, `Option<T>` fields without `default` are emitted as required (`field: T | null`). Add `#[typewire(default)]` to make them optional in the TypeScript type:

```rust
#[derive(Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct User {
  pub name: String,
  // Required in TS: `email: string | null`
  pub email: Option<String>,
  // Optional in TS: `bio?: string | null`
  #[typewire(default)]
  pub bio: Option<String>,
}
```

### skip_serializing_if for cleaner output

Use `skip_serializing_if` to omit null/empty values from `to_js` output:

```rust
#[derive(Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct User {
  pub name: String,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub avatar_url: Option<String>,
}
```

When `avatar_url` is `None`, the field is omitted from the JS object entirely (the property will be `undefined`, not `null`).

### Container defaults for optional config

When all fields of a struct should be optional, use container-level `default`:

```rust
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase", default)]
pub struct Options {
  pub notify: bool,
  pub retries: u32,
  pub timeout_ms: u32,
}

impl Default for Options {
  fn default() -> Self {
    Self { notify: true, retries: 3, timeout_ms: 5000 }
  }
}
```

TypeScript output:

```typescript
export interface Options {
  notify?: boolean;
  retries?: number;
  timeoutMs?: number;
}
```

All fields are optional -- omitted fields use the Rust `Default` implementation's values.

### Importing types in wasm-bindgen

Use `#[wasm_bindgen(typescript_custom_section)]` to import your generated types into the wasm-bindgen `.d.ts`:

```rust
#[wasm_bindgen(typescript_custom_section)]
const TS_IMPORTS: &str = r#"import type { User, Message } from '../types.d.ts';"#;
```

Then use `unchecked_param_type` and `unchecked_return_type` on your exports:

```rust
#[wasm_bindgen(unchecked_return_type = "User")]
pub fn create_user(
  #[wasm_bindgen(unchecked_param_type = "User")] value: User,
) -> Result<User, typewire::Error> {
  Ok(value)
}
```

### HashMap as metadata

`HashMap<String, V>` maps to `Record<string, V>` in TypeScript:

```rust
#[derive(Typewire)]
pub struct User {
  pub name: String,
  pub metadata: std::collections::HashMap<String, String>,
}
```

```typescript
export interface User {
  name: string;
  metadata: Record<string, string>;
}
```

### Tuple structs for coordinates

Tuple structs map to TypeScript tuples:

```rust
#[derive(Typewire)]
pub struct Position(pub f64, pub f64);
```

```typescript
export type Position = [number, number];
```

### Recursive types

Typewire handles recursive types naturally:

```rust
#[derive(Typewire)]
pub struct Comment {
  pub text: String,
  pub replies: Vec<Comment>,
}
```

```typescript
export interface Comment {
  text: string;
  replies: Comment[];
}
```

### Lenient collection parsing

Use `#[typewire(lenient)]` on collection fields to skip invalid elements instead of failing the entire parse:

```rust
#[derive(Typewire)]
pub struct Feed {
  #[typewire(lenient)]
  pub items: Vec<Item>,
  #[typewire(lenient)]
  pub metadata: Option<MetaData>,
}
```

Invalid elements are silently skipped with a `log::warn!` message. This is useful for forward-compatibility when the data source might include items your schema doesn't yet handle.

### base64-encoded binary data

For `Vec<u8>` fields that should be transferred as base64 strings (smaller than JSON arrays of numbers):

```rust
#[derive(Typewire)]
pub struct Attachment {
  pub name: String,
  #[typewire(base64)]
  pub data: Vec<u8>,
}
```

Requires the `base64` feature. The TypeScript type is `string`, and the runtime encoding is base64.

### Display/FromStr fields

For types that implement `Display` and `FromStr`, use `#[typewire(display)]` to serialize them as strings:

```rust
#[derive(Typewire)]
#[typewire(transparent)]
pub struct MyId(u64);

impl std::fmt::Display for MyId { /* ... */ }
impl std::str::FromStr for MyId { /* ... */ }

#[derive(Typewire)]
pub struct Record {
  #[typewire(display)]
  pub id: MyId,
}
```

The field serializes as a string in JS, so `id` shows as type `string` in TypeScript.
