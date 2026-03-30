extern crate proc_macro;

mod attr;
mod case;
mod expand;
mod wasm;

use proc_macro::TokenStream;
use syn::{DeriveInput, parse_macro_input};

/// Derive macro for the `Typewire` trait.
///
/// Generates `to_js` and `from_js` implementations gated by
/// `#[cfg(target_arch = "wasm32")]`, respecting all serde attributes
/// that affect the wire shape.
///
/// # Example
///
/// ```ignore
/// use typewire::Typewire;
///
/// #[derive(Typewire)]
/// #[serde(rename_all = "camelCase")]
/// struct MyStruct {
///     field_name: String,
///     #[serde(skip)]
///     internal: u32,
/// }
/// ```
#[proc_macro_derive(Typewire, attributes(serde, diffable, typewire))]
pub fn derive_typewire(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as DeriveInput);
  expand::expand::<wasm::WasmCodegen>(&input).into()
}
