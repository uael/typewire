# typewire

Main library crate. Provides the `Typewire` trait and all built-in implementations.

typewire is a derive-based **cross-language type bridging** framework. WASM/TypeScript is the first supported target; Kotlin and Swift are planned.

## Key Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | `Typewire` trait, primitive/compound type impls, platform-specific helpers |
| `src/error.rs` | `Error` enum with context wrapping |
| `src/bin.rs` | CLI binary (feature `cli`): extract schemas from binaries, generate declarations |

## Typewire Trait

```rust
pub trait Typewire: Sized {
  type Ident: Copy + 'static;             // compile-time schema identity
  const IDENT: Self::Ident;
  fn or_default() -> Option<Self>;        // implicit default for absent fields

  // Platform-specific methods (e.g. wasm32: to_js/from_js/patch_js)
  // are cfg-gated and generated per-platform by the derive macro.
}
```

On non-wasm targets, only `Ident`/`IDENT` and `or_default` exist. On wasm32, `to_js`/`from_js`/`from_js_lenient`/`patch_js` methods are added. Future platforms will add their own cfg-gated methods.

## Features

| Feature | What it enables |
|---------|----------------|
| `derive` (default) | Re-exports `#[derive(Typewire)]` from `typewire-derive` |
| `schemas` | Embeds schema records in link sections (opt-in, propagates to `typewire-derive`) |
| `uuid`, `chrono`, `url`, `bytes`, `indexmap`, `fractional_index`, `base64`, `serde_json` | `Typewire` impls for these types |
| `cli` | Binary target + `typewire-schema/typescript` for codegen |

## Platform Helpers

### wasm32

The `wasm` module (cfg-gated) provides:
- `is_nullish`, `as_safe_f64` — JS value inspection
- `as_u32`, `isize_as_u32` — lossless index casts on wasm32 (usize is 32-bit)

## Tests

- `tests/schema_integration.rs` — Verifies derive produces correct binary idents
- `tests/compile_fail/` — Compile-time error tests (trybuild)
- `tests/wasm_*.rs` — wasm32 tests for all type impls, run via `cargo xtask test wasm`
