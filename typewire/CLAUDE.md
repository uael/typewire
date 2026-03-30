# typewire

Main library crate. Provides the `Typewire` trait and all built-in implementations.

## Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | `Typewire` trait, primitive/compound type impls, `patch_js` helpers, wasm utilities |
| `src/error.rs` | `Error` enum with context wrapping |
| `src/bin.rs` | CLI binary (feature `cli`): extract schemas from binaries, generate TypeScript |

## Typewire Trait

```rust
pub trait Typewire: Sized {
  type Ident: Copy + 'static;
  const IDENT: Self::Ident;
  fn or_default() -> Option<Self>;        // implicit default for absent fields
  fn to_js(&self) -> JsValue;             // wasm32 only
  fn from_js(value: JsValue) -> Result<Self, Error>;
  fn from_js_lenient(value: JsValue, field: &str) -> Result<Self, Error>;
  fn patch_js(&self, old: &JsValue, set: impl FnOnce(JsValue));
}
```

All `to_js`/`from_js`/`patch_js` methods are `#[cfg(target_arch = "wasm32")]`. On non-wasm targets, only `Ident`/`IDENT` and `or_default` exist.

## Features

| Feature | What it enables |
|---------|----------------|
| `derive` (default) | Re-exports `#[derive(Typewire)]` from `typewire-derive` |
| `uuid`, `chrono`, `url`, `bytes`, `indexmap`, `fractional_index`, `base64`, `serde_json` | `Typewire` impls for these types |
| `cli` | Binary target + `typewire-schema/typescript` for codegen |

## wasm32 Helpers

The `wasm` module (cfg-gated) provides:
- `is_nullish`, `as_safe_f64` — JS value inspection
- `as_u32`, `isize_as_u32` — lossless index casts on wasm32 (usize is 32-bit)

## Tests

- `tests/schema_integration.rs` — Verifies derive produces correct binary idents
- `tests/compile_fail/` — Compile-time error tests (trybuild)
- `tests/wasm_*.rs` — wasm32 tests for all type impls, run via `cargo xtask test wasm`
