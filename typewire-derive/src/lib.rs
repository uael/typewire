//! Derive macro for the `Typewire` trait.
//!
//! This crate provides `#[derive(Typewire)]`, which generates platform-specific
//! conversion methods and compile-time schema records from Rust types.
//!
//! Most users should depend on [`typewire`](https://docs.rs/typewire) (which
//! re-exports this macro) rather than depending on `typewire-derive` directly.
//! See the [typewire crate docs](https://docs.rs/typewire) for supported
//! targets and the full pipeline overview.
//!
//! # Attribute reference
//!
//! Attributes can be placed under `#[serde(...)]`,
//! [`#[diffable(...)]`](https://docs.rs/difficient/latest/difficient/trait.Diffable.html), or
//! `#[typewire(...)]`. The `#[typewire(...)]` namespace is a superset — when
//! a type also derives [`Serialize`](https://docs.rs/serde/latest/serde/trait.Serialize.html)/[`Deserialize`](https://docs.rs/serde/latest/serde/trait.Deserialize.html),
//! typewire reads `#[serde]` attributes too so you don't need to duplicate them.
//!
//! ## Container attributes
//!
//! | Attribute | Effect |
//! |-----------|--------|
//! | `rename_all = "..."` | Rename all fields (e.g. `"camelCase"`, `"snake_case"`) |
//! | `rename_all_fields = "..."` | Rename fields in all enum variants |
//! | `tag = "..."` | Internally-tagged enum |
//! | `content = "..."` | Adjacent tagging (requires `tag`) |
//! | `untagged` | Untagged enum |
//! | `transparent` | Newtype wrapper — delegates to inner type |
//! | `default` | Per-field `Default::default()` fallback for missing fields |
//! | `deny_unknown_fields` | Error on unrecognized fields |
//! | `from = "Type"` / `try_from = "Type"` | Proxy deserialization |
//! | `into = "Type"` | Proxy serialization |
//!
//! ## Variant attributes
//!
//! | Attribute | Effect |
//! |-----------|--------|
//! | `rename = "..."` | Wire name for this variant |
//! | `alias = "..."` | Additional accepted names (repeatable) |
//! | `rename_all = "..."` | Rename fields within this variant |
//! | `skip` | Skip entirely |
//! | `skip_serializing` / `skip_deserializing` | Skip one direction |
//! | `other` | Catch-all for unknown variant names |
//! | `untagged` | Untagged within a tagged enum |
//!
//! ## Field attributes
//!
//! | Attribute | Effect |
//! |-----------|--------|
//! | `rename = "..."` | Wire name for this field |
//! | `alias = "..."` | Additional accepted names (repeatable) |
//! | `skip` | Skip entirely (field must impl `Default`) |
//! | `default` / `default = "path"` | Use `Default` or custom fn when absent |
//! | `flatten` | Flatten nested struct fields into parent |
//! | `skip_serializing_if = "path"` | Conditionally omit on serialization |
//! | `with = "serde_bytes"` | Use `serde_bytes` encoding |
//! | `base64` | Base64 encode/decode `Vec<u8>` fields |
//! | `display` | Use `Display`/`FromStr` for conversion |
//! | `lenient` | Skip invalid elements instead of failing |
//!
//! ## [`Diffable`](https://docs.rs/difficient/latest/difficient/trait.Diffable.html) attributes
//!
//! | Attribute | Effect |
//! |-----------|--------|
//! | `atomic` | Treat as opaque for patching (no field-level diff) |
//! | `visit_transparent` | Generate patching code for transparent types |
//!
//! # [Serde](https://docs.rs/serde) divergence
//!
//! `#[serde(default)]` on a **container** uses per-field `Default::default()`
//! fallbacks, **not** the container's `Default` impl. This is intentional —
//! the derive generates independent field fallbacks rather than constructing
//! a default instance and overwriting present fields.

extern crate proc_macro;

mod attr;
mod case;
mod expand;
mod wasm;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Derive macro for the `Typewire` trait.
///
/// Generates platform-specific conversion methods (gated by `#[cfg]`) and
/// the compile-time `Ident`/`IDENT` schema identity. See the
/// [crate-level documentation](crate) for the full attribute reference.
///
/// For usage examples, see the [`typewire`](https://docs.rs/typewire) crate docs.
#[proc_macro_derive(Typewire, attributes(serde, diffable, typewire))]
pub fn derive_typewire(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  expand::expand::<wasm::WasmCodegen>(&input).into()
}
