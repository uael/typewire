# todo-app

End-to-end example and practical guide for the typewire pipeline:
**derive → compile → typegen → type-check → runtime test**.

This example showcases the full typewire API surface -- transparent newtypes,
all enum tagging modes, `HashMap`, `base64` encoding, proxy types with
validation, tuple structs, untagged enums, derive attributes, and more.

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

## Getting started (new project)

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
) -> User {
  value
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

## Learn more

For detailed documentation on all the topics below, see the
[API docs on docs.rs](https://docs.rs/typewire):

- **The pipeline** -- how `#[derive(Typewire)]` encodes schemas into link
  sections, the CLI extracts and generates declarations, and the schema
  section is stripped for production.
- **Derive attributes** -- container, variant, and field attributes under
  `#[typewire(...)]` (and serde compatibility).
- **Type mapping** -- how Rust primitives, compounds, and feature-gated
  types map to TypeScript.
- **Error handling** -- `typewire::Error`, context chaining, and returning
  errors from `#[wasm_bindgen]` functions.
- **`patch_js`** -- structural diffing, LCS-based collection patching,
  and `#[diffable]` attributes.
- **Generic types** -- structs, enums, const generics, and monomorphized
  TypeScript output.
- **CLI usage** -- `typewire [OPTIONS] <BINARY>`, install, and typical
  workflow.
- **Feature flags** -- `schemas`, `uuid`, `chrono`, `url`, `bytes`,
  `indexmap`, `base64`, `serde_json`, `fractional_index`, `cli`.
