use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Index};
use typewire_schema::{
  Enum as SchemaEnum, EnumFlags, Field as SchemaField, FieldDefault as SchemaFieldDefault,
  FieldFlags, FromBody, FromProxy, IntoProxy, Struct as SchemaStruct, StructFlags, StructShape,
  Tagging, Transparent, TypeShape, Variant as SchemaVariant, VariantFlags, VariantKind,
};

use crate::expand::Codegen;

// ---------------------------------------------------------------------------
// WasmCodegen
// ---------------------------------------------------------------------------

pub struct WasmCodegen;

impl Codegen for WasmCodegen {
  fn cfg_predicate() -> TokenStream {
    quote! { target_arch = "wasm32" }
  }

  fn expand_struct(s: &SchemaStruct) -> Vec<TokenStream> {
    let name_str = s.ident.to_string();
    let to_js_body = struct_to_js_body(s);
    let from_js_body = struct_from_js_body(s);
    let patch_js = struct_patch_js_fn(s);

    vec![
      quote! {
        fn to_js(&self) -> ::wasm_bindgen::JsValue {
          use ::wasm_bindgen::JsCast as _;
          #to_js_body
        }
      },
      quote! {
        fn from_js(value: ::wasm_bindgen::JsValue) -> ::core::result::Result<Self, ::typewire::Error> {
          use ::wasm_bindgen::JsCast as _;
          let __parse = move || -> ::core::result::Result<Self, ::typewire::Error> {
            #from_js_body
          };
          __parse().map_err(|e| e.in_context(#name_str))
        }
      },
      patch_js,
    ]
  }

  fn expand_transparent(t: &Transparent) -> Vec<TokenStream> {
    let access = t.field_ident.as_ref().map_or_else(
      || {
        let idx = Index::from(0);
        quote! { self.#idx }
      },
      |ident| quote! { self.#ident },
    );
    let construct = t
      .field_ident
      .as_ref()
      .map_or_else(|| quote! { Self(val) }, |ident| quote! { Self { #ident: val } });
    let ty = &t.field_ty;

    let patch_js = if t.atomic {
      quote! {
        fn patch_js(&self, old: &::wasm_bindgen::JsValue, set: impl FnOnce(::wasm_bindgen::JsValue)) {
          typewire::patch_js_atomic(self, old, set);
        }
      }
    } else {
      quote! {
        fn patch_js(&self, old: &::wasm_bindgen::JsValue, set: impl FnOnce(::wasm_bindgen::JsValue)) {
          <#ty as ::typewire::Typewire>::patch_js(&#access, old, set);
        }
      }
    };

    vec![
      quote! {
        fn to_js(&self) -> ::wasm_bindgen::JsValue {
          <#ty as ::typewire::Typewire>::to_js(&#access)
        }
      },
      quote! {
        fn from_js(value: ::wasm_bindgen::JsValue) -> ::core::result::Result<Self, ::typewire::Error> {
          let val = <#ty as ::typewire::Typewire>::from_js(value)?;
          Ok(#construct)
        }
      },
      patch_js,
    ]
  }

  fn expand_enum(e: &SchemaEnum) -> Vec<TokenStream> {
    let name_str = e.ident.to_string();
    let to_js_body = enum_to_js_body(e);
    let from_js_body = enum_from_js_body(e);
    let patch_js = enum_patch_js_fn(e);

    vec![
      quote! {
        fn to_js(&self) -> ::wasm_bindgen::JsValue {
          use ::wasm_bindgen::JsCast as _;
          #to_js_body
        }
      },
      quote! {
        fn from_js(value: ::wasm_bindgen::JsValue) -> ::core::result::Result<Self, ::typewire::Error> {
          use ::wasm_bindgen::JsCast as _;
          let __parse = move || -> ::core::result::Result<Self, ::typewire::Error> {
            #from_js_body
          };
          __parse().map_err(|e| e.in_context(#name_str))
        }
      },
      patch_js,
    ]
  }

  fn expand_into_proxy(p: &IntoProxy) -> Vec<TokenStream> {
    let into_ty = &p.into_ty;

    let from_js_body = match &p.from_body {
      FromBody::Proxy(proxy) => quote! {
        let proxy = <#proxy as ::typewire::Typewire>::from_js(value)?;
        Ok(<Self as ::core::convert::From<#proxy>>::from(proxy))
      },
      FromBody::TryProxy(proxy) => quote! {
        let proxy = <#proxy as ::typewire::Typewire>::from_js(value)?;
        <Self as ::core::convert::TryFrom<#proxy>>::try_from(proxy)
          .map_err(|e| ::typewire::Error::Custom(::std::string::ToString::to_string(&e)))
      },
      FromBody::Own(shape) => match shape {
        TypeShape::Struct(s) => struct_from_js_body(s),
        TypeShape::Enum(e) => enum_from_js_body(e),
      },
    };

    vec![
      quote! {
        fn to_js(&self) -> ::wasm_bindgen::JsValue {
          let proxy: #into_ty = ::core::convert::Into::into(self.clone());
          <#into_ty as ::typewire::Typewire>::to_js(&proxy)
        }
      },
      quote! {
        fn from_js(value: ::wasm_bindgen::JsValue) -> ::core::result::Result<Self, ::typewire::Error> {
          #from_js_body
        }
      },
      quote! {
        fn patch_js(&self, old: &::wasm_bindgen::JsValue, set: impl FnOnce(::wasm_bindgen::JsValue)) {
          let proxy: #into_ty = ::core::convert::Into::into(self.clone());
          <#into_ty as ::typewire::Typewire>::patch_js(&proxy, old, set);
        }
      },
    ]
  }

  fn expand_from_proxy(p: &FromProxy) -> Vec<TokenStream> {
    let name = &p.ident;
    let (_, ty_generics, _) = p.generics.split_for_impl();
    let proxy = &p.proxy;

    let from_body = if p.is_try {
      quote! {
        let proxy = <#proxy as ::typewire::Typewire>::from_js(value)?;
        <#name #ty_generics as ::core::convert::TryFrom<#proxy>>::try_from(proxy)
          .map_err(|e| ::typewire::Error::Custom(::std::string::ToString::to_string(&e)))
      }
    } else {
      quote! {
        let proxy = <#proxy as ::typewire::Typewire>::from_js(value)?;
        Ok(<#name #ty_generics as ::core::convert::From<#proxy>>::from(proxy))
      }
    };

    let to_js_body = match &p.own_shape {
      TypeShape::Struct(s) => struct_to_js_body(s),
      TypeShape::Enum(e) => enum_to_js_body(e),
    };

    let patch_js = match &p.own_shape {
      TypeShape::Struct(s) => struct_patch_js_fn(s),
      TypeShape::Enum(e) => enum_patch_js_fn(e),
    };

    vec![
      quote! {
        fn to_js(&self) -> ::wasm_bindgen::JsValue {
          use ::wasm_bindgen::JsCast as _;
          #to_js_body
        }
      },
      quote! {
        fn from_js(value: ::wasm_bindgen::JsValue) -> ::core::result::Result<Self, ::typewire::Error> {
          #from_body
        }
      },
      patch_js,
    ]
  }

  fn extra_impls(
    schema: &typewire_schema::Schema,
    ident: &syn::Ident,
    generics: &syn::Generics,
  ) -> TokenStream {
    let cfg = Self::cfg_predicate();
    let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

    let js_bindings = js_bindings_for_schema(schema);

    quote! {
      #[cfg(#cfg)]
      impl #impl_generics ::wasm_bindgen::describe::WasmDescribe
        for #ident #ty_generics #where_clause
      {
        fn describe() {
          <::wasm_bindgen::JsValue as ::wasm_bindgen::describe::WasmDescribe>::describe()
        }
      }

      #[cfg(#cfg)]
      impl #impl_generics ::wasm_bindgen::convert::IntoWasmAbi
        for #ident #ty_generics #where_clause
      {
        type Abi = <::wasm_bindgen::JsValue as ::wasm_bindgen::convert::IntoWasmAbi>::Abi;

        #[inline]
        fn into_abi(self) -> Self::Abi {
          ::wasm_bindgen::convert::IntoWasmAbi::into_abi(
            ::typewire::Typewire::to_js(&self),
          )
        }
      }

      #[cfg(#cfg)]
      impl #impl_generics ::wasm_bindgen::convert::FromWasmAbi
        for #ident #ty_generics #where_clause
      {
        type Abi = <::wasm_bindgen::JsValue as ::wasm_bindgen::convert::FromWasmAbi>::Abi;

        #[inline]
        unsafe fn from_abi(js: Self::Abi) -> Self {
          use ::wasm_bindgen::UnwrapThrowExt as _;
          let value = unsafe {
            <::wasm_bindgen::JsValue as ::wasm_bindgen::convert::FromWasmAbi>::from_abi(js)
          };
          ::typewire::Typewire::from_js(value).unwrap_throw()
        }
      }

      #[cfg(#cfg)]
      impl #impl_generics ::wasm_bindgen::convert::RefFromWasmAbi
        for #ident #ty_generics #where_clause
      {
        type Abi = <::wasm_bindgen::JsValue as ::wasm_bindgen::convert::RefFromWasmAbi>::Abi;
        type Anchor = ::std::boxed::Box<Self>;

        #[inline]
        unsafe fn ref_from_abi(js: Self::Abi) -> Self::Anchor {
          use ::wasm_bindgen::UnwrapThrowExt as _;
          let value = unsafe {
            <::wasm_bindgen::JsValue as ::wasm_bindgen::convert::FromWasmAbi>::from_abi(js)
          };
          ::std::boxed::Box::new(::typewire::Typewire::from_js(value).unwrap_throw())
        }
      }

      #[cfg(#cfg)]
      impl #impl_generics ::wasm_bindgen::convert::LongRefFromWasmAbi
        for #ident #ty_generics #where_clause
      {
        type Abi = <::wasm_bindgen::JsValue as ::wasm_bindgen::convert::LongRefFromWasmAbi>::Abi;
        type Anchor = Self;

        #[inline]
        unsafe fn long_ref_from_abi(js: Self::Abi) -> Self::Anchor {
          use ::wasm_bindgen::UnwrapThrowExt as _;
          let value = unsafe {
            <::wasm_bindgen::JsValue as ::wasm_bindgen::convert::FromWasmAbi>::from_abi(js)
          };
          ::typewire::Typewire::from_js(value).unwrap_throw()
        }
      }

      #[cfg(#cfg)]
      impl #impl_generics ::wasm_bindgen::convert::RefMutFromWasmAbi
        for #ident #ty_generics #where_clause
      {
        type Abi = <::wasm_bindgen::JsValue as ::wasm_bindgen::convert::RefFromWasmAbi>::Abi;
        type Anchor = ::std::boxed::Box<Self>;

        #[inline]
        unsafe fn ref_mut_from_abi(js: Self::Abi) -> Self::Anchor {
          use ::wasm_bindgen::UnwrapThrowExt as _;
          let value = unsafe {
            <::wasm_bindgen::JsValue as ::wasm_bindgen::convert::FromWasmAbi>::from_abi(js)
          };
          ::std::boxed::Box::new(::typewire::Typewire::from_js(value).unwrap_throw())
        }
      }

      #[cfg(#cfg)]
      impl #impl_generics ::wasm_bindgen::convert::OptionIntoWasmAbi
        for #ident #ty_generics #where_clause
      {
        #[inline]
        fn none() -> Self::Abi {
          <::wasm_bindgen::JsValue as ::wasm_bindgen::convert::OptionIntoWasmAbi>::none()
        }
      }

      #[cfg(#cfg)]
      impl #impl_generics ::wasm_bindgen::convert::OptionFromWasmAbi
        for #ident #ty_generics #where_clause
      {
        #[inline]
        fn is_none(abi: &Self::Abi) -> bool {
          <::wasm_bindgen::JsValue as ::wasm_bindgen::convert::OptionFromWasmAbi>::is_none(abi)
        }
      }

      #[cfg(#cfg)]
      impl #impl_generics ::wasm_bindgen::convert::TryFromJsValue
        for #ident #ty_generics #where_clause
      {
        fn try_from_js_value(
          value: ::wasm_bindgen::JsValue,
        ) -> ::core::result::Result<Self, ::wasm_bindgen::JsValue> {
          ::typewire::Typewire::from_js(value)
            .map_err(|e| ::wasm_bindgen::JsValue::from_str(
              &::std::string::ToString::to_string(&e),
            ))
        }

        fn try_from_js_value_ref(value: &::wasm_bindgen::JsValue) -> ::core::option::Option<Self> {
          ::typewire::Typewire::from_js(value.clone()).ok()
        }
      }

      #js_bindings
    }
  }
}

// ---------------------------------------------------------------------------
// Struct codegen
// ---------------------------------------------------------------------------

fn struct_to_js_body(s: &SchemaStruct) -> TokenStream {
  match &s.shape {
    StructShape::Named(fields) => {
      let type_name = s.ident.to_string();
      let construct_fn = format_ident!("__tw_{type_name}_construct");

      // Build argument list: one arg per non-SKIP_SER field, in field order
      // (matching the parameter order of the JS construct function).
      let args: Vec<TokenStream> = fields
        .iter()
        .filter(|f| !f.flags.contains(FieldFlags::SKIP_SER))
        .map(|f| {
          let ident = &f.ident;
          let ident_ts = quote! { &self.#ident };
          let to_js = field_to_js_expr(&ident_ts, f);

          if let Some(ref pred_path) = f.skip_serializing_if {
            // Pass UNDEFINED when the predicate says to skip.
            quote! {
              if #pred_path(&self.#ident) {
                ::wasm_bindgen::JsValue::UNDEFINED
              } else {
                #to_js
              }
            }
          } else {
            to_js
          }
        })
        .collect();

      quote! {
        #construct_fn(#(#args),*).into()
      }
    }
    StructShape::Tuple(types) => {
      let setters = types.iter().enumerate().map(|(i, _)| {
        let idx = Index::from(i);
        quote! {
          arr.push(&::typewire::Typewire::to_js(&self.#idx));
        }
      });
      quote! {
        let arr = ::js_sys::Array::new();
        #(#setters)*
        arr.into()
      }
    }
    StructShape::Unit => quote! { ::wasm_bindgen::JsValue::NULL },
  }
}

fn struct_from_js_body(s: &SchemaStruct) -> TokenStream {
  let name = &s.ident;
  match &s.shape {
    StructShape::Named(fields) => {
      let type_name = s.ident.to_string();
      let destruct_fn = format_ident!("__tw_{type_name}_destruct");

      // deny_unknown_fields check via JS helper
      let deny_check = if s.flags.contains(StructFlags::DENY_UNKNOWN_FIELDS) {
        let check_fn = format_ident!("__tw_{type_name}_check_keys");
        quote! {
          if let Some(__unknown) = #check_fn(&value).as_string() {
            return Err(::typewire::Error::InvalidValue {
              message: ::std::format!("unknown field `{__unknown}`"),
            });
          }
        }
      } else {
        quote! {}
      };

      // Call destruct, then bind each field from the array.
      // Active fields (not both-skip) map 1:1 to array positions.
      let mut arr_idx: u32 = 0;
      let field_bindings: Vec<TokenStream> = fields
        .iter()
        .map(|f| {
          let ident = &f.ident;

          // Fully skipped fields (both SKIP_SER and SKIP_DE) are not in the
          // destruct array — just use their default.
          if f.flags.contains(FieldFlags::SKIP_SER) && f.flags.contains(FieldFlags::SKIP_DE) {
            let default_expr = default_expr_for_field(f);
            return quote! { let #ident = #default_expr; };
          }

          // SKIP_DE fields are in the destruct array (for patch_js) but
          // from_js ignores them and uses the default value.
          if f.flags.contains(FieldFlags::SKIP_DE) {
            arr_idx += 1; // consume the array slot
            let default_expr = default_expr_for_field(f);
            return quote! { let #ident = #default_expr; };
          }

          // FLATTEN fields get the whole parent object from destruct.
          if f.flags.contains(FieldFlags::FLATTEN) {
            let ty = &f.ty;
            let field_str = ident.to_string();
            let idx = arr_idx;
            arr_idx += 1;
            return quote! {
              let #ident = <#ty as ::typewire::Typewire>::from_js(__arr.get(#idx))
                .map_err(|e| e.in_context(#field_str))?;
            };
          }

          let idx = arr_idx;
          arr_idx += 1;
          let from_js = field_from_js_expr(f);
          let js_key = &f.wire_name;
          let ty = &f.ty;
          let has_default = !matches!(f.default, SchemaFieldDefault::None);

          if has_default {
            let default_expr = default_expr_for_field(f);
            quote! {
              let #ident = {
                let v = __arr.get(#idx);
                if !v.is_undefined() && !v.is_null() {
                  #from_js
                } else {
                  #default_expr
                }
              };
            }
          } else {
            quote! {
              let #ident = {
                let v = __arr.get(#idx);
                if !v.is_undefined() {
                  #from_js
                } else {
                  match <#ty as ::typewire::Typewire>::or_default() {
                    Some(d) => d,
                    None => return Err(::typewire::Error::MissingField { field: #js_key }),
                  }
                }
              };
            }
          }
        })
        .collect();

      let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();

      quote! {
        let __arr = #destruct_fn(&value);
        #deny_check
        #(#field_bindings)*
        Ok(#name { #(#field_names,)* })
      }
    }
    StructShape::Tuple(types) => {
      let bindings = types.iter().enumerate().map(|(i, ty)| {
        let var = format_ident!("__field{i}");
        #[expect(clippy::cast_possible_truncation, reason = "field index always fits u32")]
        let idx = i as u32;
        quote! {
          let #var = <#ty as ::typewire::Typewire>::from_js(arr.get(#idx))?;
        }
      });
      let vars: Vec<_> = (0..types.len()).map(|i| format_ident!("__field{i}")).collect();
      quote! {
        let arr: ::js_sys::Array = value
          .try_into()
          .map_err(|_| ::typewire::Error::UnexpectedType { expected: "array" })?;
        #(#bindings)*
        Ok(#name(#(#vars),*))
      }
    }
    StructShape::Unit => quote! { Ok(#name) },
  }
}

fn struct_patch_js_fn(s: &SchemaStruct) -> TokenStream {
  if s.flags.contains(StructFlags::ATOMIC) {
    return quote! {
      fn patch_js(&self, old: &::wasm_bindgen::JsValue, set: impl FnOnce(::wasm_bindgen::JsValue)) {
        match Self::from_js(old.clone()) {
          Ok(ref old_val) if self == old_val => {}
          _ => set(self.to_js()),
        }
      }
    };
  }

  match &s.shape {
    StructShape::Named(fields) => {
      let type_name = s.ident.to_string();
      let destruct_fn = format_ident!("__tw_{type_name}_destruct");

      let mut arr_idx: u32 = 0;
      let field_patches: Vec<TokenStream> = fields
        .iter()
        .filter_map(|f| {
          if f.flags.contains(FieldFlags::SKIP_SER) && f.flags.contains(FieldFlags::SKIP_DE) {
            return None;
          }
          let ident = &f.ident;

          if f.flags.contains(FieldFlags::FLATTEN) {
            let idx = arr_idx;
            arr_idx += 1;
            return Some(quote! {
              ::typewire::Typewire::patch_js(
                &self.#ident,
                &__arr.get(#idx),
                |_| {},
              );
            });
          }

          let idx = arr_idx;
          arr_idx += 1;
          let ident_ts = quote! { &self.#ident };
          let to_js = field_to_js_expr(&ident_ts, f);
          let is_special =
            f.flags.intersects(FieldFlags::BASE64 | FieldFlags::DISPLAY | FieldFlags::SERDE_BYTES);
          let setter_fn = format_ident!("__tw_{type_name}_set_{ident}");

          let patch_call = if is_special {
            quote! {
              {
                let __old_v = __arr.get(#idx);
                let __new_v = #to_js;
                if __old_v != __new_v {
                  #setter_fn(old, __new_v);
                }
              }
            }
          } else {
            quote! {
              ::typewire::Typewire::patch_js(
                #ident_ts,
                &__arr.get(#idx),
                |v| #setter_fn(old, v),
              );
            }
          };

          Some(patch_call)
        })
        .collect();

      quote! {
        fn patch_js(&self, old: &::wasm_bindgen::JsValue, _set: impl FnOnce(::wasm_bindgen::JsValue)) {
          if old.is_undefined() || old.is_null() {
            _set(self.to_js());
            return;
          }
          let __arr = #destruct_fn(old);
          #(#field_patches)*
        }
      }
    }
    StructShape::Tuple(types) => {
      let patches: Vec<_> = (0..types.len())
        .map(|i| {
          let idx = Index::from(i);
          #[expect(clippy::cast_possible_truncation, reason = "field index always fits u32")]
          let js_idx = i as u32;
          quote! {
            self.#idx.patch_js(&__arr.get(#js_idx), |v| __arr.set(#js_idx, v));
          }
        })
        .collect();
      quote! {
        fn patch_js(&self, old: &::wasm_bindgen::JsValue, _set: impl FnOnce(::wasm_bindgen::JsValue)) {
          use ::wasm_bindgen::JsCast as _;
          let Some(__arr) = old.dyn_ref::<::js_sys::Array>() else {
            _set(self.to_js());
            return;
          };
          #(#patches)*
        }
      }
    }
    StructShape::Unit => {
      quote! {
        fn patch_js(&self, _old: &::wasm_bindgen::JsValue, _set: impl FnOnce(::wasm_bindgen::JsValue)) {}
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Enum dispatch
// ---------------------------------------------------------------------------

fn enum_to_js_body(e: &SchemaEnum) -> TokenStream {
  match &e.tagging {
    Tagging::Untagged => untagged_to_js(e),
    Tagging::Internal { tag } => int_tagged_to_js(e, tag),
    Tagging::Adjacent { .. } => adj_tagged_to_js(e),
    Tagging::External => ext_tagged_to_js(e),
  }
}

fn enum_from_js_body(e: &SchemaEnum) -> TokenStream {
  match &e.tagging {
    Tagging::Untagged => untagged_from_js(e),
    Tagging::Internal { tag } => int_tagged_from_js(e, tag),
    Tagging::Adjacent { tag, .. } => adj_tagged_from_js(e, tag),
    Tagging::External => ext_tagged_from_js(e),
  }
}

fn enum_patch_js_fn(e: &SchemaEnum) -> TokenStream {
  if e.flags.contains(EnumFlags::ATOMIC) {
    return quote! {
      fn patch_js(&self, old: &::wasm_bindgen::JsValue, set: impl FnOnce(::wasm_bindgen::JsValue)) {
        ::typewire::patch_js_atomic(self, old, set);
      }
    };
  }

  if e.flags.contains(EnumFlags::ALL_UNIT) {
    return quote! {
      fn patch_js(&self, old: &::wasm_bindgen::JsValue, set: impl FnOnce(::wasm_bindgen::JsValue)) {
        let new = self.to_js();
        if old != &new {
          set(new);
        }
      }
    };
  }

  match &e.tagging {
    Tagging::Untagged => {
      quote! {
        fn patch_js(&self, old: &::wasm_bindgen::JsValue, set: impl FnOnce(::wasm_bindgen::JsValue)) {
          let new = self.to_js();
          if old != &new {
            set(new);
          }
        }
      }
    }
    Tagging::Internal { .. } => int_tagged_patch_js(e),
    Tagging::Adjacent { content, .. } => adj_tagged_patch_js(e, content),
    Tagging::External => ext_tagged_patch_js(e),
  }
}

// ---------------------------------------------------------------------------
// Externally tagged: `{ "VariantName": <content> }` or `"VariantName"`
// ---------------------------------------------------------------------------

fn ext_tagged_to_js_arms(e: &SchemaEnum) -> impl Iterator<Item = TokenStream> + '_ {
  let name = &e.ident;
  let type_name = e.ident.to_string();
  e.variants.iter().filter_map(move |v| {
    if v.flags.contains(VariantFlags::SKIP_SER) {
      return None;
    }

    let tag_str = &v.wire_name;
    let vname = &v.ident;
    let construct_fn = format_ident!("__tw_{type_name}_construct_{vname}");

    match &v.kind {
      VariantKind::Unit => Some(quote! {
        #name::#vname => ::wasm_bindgen::JsValue::from_str(#tag_str),
      }),
      VariantKind::Unnamed(types) => {
        let binds: Vec<_> = (0..types.len()).map(|i| format_ident!("__f{i}")).collect();
        let content = if binds.len() == 1 {
          let b = &binds[0];
          quote! { ::typewire::Typewire::to_js(#b) }
        } else {
          let pushes = binds.iter().map(|b| quote! { arr.push(&::typewire::Typewire::to_js(#b)); });
          quote! { { let arr = ::js_sys::Array::new(); #(#pushes)* arr.into() } }
        };
        Some(quote! {
          #name::#vname(#(#binds),*) => #construct_fn(#content).into(),
        })
      }
      VariantKind::Named(fields) => {
        let (binds, args) = variant_to_js_args(fields);
        Some(quote! {
          #name::#vname { #(#binds,)* .. } => #construct_fn(#(#args),*).into(),
        })
      }
    }
  })
}

fn ext_tagged_to_js(e: &SchemaEnum) -> TokenStream {
  let to_js_arms = ext_tagged_to_js_arms(e);
  quote! {
    match self {
      #(#to_js_arms)*
      #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(unreachable_patterns, reason = "wasm-only variants may be skipped")
      )]
      _ => ::wasm_bindgen::JsValue::UNDEFINED,
    }
  }
}

fn ext_tagged_from_js(e: &SchemaEnum) -> TokenStream {
  let name = &e.ident;
  let type_name = e.ident.to_string();
  let dispatch_fn = format_ident!("__tw_{type_name}_dispatch");

  let tagged_variants: Vec<&SchemaVariant> = e
    .variants
    .iter()
    .filter(|v| {
      !v.flags.contains(VariantFlags::SKIP_DE)
        && !v.flags.contains(VariantFlags::UNTAGGED)
        && !v.flags.contains(VariantFlags::OTHER)
    })
    .collect();

  let dispatch_arms: Vec<TokenStream> = tagged_variants
    .iter()
    .enumerate()
    .map(|(i, v)| {
      let vname = &v.ident;
      #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "variant index always fits i32"
      )]
      let idx = i as i32;

      match &v.kind {
        VariantKind::Unit => quote! {
          #idx => return Ok(#name::#vname),
        },
        VariantKind::Unnamed(types) => {
          let content_fn = format_ident!("__tw_{type_name}_content_{vname}");
          if types.len() == 1 {
            let ty = &types[0];
            quote! {
              #idx => {
                let __c = #content_fn(&value);
                let inner = <#ty as ::typewire::Typewire>::from_js(__c)?;
                return Ok(#name::#vname(inner));
              }
            }
          } else {
            let bindings = types.iter().enumerate().map(|(fi, ty)| {
              let var = format_ident!("__f{fi}");
              #[expect(clippy::cast_possible_truncation, reason = "field index always fits u32")]
              let fidx = fi as u32;
              quote! { let #var = <#ty as ::typewire::Typewire>::from_js(__arr.get(#fidx))?; }
            });
            let vars: Vec<_> = (0..types.len()).map(|fi| format_ident!("__f{fi}")).collect();
            quote! {
              #idx => {
                let __c = #content_fn(&value);
                let __arr: ::js_sys::Array = __c.try_into()
                  .map_err(|_| ::typewire::Error::UnexpectedType { expected: "array" })?;
                #(#bindings)*
                return Ok(#name::#vname(#(#vars),*));
              }
            }
          }
        }
        VariantKind::Named(fields) => {
          let content_fn = format_ident!("__tw_{type_name}_content_{vname}");
          let field_bindings = named_fields_from_destruct_arr(fields);
          let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();
          let has_active = fields.iter().any(|f| !f.flags.contains(FieldFlags::SKIP_DE));
          let destruct_call = if has_active {
            let destruct_fn = format_ident!("__tw_{type_name}_destruct_{vname}");
            quote! { let __arr = #destruct_fn(&__content); }
          } else {
            quote! {}
          };
          quote! {
            #idx => {
              let __content = #content_fn(&value);
              #destruct_call
              #(#field_bindings)*
              return Ok(#name::#vname { #(#field_names,)* });
            }
          }
        }
      }
    })
    .collect();

  let untagged_fallbacks = untagged_variant_fallbacks(name, &e.variants);
  let other_fallback = other_variant_fallback(name, &e.variants);
  let has_fallbacks = !untagged_fallbacks.is_empty() || !other_fallback.is_empty();

  let all_unit = e.flags.contains(EnumFlags::ALL_UNIT);

  if has_fallbacks && all_unit {
    let string_arms: Vec<TokenStream> = tagged_variants
      .iter()
      .map(|v| {
        let all_names = &v.all_wire_names;
        let vname = &v.ident;
        quote! { #(#all_names)|* => return Ok(#name::#vname), }
      })
      .collect();
    let final_err = if other_fallback.is_empty() {
      quote! { Err(::typewire::Error::NoMatchingVariant) }
    } else {
      other_fallback
    };

    quote! {
      if let Some(s) = value.as_string() {
        match s.as_str() {
          #(#string_arms)*
          _ => {}
        }
      }
      #(#untagged_fallbacks)*
      #final_err
    }
  } else if has_fallbacks {
    let final_err = if other_fallback.is_empty() {
      quote! { Err(::typewire::Error::NoMatchingVariant) }
    } else {
      other_fallback
    };

    quote! {
      let __idx = #dispatch_fn(&value);
      match __idx {
        #(#dispatch_arms)*
        _ => {}
      }
      #(#untagged_fallbacks)*
      #final_err
    }
  } else if all_unit {
    let string_arms: Vec<TokenStream> = tagged_variants
      .iter()
      .map(|v| {
        let all_names = &v.all_wire_names;
        let vname = &v.ident;
        quote! { #(#all_names)|* => Ok(#name::#vname), }
      })
      .collect();
    quote! {
      let s = value.as_string()
        .ok_or(::typewire::Error::UnexpectedType { expected: "string" })?;
      match s.as_str() {
        #(#string_arms)*
        other => Err(::typewire::Error::UnknownVariant { variant: other.into() }),
      }
    }
  } else {
    quote! {
      match #dispatch_fn(&value) {
        #(#dispatch_arms)*
        _ => Err(::typewire::Error::UnknownVariant {
          variant: value.as_string().unwrap_or_default(),
        }),
      }
    }
  }
}

fn ext_tagged_patch_js(e: &SchemaEnum) -> TokenStream {
  let name = &e.ident;
  let type_name = e.ident.to_string();
  let dispatch_fn = format_ident!("__tw_{type_name}_dispatch");

  let tagged: Vec<&SchemaVariant> = e
    .variants
    .iter()
    .filter(|v| {
      !v.flags.contains(VariantFlags::SKIP_DE)
        && !v.flags.contains(VariantFlags::UNTAGGED)
        && !v.flags.contains(VariantFlags::OTHER)
    })
    .collect();

  let arms: Vec<_> = e
    .variants
    .iter()
    .filter_map(|v| {
      if v.flags.contains(VariantFlags::SKIP_SER) {
        return None;
      }
      let vname = &v.ident;
      let expected_idx = tagged.iter().position(|tv| tv.ident == v.ident);
      #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "variant count always fits i32"
      )]
      let cmp = expected_idx.map_or_else(
        || quote! { true },
        |i| {
          let idx = i as i32;
          quote! { __old_idx != #idx }
        },
      );

      match &v.kind {
        VariantKind::Unit => Some(quote! {
          #name::#vname => {
            if #cmp { _set(self.to_js()); }
          }
        }),
        VariantKind::Named(fields) => {
          let binds = field_binds(fields);
          let active: Vec<&SchemaField> = fields
            .iter()
            .filter(|f| {
              !(f.flags.contains(FieldFlags::SKIP_SER) && f.flags.contains(FieldFlags::SKIP_DE))
            })
            .collect();
          if active.is_empty() {
            return Some(quote! {
              #name::#vname { #(#binds,)* .. } => {
                if #cmp { _set(self.to_js()); }
              }
            });
          }
          let content_fn = format_ident!("__tw_{type_name}_content_{vname}");
          let destruct_fn = format_ident!("__tw_{type_name}_destruct_{vname}");
          let mut arr_idx: u32 = 0;
          let patches: Vec<TokenStream> = active
            .iter()
            .map(|f| {
              let ident = &f.ident;
              if f.flags.contains(FieldFlags::FLATTEN) {
                let idx = arr_idx;
                arr_idx += 1;
                return quote! {
                  ::typewire::Typewire::patch_js(#ident, &__varr.get(#idx), |_| {});
                };
              }
              let idx = arr_idx;
              arr_idx += 1;
              let ident_ts = quote! { #ident };
              let to_js = field_to_js_expr(&ident_ts, f);
              let setter_fn = format_ident!("__tw_{type_name}_set_{vname}_{ident}");
              let is_special = f
                .flags
                .intersects(FieldFlags::BASE64 | FieldFlags::DISPLAY | FieldFlags::SERDE_BYTES);
              if is_special {
                quote! {
                  {
                    let __old_v = __varr.get(#idx);
                    let __new_v = #to_js;
                    if __old_v != __new_v { #setter_fn(&__content, __new_v); }
                  }
                }
              } else {
                quote! {
                  ::typewire::Typewire::patch_js(
                    #ident_ts,
                    &__varr.get(#idx),
                    |v| #setter_fn(&__content, v),
                  );
                }
              }
            })
            .collect();

          Some(quote! {
            #name::#vname { #(#binds,)* .. } => {
              if #cmp {
                _set(self.to_js());
              } else {
                let __content = #content_fn(old);
                let __varr = #destruct_fn(&__content);
                #(#patches)*
              }
            }
          })
        }
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          let content_fn = format_ident!("__tw_{type_name}_content_{vname}");
          Some(quote! {
            #name::#vname(__inner) => {
              if #cmp {
                _set(self.to_js());
              } else {
                let __content = #content_fn(old);
                <#ty as ::typewire::Typewire>::patch_js(__inner, &__content, |v| {
                  _set(self.to_js());
                });
              }
            }
          })
        }
        VariantKind::Unnamed(_) => Some(quote! {
          #name::#vname(..) => { _set(self.to_js()); }
        }),
      }
    })
    .collect();

  quote! {
    fn patch_js(&self, old: &::wasm_bindgen::JsValue, _set: impl FnOnce(::wasm_bindgen::JsValue)) {
      if old.is_undefined() || old.is_null() {
        _set(self.to_js());
        return;
      }
      let __old_idx = #dispatch_fn(old);
      match self {
        #(#arms)*
        #[expect(unreachable_patterns, reason = "wasm-only variants may be skipped")]
        _ => { _set(self.to_js()); }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Internally tagged: `{ "tag": "VariantName", ...fields }`
// ---------------------------------------------------------------------------

fn int_tagged_to_js_arms<'a>(
  e: &'a SchemaEnum,
  tag: &'a str,
) -> impl Iterator<Item = TokenStream> + 'a {
  let name = &e.ident;
  let type_name = e.ident.to_string();
  e.variants.iter().filter_map(move |v| {
    if v.flags.contains(VariantFlags::SKIP_SER) {
      return None;
    }
    let vname = &v.ident;
    let construct_fn = format_ident!("__tw_{type_name}_construct_{vname}");

    match &v.kind {
      VariantKind::Unit => Some(quote! {
        #name::#vname => #construct_fn().into(),
      }),
      VariantKind::Named(fields) => {
        let (binds, args) = variant_to_js_args(fields);
        Some(quote! {
          #name::#vname { #(#binds,)* .. } => #construct_fn(#(#args),*).into(),
        })
      }
      VariantKind::Unnamed(types) if types.len() == 1 => {
        // Internally tagged single newtype: injects tag into inner's to_js result.
        // Reflect::set: internally-tagged single-newtype `to_js` must inject the
        // tag field into the *inner type's* already-constructed JS object. A JS
        // construct helper cannot be used here because the object is produced by
        // the inner type's own `to_js`, not by us.
        let tag_val = &v.wire_name;
        Some(quote! {
          #name::#vname(__inner) => {
            let obj_val = ::typewire::Typewire::to_js(__inner);
            let _ = ::js_sys::Reflect::set(
              &obj_val,
              &::wasm_bindgen::JsValue::from_str(#tag),
              &::wasm_bindgen::JsValue::from_str(#tag_val),
            );
            obj_val
          }
        })
      }
      // Multi-field tuple variants are rejected by analyze's validation.
      VariantKind::Unnamed(_) => None,
    }
  })
}

fn int_tagged_to_js(e: &SchemaEnum, tag: &str) -> TokenStream {
  let to_js_arms = int_tagged_to_js_arms(e, tag);
  quote! {
    match self {
      #(#to_js_arms)*
      #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(unreachable_patterns, reason = "wasm-only variants may be skipped")
      )]
      _ => ::wasm_bindgen::JsValue::UNDEFINED,
    }
  }
}

fn int_tagged_from_js(e: &SchemaEnum, tag: &str) -> TokenStream {
  let name = &e.ident;
  let type_name = e.ident.to_string();
  let all_unit = e.flags.contains(EnumFlags::ALL_UNIT);

  let tagged_variants: Vec<&SchemaVariant> = e
    .variants
    .iter()
    .filter(|v| {
      !v.flags.contains(VariantFlags::SKIP_DE)
        && !v.flags.contains(VariantFlags::UNTAGGED)
        && !v.flags.contains(VariantFlags::OTHER)
    })
    .collect();

  let untagged_fallbacks = untagged_variant_fallbacks(name, &e.variants);
  let other_fallback = other_variant_fallback(name, &e.variants);
  let has_fallbacks = !untagged_fallbacks.is_empty() || !other_fallback.is_empty();

  // All-unit enums have no JS dispatch helper — use direct tag string
  // matching. Reflect::get reads the tag field; this is the most efficient
  // approach for all-unit enums since no JS boundary crossing is needed
  // beyond the single Reflect::get, and the string match avoids allocating
  // an intermediate index.
  if all_unit {
    let string_arms: Vec<TokenStream> = tagged_variants
      .iter()
      .map(|v| {
        let all_names = &v.all_wire_names;
        let vname = &v.ident;
        if has_fallbacks {
          quote! { #(#all_names)|* => return Ok(#name::#vname), }
        } else {
          quote! { #(#all_names)|* => Ok(#name::#vname), }
        }
      })
      .collect();

    if has_fallbacks {
      let final_err = if other_fallback.is_empty() {
        quote! { Err(::typewire::Error::NoMatchingVariant) }
      } else {
        other_fallback
      };
      return quote! {
        // Reflect::get: all-unit enum tag read (see comment above).
        let __tag_val = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#tag))
          .ok()
          .and_then(|v| v.as_string());
        if let Some(ref tag_val) = __tag_val {
          match tag_val.as_str() {
            #(#string_arms)*
            _ => {}
          }
        }
        #(#untagged_fallbacks)*
        #final_err
      };
    }
    return quote! {
      // Reflect::get: all-unit enum tag read (see comment above).
      let tag_val = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#tag))
        .ok()
        .and_then(|v| v.as_string())
        .ok_or(::typewire::Error::MissingField { field: #tag })?;
      match tag_val.as_str() {
        #(#string_arms)*
        other => Err(::typewire::Error::UnknownVariant { variant: other.into() }),
      }
    };
  }

  let dispatch_fn = format_ident!("__tw_{type_name}_dispatch");

  let dispatch_arms: Vec<TokenStream> = tagged_variants
    .iter()
    .enumerate()
    .filter_map(|(i, v)| {
      let vname = &v.ident;
      #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "variant index always fits i32"
      )]
      let idx = i as i32;

      match &v.kind {
        VariantKind::Unit => Some(quote! {
          #idx => return Ok(#name::#vname),
        }),
        VariantKind::Named(fields) => {
          let field_bindings = named_fields_from_destruct_arr(fields);
          let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();
          let has_active = fields.iter().any(|f| !f.flags.contains(FieldFlags::SKIP_DE));
          let destruct_call = if has_active {
            let destruct_fn = format_ident!("__tw_{type_name}_destruct_{vname}");
            quote! { let __arr = #destruct_fn(&value); }
          } else {
            quote! {}
          };
          Some(quote! {
            #idx => {
              #destruct_call
              #(#field_bindings)*
              return Ok(#name::#vname { #(#field_names,)* });
            }
          })
        }
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          Some(quote! {
            #idx => {
              let inner = <#ty as ::typewire::Typewire>::from_js(value)?;
              return Ok(#name::#vname(inner));
            }
          })
        }
        VariantKind::Unnamed(_) => None,
      }
    })
    .collect();

  if has_fallbacks {
    let final_err = if other_fallback.is_empty() {
      quote! { Err(::typewire::Error::NoMatchingVariant) }
    } else {
      other_fallback
    };

    quote! {
      match #dispatch_fn(&value) {
        #(#dispatch_arms)*
        _ => {}
      }
      #(#untagged_fallbacks)*
      #final_err
    }
  } else {
    quote! {
      match #dispatch_fn(&value) {
        #(#dispatch_arms)*
        _ => {}
      }
      {
        // Reflect::get: error-path only — reads the tag string to produce a
        // descriptive `UnknownVariant` / `MissingField` error message.
        let __tag_str = ::js_sys::Reflect::get(
          &value,
          &::wasm_bindgen::JsValue::from_str(#tag),
        )
        .ok()
        .and_then(|v| v.as_string());
        match __tag_str {
          Some(t) => Err(::typewire::Error::UnknownVariant { variant: t }),
          None => Err(::typewire::Error::MissingField { field: #tag }),
        }
      }
    }
  }
}

fn int_tagged_patch_js(e: &SchemaEnum) -> TokenStream {
  let name = &e.ident;
  let type_name = e.ident.to_string();
  let dispatch_fn = format_ident!("__tw_{type_name}_dispatch");

  // Build a mapping from variant index → patch arm.
  // The dispatch indices must match the same filtered order as js_enum_dispatch.
  let tagged: Vec<&SchemaVariant> = e
    .variants
    .iter()
    .filter(|v| {
      !v.flags.contains(VariantFlags::SKIP_DE)
        && !v.flags.contains(VariantFlags::UNTAGGED)
        && !v.flags.contains(VariantFlags::OTHER)
    })
    .collect();

  let arms: Vec<_> = e
    .variants
    .iter()
    .filter_map(|v| {
      if v.flags.contains(VariantFlags::SKIP_SER) {
        return None;
      }
      let vname = &v.ident;

      // Find this variant's dispatch index.
      let expected_idx = tagged.iter().position(|tv| tv.ident == v.ident);
      #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "variant count always fits i32"
      )]
      let cmp = expected_idx.map_or_else(
        || quote! { true },
        |i| {
          let idx = i as i32;
          quote! { __old_idx != #idx }
        },
      );

      match &v.kind {
        VariantKind::Unit => Some(quote! {
          #name::#vname => {
            if #cmp { _set(self.to_js()); }
          }
        }),
        VariantKind::Named(fields) => {
          let binds = field_binds(fields);
          let destruct_fn = format_ident!("__tw_{type_name}_destruct_{vname}");
          let active: Vec<&SchemaField> = fields
            .iter()
            .filter(|f| {
              !(f.flags.contains(FieldFlags::SKIP_SER) && f.flags.contains(FieldFlags::SKIP_DE))
            })
            .collect();
          let mut arr_idx: u32 = 0;
          let patches: Vec<TokenStream> = active
            .iter()
            .map(|f| {
              let ident = &f.ident;
              if f.flags.contains(FieldFlags::FLATTEN) {
                let idx = arr_idx;
                arr_idx += 1;
                return quote! {
                  ::typewire::Typewire::patch_js(#ident, &__varr.get(#idx), |_| {});
                };
              }
              let idx = arr_idx;
              arr_idx += 1;
              let ident_ts = quote! { #ident };
              let to_js = field_to_js_expr(&ident_ts, f);
              let setter_fn = format_ident!("__tw_{type_name}_set_{vname}_{ident}");
              let is_special = f
                .flags
                .intersects(FieldFlags::BASE64 | FieldFlags::DISPLAY | FieldFlags::SERDE_BYTES);
              if is_special {
                quote! {
                  {
                    let __old_v = __varr.get(#idx);
                    let __new_v = #to_js;
                    if __old_v != __new_v { #setter_fn(old, __new_v); }
                  }
                }
              } else {
                quote! {
                  ::typewire::Typewire::patch_js(
                    #ident_ts,
                    &__varr.get(#idx),
                    |v| #setter_fn(old, v),
                  );
                }
              }
            })
            .collect();

          Some(quote! {
            #name::#vname { #(#binds,)* .. } => {
              if #cmp {
                _set(self.to_js());
              } else {
                let __varr = #destruct_fn(old);
                #(#patches)*
              }
            }
          })
        }
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          Some(quote! {
            #name::#vname(__inner) => {
              if #cmp {
                _set(self.to_js());
              } else {
                <#ty as ::typewire::Typewire>::patch_js(__inner, old, |v| _set(v));
              }
            }
          })
        }
        VariantKind::Unnamed(_) => Some(quote! {
          #name::#vname(..) => { _set(self.to_js()); }
        }),
      }
    })
    .collect();

  quote! {
    fn patch_js(&self, old: &::wasm_bindgen::JsValue, _set: impl FnOnce(::wasm_bindgen::JsValue)) {
      if old.is_undefined() || old.is_null() {
        _set(self.to_js());
        return;
      }
      let __old_idx = #dispatch_fn(old);
      match self {
        #(#arms)*
        #[expect(unreachable_patterns, reason = "wasm-only variants may be skipped")]
        _ => { _set(self.to_js()); }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Adjacently tagged: `{ "t": "VariantName", "c": <content> }`
// ---------------------------------------------------------------------------

fn adj_tagged_to_js_arms(e: &SchemaEnum) -> impl Iterator<Item = TokenStream> + '_ {
  let name = &e.ident;
  let type_name = e.ident.to_string();
  e.variants.iter().filter_map(move |v| {
    if v.flags.contains(VariantFlags::SKIP_SER) {
      return None;
    }
    let vname = &v.ident;
    let construct_fn = format_ident!("__tw_{type_name}_construct_{vname}");

    match &v.kind {
      VariantKind::Unit => Some(quote! {
        #name::#vname => #construct_fn().into(),
      }),
      VariantKind::Unnamed(types) => {
        let binds: Vec<_> = (0..types.len()).map(|i| format_ident!("__f{i}")).collect();
        let content_val = if binds.len() == 1 {
          let b = &binds[0];
          quote! { ::typewire::Typewire::to_js(#b) }
        } else {
          let pushes = binds.iter().map(|b| quote! { arr.push(&::typewire::Typewire::to_js(#b)); });
          quote! { { let arr = ::js_sys::Array::new(); #(#pushes)* arr.into() } }
        };
        Some(quote! {
          #name::#vname(#(#binds),*) => #construct_fn(#content_val).into(),
        })
      }
      VariantKind::Named(fields) => {
        let (binds, args) = variant_to_js_args(fields);
        Some(quote! {
          #name::#vname { #(#binds,)* .. } => #construct_fn(#(#args),*).into(),
        })
      }
    }
  })
}

fn adj_tagged_to_js(e: &SchemaEnum) -> TokenStream {
  let to_js_arms = adj_tagged_to_js_arms(e);
  quote! {
    match self {
      #(#to_js_arms)*
      #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(unreachable_patterns, reason = "wasm-only variants may be skipped")
      )]
      _ => ::wasm_bindgen::JsValue::UNDEFINED,
    }
  }
}

fn adj_tagged_from_js(e: &SchemaEnum, tag: &str) -> TokenStream {
  let name = &e.ident;
  let type_name = e.ident.to_string();
  let all_unit = e.flags.contains(EnumFlags::ALL_UNIT);

  let tagged_variants: Vec<&SchemaVariant> = e
    .variants
    .iter()
    .filter(|v| {
      !v.flags.contains(VariantFlags::SKIP_DE)
        && !v.flags.contains(VariantFlags::UNTAGGED)
        && !v.flags.contains(VariantFlags::OTHER)
    })
    .collect();

  let untagged_fallbacks = untagged_variant_fallbacks(name, &e.variants);
  let other_fallback = other_variant_fallback(name, &e.variants);
  let has_fallbacks = !untagged_fallbacks.is_empty() || !other_fallback.is_empty();

  // All-unit enums have no JS dispatch helper — use direct tag string
  // matching. Reflect::get reads the tag field (same rationale as
  // `int_tagged_from_js`).
  if all_unit {
    let string_arms: Vec<TokenStream> = tagged_variants
      .iter()
      .map(|v| {
        let all_names = &v.all_wire_names;
        let vname = &v.ident;
        if has_fallbacks {
          quote! { #(#all_names)|* => return Ok(#name::#vname), }
        } else {
          quote! { #(#all_names)|* => Ok(#name::#vname), }
        }
      })
      .collect();

    if has_fallbacks {
      let final_err = if other_fallback.is_empty() {
        quote! { Err(::typewire::Error::NoMatchingVariant) }
      } else {
        other_fallback
      };
      return quote! {
        // Reflect::get: all-unit enum tag read (see comment above).
        let __tag_val = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#tag))
          .ok()
          .and_then(|v| v.as_string());
        if let Some(ref tag_val) = __tag_val {
          match tag_val.as_str() {
            #(#string_arms)*
            _ => {}
          }
        }
        #(#untagged_fallbacks)*
        #final_err
      };
    }
    return quote! {
      // Reflect::get: all-unit enum tag read (see comment above).
      let tag_val = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#tag))
        .ok()
        .and_then(|v| v.as_string())
        .ok_or(::typewire::Error::MissingField { field: #tag })?;
      match tag_val.as_str() {
        #(#string_arms)*
        other => Err(::typewire::Error::UnknownVariant { variant: other.into() }),
      }
    };
  }

  let dispatch_fn = format_ident!("__tw_{type_name}_dispatch");
  let content_fn = format_ident!("__tw_{type_name}_content");

  let dispatch_arms: Vec<TokenStream> = tagged_variants
    .iter()
    .enumerate()
    .map(|(i, v)| {
      let vname = &v.ident;
      #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "variant index always fits i32"
      )]
      let idx = i as i32;

      match &v.kind {
        VariantKind::Unit => quote! {
          #idx => Ok(#name::#vname),
        },
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          quote! {
            #idx => {
              let __c = #content_fn(&value);
              let inner = <#ty as ::typewire::Typewire>::from_js(__c)?;
              Ok(#name::#vname(inner))
            }
          }
        }
        VariantKind::Unnamed(types) => {
          let bindings = types.iter().enumerate().map(|(fi, ty)| {
            let var = format_ident!("__f{fi}");
            #[expect(clippy::cast_possible_truncation, reason = "field index always fits u32")]
            let fidx = fi as u32;
            quote! { let #var = <#ty as ::typewire::Typewire>::from_js(__arr.get(#fidx))?; }
          });
          let vars: Vec<_> = (0..types.len()).map(|fi| format_ident!("__f{fi}")).collect();
          quote! {
            #idx => {
              let __c = #content_fn(&value);
              let __arr: ::js_sys::Array = __c.try_into()
                .map_err(|_| ::typewire::Error::UnexpectedType { expected: "array" })?;
              #(#bindings)*
              Ok(#name::#vname(#(#vars),*))
            }
          }
        }
        VariantKind::Named(fields) => {
          let field_bindings = named_fields_from_destruct_arr(fields);
          let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();
          let has_active = fields.iter().any(|f| !f.flags.contains(FieldFlags::SKIP_DE));
          let destruct_call = if has_active {
            let destruct_fn = format_ident!("__tw_{type_name}_destruct_{vname}");
            quote! {
              let __content = #content_fn(&value);
              let __arr = #destruct_fn(&__content);
            }
          } else {
            quote! {}
          };
          quote! {
            #idx => {
              #destruct_call
              #(#field_bindings)*
              Ok(#name::#vname { #(#field_names,)* })
            }
          }
        }
      }
    })
    .collect();

  if has_fallbacks {
    let final_err = if other_fallback.is_empty() {
      quote! { Err(::typewire::Error::NoMatchingVariant) }
    } else {
      other_fallback
    };

    quote! {
      let __idx = #dispatch_fn(&value);
      match __idx {
        #(#dispatch_arms)*
        _ => {}
      }
      #(#untagged_fallbacks)*
      #final_err
    }
  } else {
    quote! {
      match #dispatch_fn(&value) {
        #(#dispatch_arms)*
        _ => {
          // Reflect::get: error-path only — reads the tag string to produce a
          // descriptive `UnknownVariant` / `MissingField` error message.
          let __tag_str = ::js_sys::Reflect::get(
            &value,
            &::wasm_bindgen::JsValue::from_str(#tag),
          )
          .ok()
          .and_then(|v| v.as_string());
          match __tag_str {
            Some(t) => Err(::typewire::Error::UnknownVariant { variant: t }),
            None => Err(::typewire::Error::MissingField { field: #tag }),
          }
        }
      }
    }
  }
}

fn adj_tagged_patch_js(e: &SchemaEnum, content_key: &str) -> TokenStream {
  let name = &e.ident;
  let type_name = e.ident.to_string();
  let dispatch_fn = format_ident!("__tw_{type_name}_dispatch");
  let content_fn = format_ident!("__tw_{type_name}_content");

  let tagged: Vec<&SchemaVariant> = e
    .variants
    .iter()
    .filter(|v| {
      !v.flags.contains(VariantFlags::SKIP_DE)
        && !v.flags.contains(VariantFlags::UNTAGGED)
        && !v.flags.contains(VariantFlags::OTHER)
    })
    .collect();

  let arms: Vec<_> = e
    .variants
    .iter()
    .filter_map(|v| {
      if v.flags.contains(VariantFlags::SKIP_SER) {
        return None;
      }
      let vname = &v.ident;
      let expected_idx = tagged.iter().position(|tv| tv.ident == v.ident);
      #[expect(
        clippy::cast_possible_truncation,
        clippy::cast_possible_wrap,
        reason = "variant count always fits i32"
      )]
      let cmp = expected_idx.map_or_else(
        || quote! { true },
        |i| {
          let idx = i as i32;
          quote! { __old_idx != #idx }
        },
      );

      match &v.kind {
        VariantKind::Unit => Some(quote! {
          #name::#vname => {
            if #cmp { _set(self.to_js()); }
          }
        }),
        VariantKind::Named(fields) => {
          let binds = field_binds(fields);
          let destruct_fn = format_ident!("__tw_{type_name}_destruct_{vname}");
          let active: Vec<&SchemaField> = fields
            .iter()
            .filter(|f| {
              !(f.flags.contains(FieldFlags::SKIP_SER) && f.flags.contains(FieldFlags::SKIP_DE))
            })
            .collect();
          let mut arr_idx: u32 = 0;
          let patches: Vec<TokenStream> = active
            .iter()
            .map(|f| {
              let ident = &f.ident;
              if f.flags.contains(FieldFlags::FLATTEN) {
                let idx = arr_idx;
                arr_idx += 1;
                return quote! {
                  ::typewire::Typewire::patch_js(#ident, &__varr.get(#idx), |_| {});
                };
              }
              let idx = arr_idx;
              arr_idx += 1;
              let ident_ts = quote! { #ident };
              let to_js = field_to_js_expr(&ident_ts, f);
              let setter_fn = format_ident!("__tw_{type_name}_set_{vname}_{ident}");
              let is_special = f
                .flags
                .intersects(FieldFlags::BASE64 | FieldFlags::DISPLAY | FieldFlags::SERDE_BYTES);
              if is_special {
                quote! {
                  {
                    let __old_v = __varr.get(#idx);
                    let __new_v = #to_js;
                    if __old_v != __new_v { #setter_fn(&__content, __new_v); }
                  }
                }
              } else {
                quote! {
                  ::typewire::Typewire::patch_js(
                    #ident_ts,
                    &__varr.get(#idx),
                    |v| #setter_fn(&__content, v),
                  );
                }
              }
            })
            .collect();

          Some(quote! {
            #name::#vname { #(#binds,)* .. } => {
              if #cmp {
                _set(self.to_js());
              } else {
                let __content = #content_fn(old);
                let __varr = #destruct_fn(&__content);
                #(#patches)*
              }
            }
          })
        }
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          Some(quote! {
            #name::#vname(__inner) => {
              if #cmp {
                _set(self.to_js());
              } else {
                let __old_content = #content_fn(old);
                // Reflect::set: updates the content field in-place on the
                // existing wrapper object when the inner value changes.
                // A JS setter helper is not generated for this single-use
                // write in the `patch_js` callback path.
                <#ty as ::typewire::Typewire>::patch_js(__inner, &__old_content, |v| {
                  let _ = ::js_sys::Reflect::set(
                    old,
                    &::wasm_bindgen::JsValue::from_str(#content_key),
                    &v,
                  );
                });
              }
            }
          })
        }
        VariantKind::Unnamed(_) => Some(quote! {
          #name::#vname(..) => { _set(self.to_js()); }
        }),
      }
    })
    .collect();

  quote! {
    fn patch_js(&self, old: &::wasm_bindgen::JsValue, _set: impl FnOnce(::wasm_bindgen::JsValue)) {
      if old.is_undefined() || old.is_null() {
        _set(self.to_js());
        return;
      }
      let __old_idx = #dispatch_fn(old);
      match self {
        #(#arms)*
        #[expect(unreachable_patterns, reason = "wasm-only variants may be skipped")]
        _ => { _set(self.to_js()); }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Untagged: try each variant in order
// ---------------------------------------------------------------------------

fn untagged_to_js_arms(e: &SchemaEnum) -> impl Iterator<Item = TokenStream> + '_ {
  let name = &e.ident;
  let type_name = e.ident.to_string();
  e.variants.iter().filter_map(move |v| {
    if v.flags.contains(VariantFlags::SKIP_SER) {
      return None;
    }
    let vname = &v.ident;

    match &v.kind {
      VariantKind::Unit => Some(quote! {
        #name::#vname => ::wasm_bindgen::JsValue::NULL,
      }),
      VariantKind::Unnamed(types) if types.len() == 1 => Some(quote! {
        #name::#vname(__inner) => ::typewire::Typewire::to_js(__inner),
      }),
      VariantKind::Unnamed(types) => {
        let binds: Vec<_> = (0..types.len()).map(|i| format_ident!("__f{i}")).collect();
        let pushes = binds.iter().map(|b| quote! { arr.push(&::typewire::Typewire::to_js(#b)); });
        Some(quote! {
          #name::#vname(#(#binds),*) => {
            let arr = ::js_sys::Array::new();
            #(#pushes)*
            arr.into()
          }
        })
      }
      VariantKind::Named(fields) => {
        let (binds, args) = variant_to_js_args(fields);
        let construct_fn = format_ident!("__tw_{type_name}_construct_{vname}");
        Some(quote! {
          #name::#vname { #(#binds,)* .. } => #construct_fn(#(#args),*).into(),
        })
      }
    }
  })
}

fn untagged_to_js(e: &SchemaEnum) -> TokenStream {
  let to_js_arms = untagged_to_js_arms(e);
  quote! {
    match self {
      #(#to_js_arms)*
      #[cfg_attr(
        not(target_arch = "wasm32"),
        expect(unreachable_patterns, reason = "wasm-only variants may be skipped")
      )]
      _ => ::wasm_bindgen::JsValue::UNDEFINED,
    }
  }
}

fn untagged_from_js(e: &SchemaEnum) -> TokenStream {
  let name = &e.ident;
  let type_name = e.ident.to_string();

  let from_js_attempts: Vec<_> = e
    .variants
    .iter()
    .filter(|v| !v.flags.contains(VariantFlags::SKIP_DE))
    .map(|v| {
      let vname = &v.ident;

      match &v.kind {
        VariantKind::Unit => quote! {
          if value.is_null() || value.is_undefined() {
            return Ok(#name::#vname);
          }
        },
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          quote! {
            if let Ok(v) = <#ty as ::typewire::Typewire>::from_js(value.clone()) {
              return Ok(#name::#vname(v));
            }
          }
        }
        VariantKind::Unnamed(types) => {
          let bindings = types.iter().enumerate().map(|(i, ty)| {
            let var = format_ident!("__f{i}");
            #[expect(clippy::cast_possible_truncation, reason = "field index always fits u32")]
            let idx = i as u32;
            quote! { let #var = <#ty as ::typewire::Typewire>::from_js(arr.get(#idx))?; }
          });
          let vars: Vec<_> = (0..types.len()).map(|i| format_ident!("__f{i}")).collect();
          #[expect(
            clippy::cast_possible_truncation,
            reason = "variant field count always fits u32"
          )]
          let len_u32 = types.len() as u32;
          quote! {
            if let Ok(arr) = ::core::convert::TryInto::<::js_sys::Array>::try_into(value.clone()) {
              if arr.length() == #len_u32 {
                if let Ok(v) = (|| -> ::core::result::Result<#name, ::typewire::Error> {
                  #(#bindings)*
                  Ok(#name::#vname(#(#vars),*))
                })() {
                  return Ok(v);
                }
              }
            }
          }
        }
        VariantKind::Named(fields) => {
          let destruct_fn = format_ident!("__tw_{type_name}_destruct_{vname}");
          let field_bindings = named_fields_from_destruct_arr(fields);
          let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();
          let has_active = fields.iter().any(|f| {
            !(f.flags.contains(FieldFlags::SKIP_SER) && f.flags.contains(FieldFlags::SKIP_DE))
          });
          let destruct_call = if has_active {
            quote! { let __arr = #destruct_fn(&value); }
          } else {
            quote! {}
          };
          quote! {
            if let Ok(v) = (|| -> ::core::result::Result<#name, ::typewire::Error> {
              #destruct_call
              #(#field_bindings)*
              Ok(#name::#vname { #(#field_names,)* })
            })() {
              return Ok(v);
            }
          }
        }
      }
    })
    .collect();

  quote! {
    #(#from_js_attempts)*
    Err(::typewire::Error::NoMatchingVariant)
  }
}

// ---------------------------------------------------------------------------
// Field codegen helpers
// ---------------------------------------------------------------------------

/// Generate the `to_js` expression for a field value.
fn field_to_js_expr(ident: &TokenStream, f: &SchemaField) -> TokenStream {
  if f.flags.contains(FieldFlags::BASE64) {
    quote! {
      ::wasm_bindgen::JsValue::from_str(
        &::typewire::base64_encode(#ident)
      )
    }
  } else if f.flags.contains(FieldFlags::DISPLAY) {
    quote! {
      ::wasm_bindgen::JsValue::from_str(&::std::string::ToString::to_string(#ident))
    }
  } else if f.flags.contains(FieldFlags::SERDE_BYTES) {
    quote! {
      ::js_sys::Uint8Array::from((#ident).as_slice()).into()
    }
  } else {
    quote! { ::typewire::Typewire::to_js(#ident) }
  }
}

/// Generate the `from_js` expression for a field. Returns a `Result<T, Error>`
/// expression that ends with `?`. Variable `v` must be in scope.
fn field_from_js_expr(f: &SchemaField) -> TokenStream {
  let ty = &f.ty;
  let context = &f.wire_name;
  let js_key = &f.wire_name;

  if f.flags.contains(FieldFlags::BASE64) {
    quote! {
      (|| -> ::core::result::Result<_, ::typewire::Error> {
        let s = v.as_string()
          .ok_or(::typewire::Error::UnexpectedType { expected: "string" })?;
        ::typewire::base64_decode(&s)
          .map_err(|e| ::typewire::Error::InvalidValue {
            message: e.to_string(),
          })
      })().map_err(|e| e.in_context(#context))?
    }
  } else if f.flags.contains(FieldFlags::DISPLAY) {
    quote! {
      (|| -> ::core::result::Result<_, ::typewire::Error> {
        let s = v.as_string()
          .ok_or(::typewire::Error::UnexpectedType { expected: "string" })?;
        s.parse::<#ty>()
          .map_err(|e| ::typewire::Error::InvalidValue {
            message: ::std::string::ToString::to_string(&e),
          })
      })().map_err(|e| e.in_context(#context))?
    }
  } else if f.flags.contains(FieldFlags::SERDE_BYTES) {
    quote! {
      (|| -> ::core::result::Result<_, ::typewire::Error> {
        use ::wasm_bindgen::JsCast as _;
        if let Some(arr) = v.dyn_ref::<::js_sys::Uint8Array>() {
          Ok(arr.to_vec())
        } else if let Some(arr) = v.dyn_ref::<::js_sys::Uint8ClampedArray>() {
          Ok(arr.to_vec())
        } else {
          Err(::typewire::Error::UnexpectedType {
            expected: "Uint8Array or Uint8ClampedArray",
          })
        }
      })().map_err(|e| e.in_context(#context))?
    }
  } else if f.flags.contains(FieldFlags::LENIENT) {
    quote! {
      <#ty as ::typewire::Typewire>::from_js_lenient(v, #js_key)
        .map_err(|e| e.in_context(#context))?
    }
  } else {
    quote! {
      <#ty as ::typewire::Typewire>::from_js(v)
        .map_err(|e| e.in_context(#context))?
    }
  }
}

/// Generate `let field = ...;` bindings from a destruct array (`__arr`).
/// Uses the same pattern as `struct_from_js_body`'s Named case: array indices
/// map 1:1 to active fields (not both-skip), with `SKIP_DE`/`FLATTEN`/default handling.
fn named_fields_from_destruct_arr(fields: &[SchemaField]) -> Vec<TokenStream> {
  let mut arr_idx: u32 = 0;
  fields
    .iter()
    .map(|f| {
      let ident = &f.ident;

      // Fully skipped fields (both SKIP_SER and SKIP_DE) are not in the
      // destruct array -- just use their default.
      if f.flags.contains(FieldFlags::SKIP_SER) && f.flags.contains(FieldFlags::SKIP_DE) {
        let default_expr = default_expr_for_field(f);
        return quote! { let #ident = #default_expr; };
      }

      // SKIP_DE fields are in the destruct array (for patch_js) but
      // from_js ignores them and uses the default value.
      if f.flags.contains(FieldFlags::SKIP_DE) {
        arr_idx += 1; // consume the array slot
        let default_expr = default_expr_for_field(f);
        return quote! { let #ident = #default_expr; };
      }

      // FLATTEN fields get the whole parent object from destruct.
      if f.flags.contains(FieldFlags::FLATTEN) {
        let ty = &f.ty;
        let field_str = ident.to_string();
        let idx = arr_idx;
        arr_idx += 1;
        return quote! {
          let #ident = <#ty as ::typewire::Typewire>::from_js(__arr.get(#idx))
            .map_err(|e| e.in_context(#field_str))?;
        };
      }

      let idx = arr_idx;
      arr_idx += 1;
      let from_js = field_from_js_expr(f);
      let js_key = &f.wire_name;
      let ty = &f.ty;
      let has_default = !matches!(f.default, SchemaFieldDefault::None);

      if has_default {
        let default_expr = default_expr_for_field(f);
        quote! {
          let #ident = {
            let v = __arr.get(#idx);
            if !v.is_undefined() && !v.is_null() {
              #from_js
            } else {
              #default_expr
            }
          };
        }
      } else {
        quote! {
          let #ident = {
            let v = __arr.get(#idx);
            if !v.is_undefined() {
              #from_js
            } else {
              match <#ty as ::typewire::Typewire>::or_default() {
                Some(d) => d,
                None => return Err(::typewire::Error::MissingField { field: #js_key }),
              }
            }
          };
        }
      }
    })
    .collect()
}

/// Generate bind patterns and `to_js` argument expressions for named fields.
/// Used by enum variant `to_js` arms that call a JS construct helper.
/// Returns (`bind_patterns`, `arg_expressions`).
fn variant_to_js_args(fields: &[SchemaField]) -> (Vec<TokenStream>, Vec<TokenStream>) {
  let mut binds = Vec::new();
  let mut args = Vec::new();

  for f in fields {
    if f.flags.contains(FieldFlags::SKIP_SER) {
      continue;
    }
    let ident = &f.ident;
    binds.push(quote! { #ident });

    let ident_ts = quote! { #ident };
    let to_js = field_to_js_expr(&ident_ts, f);

    let arg = if let Some(ref pred_path) = f.skip_serializing_if {
      quote! {
        if #pred_path(#ident) {
          ::wasm_bindgen::JsValue::UNDEFINED
        } else {
          #to_js
        }
      }
    } else {
      to_js
    };
    args.push(arg);
  }

  (binds, args)
}

/// Generate `let field = ...;` bindings that read from `__obj` (a `&JsValue`).
fn named_field_bindings(fields: &[SchemaField]) -> Vec<TokenStream> {
  fields
    .iter()
    .map(|f| {
      let ident = &f.ident;

      if f.flags.contains(FieldFlags::SKIP_DE) {
        let default_expr = default_expr_for_field(f);
        return quote! { let #ident = #default_expr; };
      }

      if f.flags.contains(FieldFlags::FLATTEN) {
        let ty = &f.ty;
        let field_str = ident.to_string();
        return quote! {
          let #ident = <#ty as ::typewire::Typewire>::from_js((*__obj).clone())
            .map_err(|e| e.in_context(#field_str))?;
        };
      }

      let js_key = &f.wire_name;
      let ty = &f.ty;

      // Reflect::get: used only for per-variant `#[serde(untagged)]` fallbacks
      // within tagged enums. These variants are tried speculatively and have no
      // JS destruct helpers — they may not match the actual JS shape at all.
      let get_value = if f.aliases.is_empty() {
        quote! {
          ::js_sys::Reflect::get(__obj, &::wasm_bindgen::JsValue::from_str(#js_key))
            .ok()
        }
      } else {
        let all_keys: Vec<&str> = std::iter::once(f.wire_name.as_str())
          .chain(f.aliases.iter().map(std::string::String::as_str))
          .collect();
        quote! {
          {
            let mut __val = None;
            #(
              if __val.is_none() {
                __val = ::js_sys::Reflect::get(
                  __obj,
                  &::wasm_bindgen::JsValue::from_str(#all_keys),
                )
                .ok()
                .filter(|v| !v.is_undefined());
              }
            )*
            __val
          }
        }
      };

      let has_default = !matches!(f.default, SchemaFieldDefault::None);
      let from_js = field_from_js_expr(f);

      if has_default {
        let default_expr = default_expr_for_field(f);
        quote! {
          let #ident = match #get_value {
            Some(v) if !v.is_undefined() && !v.is_null() => {
              #from_js
            }
            _ => #default_expr,
          };
        }
      } else {
        quote! {
          let #ident = match #get_value {
            Some(v) if !v.is_undefined() => {
              #from_js
            }
            _ => match <#ty as ::typewire::Typewire>::or_default() {
              Some(d) => d,
              None => return Err(::typewire::Error::MissingField { field: #js_key }),
            },
          };
        }
      }
    })
    .collect()
}

/// Construct a variant from named fields read from a JS object.
fn named_fields_from_js_obj(
  enum_name: &Ident,
  vname: &Ident,
  fields: &[SchemaField],
) -> TokenStream {
  let field_bindings = named_field_bindings(fields);
  let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();
  quote! {
    let __obj = &value;
    #(#field_bindings)*
    Ok(#enum_name::#vname { #(#field_names,)* })
  }
}

/// Generate field name bindings for a named-field enum pattern.
fn field_binds(fields: &[SchemaField]) -> Vec<TokenStream> {
  fields
    .iter()
    .map(|f| {
      let ident = &f.ident;
      quote! { #ident }
    })
    .collect()
}

// ---------------------------------------------------------------------------
// Enum fallback helpers
// ---------------------------------------------------------------------------

/// Generate try-each-variant fallback code for `#[serde(untagged)]` variants
/// within a tagged enum.
fn untagged_variant_fallbacks(name: &Ident, variants: &[SchemaVariant]) -> Vec<TokenStream> {
  variants
    .iter()
    .filter(|v| {
      v.flags.contains(VariantFlags::UNTAGGED) && !v.flags.contains(VariantFlags::SKIP_DE)
    })
    .map(|v| {
      let vname = &v.ident;

      match &v.kind {
        VariantKind::Unit => quote! {
          if value.is_null() || value.is_undefined() {
            return Ok(#name::#vname);
          }
        },
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          quote! {
            if let Ok(v) = <#ty as ::typewire::Typewire>::from_js(value.clone()) {
              return Ok(#name::#vname(v));
            }
          }
        }
        VariantKind::Unnamed(types) => {
          #[expect(
            clippy::cast_possible_truncation,
            reason = "variant field count always fits u32"
          )]
          let len = types.len() as u32;
          let bindings = types.iter().enumerate().map(|(i, ty)| {
            let var = format_ident!("__f{i}");
            #[expect(clippy::cast_possible_truncation, reason = "field index always fits u32")]
            let idx = i as u32;
            quote! { let #var = <#ty as ::typewire::Typewire>::from_js(arr.get(#idx))?; }
          });
          let vars: Vec<_> = (0..types.len()).map(|i| format_ident!("__f{i}")).collect();
          quote! {
            if let Ok(arr) = ::core::convert::TryInto::<::js_sys::Array>::try_into(value.clone()) {
              if arr.length() == #len {
                if let Ok(v) = (|| -> ::core::result::Result<#name, ::typewire::Error> {
                  #(#bindings)*
                  Ok(#name::#vname(#(#vars),*))
                })() {
                  return Ok(v);
                }
              }
            }
          }
        }
        VariantKind::Named(fields) => {
          let body = named_fields_from_js_obj(name, vname, fields);
          quote! {
            if let Ok(v) = (|| -> ::core::result::Result<#name, ::typewire::Error> {
              #body
            })() {
              return Ok(v);
            }
          }
        }
      }
    })
    .collect()
}

/// Generate the `#[serde(other)]` catch-all return, if any variant has it.
fn other_variant_fallback(name: &Ident, variants: &[SchemaVariant]) -> TokenStream {
  for v in variants {
    if v.flags.contains(VariantFlags::OTHER) && !v.flags.contains(VariantFlags::SKIP_DE) {
      let vname = &v.ident;
      return quote! { Ok(#name::#vname) };
    }
  }
  quote! {}
}

// ---------------------------------------------------------------------------
// Default expression
// ---------------------------------------------------------------------------

/// Produce a default-value expression for a field.
fn default_expr_for_field(f: &SchemaField) -> TokenStream {
  match &f.default {
    SchemaFieldDefault::Path(path) => quote! { #path() },
    SchemaFieldDefault::Default | SchemaFieldDefault::None => {
      quote! { ::core::default::Default::default() }
    }
  }
}

// ---------------------------------------------------------------------------
// JS bindings generation (`#[wasm_bindgen(inline_js)]` + `extern "C"`)
// ---------------------------------------------------------------------------

/// Dispatch JS bindings generation based on schema shape.
fn js_bindings_for_schema(schema: &typewire_schema::Schema) -> TokenStream {
  use typewire_schema::Schema;
  match schema {
    Schema::Struct(s) => struct_js_bindings(s),
    Schema::Enum(e) => enum_js_bindings(e),
    Schema::FromProxy(p) => match &p.own_shape {
      TypeShape::Struct(s) => struct_js_bindings(s),
      TypeShape::Enum(e) => enum_js_bindings(e),
    },
    // Transparent, IntoProxy, and proxy-with-proxy shapes delegate to
    // inner types which generate their own bindings.
    _ => TokenStream::new(),
  }
}

/// Generate `#[wasm_bindgen(inline_js)]` + `extern "C"` for a named struct.
fn struct_js_bindings(s: &SchemaStruct) -> TokenStream {
  let StructShape::Named(fields) = &s.shape else {
    return TokenStream::new();
  };

  let type_name = s.ident.to_string();

  // Active fields: participate in at least one of to_js/from_js/patch_js.
  let active: Vec<&SchemaField> = fields
    .iter()
    .filter(|f| !(f.flags.contains(FieldFlags::SKIP_SER) && f.flags.contains(FieldFlags::SKIP_DE)))
    .collect();

  if active.is_empty() {
    return TokenStream::new();
  }

  let mut js = String::new();
  let mut extern_fns: Vec<TokenStream> = Vec::new();

  // -- destruct: returns array of active field values --
  js_destruct(&mut js, &mut extern_fns, &type_name, &active);

  // -- construct: params for non-SKIP_SER active fields --
  js_construct(&mut js, &mut extern_fns, &type_name, &active);

  // -- per-field setters (for patch_js) --
  js_setters(&mut js, &mut extern_fns, &type_name, &active);

  // -- check_keys (deny_unknown_fields) --
  if s.flags.contains(StructFlags::DENY_UNKNOWN_FIELDS) {
    js_check_keys(&mut js, &mut extern_fns, &type_name, fields);
  }

  let js_lit = proc_macro2::Literal::string(&js);
  quote! {
    #[cfg(target_arch = "wasm32")]
    #[::wasm_bindgen::prelude::wasm_bindgen(inline_js = #js_lit)]
    unsafe extern "C" {
      #(#extern_fns)*
    }
  }
}

/// Generate `#[wasm_bindgen(inline_js)]` + `extern "C"` for an enum.
fn enum_js_bindings(e: &SchemaEnum) -> TokenStream {
  // All-unit enums use the string fast path — no JS helpers needed.
  if e.flags.contains(EnumFlags::ALL_UNIT) {
    return TokenStream::new();
  }

  let type_name = e.ident.to_string();
  let mut js = String::new();
  let mut extern_fns: Vec<TokenStream> = Vec::new();

  // Serializable variants (for to_js).
  let ser_variants: Vec<&SchemaVariant> =
    e.variants.iter().filter(|v| !v.flags.contains(VariantFlags::SKIP_SER)).collect();

  // Untagged enums don't need dispatch or content helpers, but still need
  // per-variant construct and destruct for Named-field variants.
  if matches!(e.tagging, Tagging::Untagged) {
    // -- per-variant construct (for to_js of Named variants) --
    js_enum_variant_constructs(&mut js, &mut extern_fns, &type_name, &ser_variants, &e.tagging);

    // Deserializable variants with named fields need destruct helpers.
    let de_variants: Vec<&SchemaVariant> =
      e.variants.iter().filter(|v| !v.flags.contains(VariantFlags::SKIP_DE)).collect();
    js_enum_variant_destructs(&mut js, &mut extern_fns, &type_name, &de_variants);
  } else {
    // Variants that participate in tagged dispatch (not skip_de, not untagged, not other).
    let tagged_variants: Vec<&SchemaVariant> = e
      .variants
      .iter()
      .filter(|v| {
        !v.flags.contains(VariantFlags::SKIP_DE)
          && !v.flags.contains(VariantFlags::UNTAGGED)
          && !v.flags.contains(VariantFlags::OTHER)
      })
      .collect();

    // -- dispatch: returns i32 index --
    js_enum_dispatch(&mut js, &mut extern_fns, &type_name, &tagged_variants, &e.tagging);

    // -- content extraction (adjacent / external) --
    js_enum_content(&mut js, &mut extern_fns, &type_name, &tagged_variants, &e.tagging);

    // -- per-variant construct (for to_js) --
    js_enum_variant_constructs(&mut js, &mut extern_fns, &type_name, &ser_variants, &e.tagging);

    // -- per-variant destruct + setters (for from_js / patch_js of named variants) --
    js_enum_variant_destructs(&mut js, &mut extern_fns, &type_name, &tagged_variants);
  }

  if js.is_empty() {
    return TokenStream::new();
  }

  let js_lit = proc_macro2::Literal::string(&js);
  quote! {
    #[cfg(target_arch = "wasm32")]
    #[::wasm_bindgen::prelude::wasm_bindgen(inline_js = #js_lit)]
    unsafe extern "C" {
      #(#extern_fns)*
    }
  }
}

/// Generate dispatch function that returns the variant index as `i32`.
/// Uses `switch` to support variant aliases.
fn js_enum_dispatch(
  js: &mut String,
  extern_fns: &mut Vec<TokenStream>,
  type_name: &str,
  variants: &[&SchemaVariant],
  tagging: &Tagging,
) {
  use std::fmt::Write;
  let fn_name = format!("__tw_{type_name}_dispatch");

  match tagging {
    Tagging::Internal { tag } | Tagging::Adjacent { tag, .. } => {
      write!(js, "export function {fn_name}(v){{switch(v[\"{tag}\"]){{").unwrap();
      for (i, v) in variants.iter().enumerate() {
        for n in &v.all_wire_names {
          write!(js, "case\"{n}\":").unwrap();
        }
        write!(js, "return {i};").unwrap();
      }
      writeln!(js, "default:return -1}}}}").unwrap();
    }
    Tagging::External => {
      write!(js, "export function {fn_name}(v){{").unwrap();
      write!(js, "if(typeof v===\"string\"){{switch(v){{").unwrap();
      for (i, v) in variants.iter().enumerate() {
        for n in &v.all_wire_names {
          write!(js, "case\"{n}\":").unwrap();
        }
        write!(js, "return {i};").unwrap();
      }
      write!(js, "default:return -1}}}}").unwrap();
      for (i, v) in variants.iter().enumerate() {
        let checks =
          v.all_wire_names.iter().map(|n| format!("\"{n}\"in v")).collect::<Vec<_>>().join("||");
        write!(js, "if({checks})return {i};").unwrap();
      }
      writeln!(js, "return -1}}").unwrap();
    }
    Tagging::Untagged => {} // unreachable — filtered above
  }

  let fn_ident = format_ident!("{fn_name}");
  extern_fns.push(quote! {
    fn #fn_ident(v: &::wasm_bindgen::JsValue) -> i32;
  });
}

/// Generate content extraction functions.
fn js_enum_content(
  js: &mut String,
  extern_fns: &mut Vec<TokenStream>,
  type_name: &str,
  variants: &[&SchemaVariant],
  tagging: &Tagging,
) {
  use std::fmt::Write;
  match tagging {
    Tagging::Adjacent { content, .. } => {
      // Single content function: returns v["content_key"]
      let fn_name = format!("__tw_{type_name}_content");
      writeln!(js, "export function {fn_name}(v){{return v[\"{content}\"]}}").unwrap();
      let fn_ident = format_ident!("{fn_name}");
      extern_fns.push(quote! {
        fn #fn_ident(v: &::wasm_bindgen::JsValue) -> ::wasm_bindgen::JsValue;
      });
    }
    Tagging::External => {
      // Per-variant content with alias fallback: v["Name"] ?? v["alias"]
      for v in variants {
        if matches!(v.kind, VariantKind::Unit) {
          continue;
        }
        let vname = &v.ident;
        let fn_name = format!("__tw_{type_name}_content_{vname}");
        let access =
          v.all_wire_names.iter().map(|n| format!("v[\"{n}\"]")).collect::<Vec<_>>().join("??");
        writeln!(js, "export function {fn_name}(v){{return {access}}}").unwrap();
        let fn_ident = format_ident!("{fn_name}");
        extern_fns.push(quote! {
          fn #fn_ident(v: &::wasm_bindgen::JsValue) -> ::wasm_bindgen::JsValue;
        });
      }
    }
    _ => {} // Internal tagging: fields are on the same object, no content extraction
  }
}

/// Generate per-variant construct functions for `to_js`.
fn js_enum_variant_constructs(
  js: &mut String,
  extern_fns: &mut Vec<TokenStream>,
  type_name: &str,
  variants: &[&SchemaVariant],
  tagging: &Tagging,
) {
  use std::fmt::Write;
  for v in variants {
    let vname = &v.ident;
    let fn_name = format!("__tw_{type_name}_construct_{vname}");

    match tagging {
      Tagging::Internal { tag } => {
        let tag_val = &v.wire_name;
        match &v.kind {
          VariantKind::Unit => {
            writeln!(js, "export function {fn_name}(){{return{{\"{tag}\":\"{tag_val}\"}}}}")
              .unwrap();
            let fn_ident = format_ident!("{fn_name}");
            extern_fns.push(quote! { fn #fn_ident() -> ::js_sys::Object; });
          }
          VariantKind::Named(fields) => {
            let ser_fields: Vec<&SchemaField> =
              fields.iter().filter(|f| !f.flags.contains(FieldFlags::SKIP_SER)).collect();
            let params =
              (0..ser_fields.len()).map(|i| format!("p{i}")).collect::<Vec<_>>().join(",");
            write!(js, "export function {fn_name}({params}){{const o={{\"{tag}\":\"{tag_val}\"}};")
              .unwrap();
            for (i, f) in ser_fields.iter().enumerate() {
              if f.flags.contains(FieldFlags::FLATTEN) {
                write!(js, "Object.assign(o,p{i});").unwrap();
              } else if f.skip_serializing_if.is_some() {
                write!(js, "if(p{i}!==undefined)o[\"{}\"]=p{i};", f.wire_name).unwrap();
              } else {
                write!(js, "o[\"{}\"]=p{i};", f.wire_name).unwrap();
              }
            }
            writeln!(js, "return o}}").unwrap();
            let fn_ident = format_ident!("{fn_name}");
            let param_idents: Vec<Ident> =
              (0..ser_fields.len()).map(|i| format_ident!("p{i}")).collect();
            extern_fns.push(quote! {
              fn #fn_ident(#(#param_idents: ::wasm_bindgen::JsValue),*) -> ::js_sys::Object;
            });
          }
          VariantKind::Unnamed(types) if types.len() == 1 => {
            // Internally tagged single newtype: inject tag into inner's to_js result
            // This is special — can't just construct, need to set tag on existing obj.
            // Keep as-is (no construct helper for this case).
          }
          VariantKind::Unnamed(_) => {} // Multi-field tuple + internal tag is rejected by analyze
        }
      }
      Tagging::Adjacent { tag, content } => {
        let tag_val = &v.wire_name;
        match &v.kind {
          VariantKind::Unit => {
            writeln!(js, "export function {fn_name}(){{return{{\"{tag}\":\"{tag_val}\"}}}}")
              .unwrap();
            let fn_ident = format_ident!("{fn_name}");
            extern_fns.push(quote! { fn #fn_ident() -> ::js_sys::Object; });
          }
          VariantKind::Named(fields) => {
            // Takes individual field params, builds { tag: "V", content: { f1: p0, ... } }
            let ser_fields: Vec<&SchemaField> =
              fields.iter().filter(|f| !f.flags.contains(FieldFlags::SKIP_SER)).collect();
            let params =
              (0..ser_fields.len()).map(|i| format!("p{i}")).collect::<Vec<_>>().join(",");
            write!(js, "export function {fn_name}({params}){{const o={{}};").unwrap();
            for (i, f) in ser_fields.iter().enumerate() {
              if f.flags.contains(FieldFlags::FLATTEN) {
                write!(js, "Object.assign(o,p{i});").unwrap();
              } else if f.skip_serializing_if.is_some() {
                write!(js, "if(p{i}!==undefined)o[\"{}\"]=p{i};", f.wire_name).unwrap();
              } else {
                write!(js, "o[\"{}\"]=p{i};", f.wire_name).unwrap();
              }
            }
            writeln!(js, "return{{\"{tag}\":\"{tag_val}\",\"{content}\":o}}}}").unwrap();
            let fn_ident = format_ident!("{fn_name}");
            let param_idents: Vec<Ident> =
              (0..ser_fields.len()).map(|i| format_ident!("p{i}")).collect();
            extern_fns.push(quote! {
              fn #fn_ident(#(#param_idents: ::wasm_bindgen::JsValue),*) -> ::js_sys::Object;
            });
          }
          VariantKind::Unnamed(_) => {
            // Unnamed: takes single content value
            writeln!(
              js,
              "export function {fn_name}(c){{return{{\"{tag}\":\"{tag_val}\",\"{content}\":c}}}}"
            )
            .unwrap();
            let fn_ident = format_ident!("{fn_name}");
            extern_fns.push(quote! {
              fn #fn_ident(c: ::wasm_bindgen::JsValue) -> ::js_sys::Object;
            });
          }
        }
      }
      Tagging::External => {
        let tag_val = &v.wire_name;
        match &v.kind {
          VariantKind::Unit => {} // string, no construct needed
          VariantKind::Named(fields) => {
            // Takes individual field params, builds { "V": { f1: p0, ... } }
            let ser_fields: Vec<&SchemaField> =
              fields.iter().filter(|f| !f.flags.contains(FieldFlags::SKIP_SER)).collect();
            let params =
              (0..ser_fields.len()).map(|i| format!("p{i}")).collect::<Vec<_>>().join(",");
            write!(js, "export function {fn_name}({params}){{const o={{}};").unwrap();
            for (i, f) in ser_fields.iter().enumerate() {
              if f.flags.contains(FieldFlags::FLATTEN) {
                write!(js, "Object.assign(o,p{i});").unwrap();
              } else if f.skip_serializing_if.is_some() {
                write!(js, "if(p{i}!==undefined)o[\"{}\"]=p{i};", f.wire_name).unwrap();
              } else {
                write!(js, "o[\"{}\"]=p{i};", f.wire_name).unwrap();
              }
            }
            writeln!(js, "return{{\"{tag_val}\":o}}}}").unwrap();
            let fn_ident = format_ident!("{fn_name}");
            let param_idents: Vec<Ident> =
              (0..ser_fields.len()).map(|i| format_ident!("p{i}")).collect();
            extern_fns.push(quote! {
              fn #fn_ident(#(#param_idents: ::wasm_bindgen::JsValue),*) -> ::js_sys::Object;
            });
          }
          VariantKind::Unnamed(_) => {
            // Unnamed: takes single content value
            writeln!(js, "export function {fn_name}(c){{return{{\"{tag_val}\":c}}}}").unwrap();
            let fn_ident = format_ident!("{fn_name}");
            extern_fns.push(quote! {
              fn #fn_ident(c: ::wasm_bindgen::JsValue) -> ::js_sys::Object;
            });
          }
        }
      }
      Tagging::Untagged => {
        // Untagged named: builds the field object with no tag wrapping.
        if let VariantKind::Named(fields) = &v.kind {
          let ser_fields: Vec<&SchemaField> =
            fields.iter().filter(|f| !f.flags.contains(FieldFlags::SKIP_SER)).collect();
          if ser_fields.is_empty() {
            continue;
          }
          let params = (0..ser_fields.len()).map(|i| format!("p{i}")).collect::<Vec<_>>().join(",");
          write!(js, "export function {fn_name}({params}){{const o={{}};").unwrap();
          for (i, f) in ser_fields.iter().enumerate() {
            if f.flags.contains(FieldFlags::FLATTEN) {
              write!(js, "Object.assign(o,p{i});").unwrap();
            } else if f.skip_serializing_if.is_some() {
              write!(js, "if(p{i}!==undefined)o[\"{}\"]=p{i};", f.wire_name).unwrap();
            } else {
              write!(js, "o[\"{}\"]=p{i};", f.wire_name).unwrap();
            }
          }
          writeln!(js, "return o}}").unwrap();
          let fn_ident = format_ident!("{fn_name}");
          let param_idents: Vec<Ident> =
            (0..ser_fields.len()).map(|i| format_ident!("p{i}")).collect();
          extern_fns.push(quote! {
            fn #fn_ident(#(#param_idents: ::wasm_bindgen::JsValue),*) -> ::js_sys::Object;
          });
        }
      }
    }
  }
}

/// Generate per-variant destruct + setter functions for named-field variants.
fn js_enum_variant_destructs(
  js: &mut String,
  extern_fns: &mut Vec<TokenStream>,
  type_name: &str,
  variants: &[&SchemaVariant],
) {
  use std::fmt::Write;
  for v in variants {
    let VariantKind::Named(fields) = &v.kind else {
      continue;
    };
    let vname = &v.ident;

    // Active fields for this variant
    let active: Vec<&SchemaField> = fields
      .iter()
      .filter(|f| {
        !(f.flags.contains(FieldFlags::SKIP_SER) && f.flags.contains(FieldFlags::SKIP_DE))
      })
      .collect();

    if active.is_empty() {
      continue;
    }

    // Destruct
    let destruct_name = format!("__tw_{type_name}_destruct_{vname}");
    write!(js, "export function {destruct_name}(v){{return[").unwrap();
    for (i, f) in active.iter().enumerate() {
      if i > 0 {
        js.push(',');
      }
      if f.flags.contains(FieldFlags::FLATTEN) {
        js.push('v');
      } else {
        write!(js, "v[\"{}\"]", f.wire_name).unwrap();
        for alias in &f.aliases {
          write!(js, "??v[\"{alias}\"]").unwrap();
        }
      }
    }
    writeln!(js, "]}}").unwrap();
    let destruct_ident = format_ident!("{destruct_name}");
    extern_fns.push(quote! {
      fn #destruct_ident(v: &::wasm_bindgen::JsValue) -> ::js_sys::Array;
    });

    // Per-field setters
    for f in &active {
      if f.flags.contains(FieldFlags::FLATTEN) {
        continue;
      }
      let field_ident = &f.ident;
      let setter_name = format!("__tw_{type_name}_set_{vname}_{field_ident}");
      writeln!(js, "export function {setter_name}(o,v){{o[\"{}\"]=v}}", f.wire_name).unwrap();
      let setter_ident = format_ident!("{setter_name}");
      extern_fns.push(quote! {
        fn #setter_ident(o: &::wasm_bindgen::JsValue, v: ::wasm_bindgen::JsValue);
      });
    }
  }
}

// ---------------------------------------------------------------------------
// JS source builders
// ---------------------------------------------------------------------------

/// `export function __tw_{name}_destruct(v){return[v["f1"],v["f2"]??v["alias"],v]}`
fn js_destruct(
  js: &mut String,
  extern_fns: &mut Vec<TokenStream>,
  type_name: &str,
  active: &[&SchemaField],
) {
  use std::fmt::Write;
  let fn_name = format!("__tw_{type_name}_destruct");
  write!(js, "export function {fn_name}(v){{return[").unwrap();
  for (i, f) in active.iter().enumerate() {
    if i > 0 {
      js.push(',');
    }
    if f.flags.contains(FieldFlags::FLATTEN) {
      js.push('v');
    } else {
      write!(js, "v[\"{}\"]", f.wire_name).unwrap();
      for alias in &f.aliases {
        write!(js, "??v[\"{alias}\"]").unwrap();
      }
    }
  }
  js.push_str("]}\n");

  let fn_ident = format_ident!("{fn_name}");
  extern_fns.push(quote! {
    fn #fn_ident(v: &::wasm_bindgen::JsValue) -> ::js_sys::Array;
  });
}

/// `export function __tw_{name}_construct(p0,p1,p2){const o={};o["f1"]=p0;...;return o}`
fn js_construct(
  js: &mut String,
  extern_fns: &mut Vec<TokenStream>,
  type_name: &str,
  active: &[&SchemaField],
) {
  use std::fmt::Write;
  let construct_fields: Vec<&SchemaField> =
    active.iter().filter(|f| !f.flags.contains(FieldFlags::SKIP_SER)).copied().collect();

  if construct_fields.is_empty() {
    return;
  }

  let fn_name = format!("__tw_{type_name}_construct");
  let params: String =
    (0..construct_fields.len()).map(|i| format!("p{i}")).collect::<Vec<_>>().join(",");
  write!(js, "export function {fn_name}({params}){{const o={{}};").unwrap();

  for (i, f) in construct_fields.iter().enumerate() {
    if f.flags.contains(FieldFlags::FLATTEN) {
      write!(js, "Object.assign(o,p{i});").unwrap();
    } else if f.skip_serializing_if.is_some() {
      write!(js, "if(p{i}!==undefined)o[\"{}\"]=p{i};", f.wire_name).unwrap();
    } else {
      write!(js, "o[\"{}\"]=p{i};", f.wire_name).unwrap();
    }
  }
  js.push_str("return o}\n");

  let fn_ident = format_ident!("{fn_name}");
  let param_idents: Vec<Ident> =
    (0..construct_fields.len()).map(|i| format_ident!("p{i}")).collect();
  extern_fns.push(quote! {
    fn #fn_ident(#(#param_idents: ::wasm_bindgen::JsValue),*) -> ::js_sys::Object;
  });
}

/// `export function __tw_{name}_set_{field}(o,v){o["wire_name"]=v}`
fn js_setters(
  js: &mut String,
  extern_fns: &mut Vec<TokenStream>,
  type_name: &str,
  active: &[&SchemaField],
) {
  use std::fmt::Write;
  for f in active {
    if f.flags.contains(FieldFlags::FLATTEN) {
      continue; // flatten fields don't have individual setters
    }
    let field_ident = &f.ident;
    let fn_name = format!("__tw_{type_name}_set_{field_ident}");
    writeln!(js, "export function {fn_name}(o,v){{o[\"{}\"]=v}}", f.wire_name).unwrap();
    let fn_ident = format_ident!("{fn_name}");
    extern_fns.push(quote! {
      fn #fn_ident(o: &::wasm_bindgen::JsValue, v: ::wasm_bindgen::JsValue);
    });
  }
}

/// `export function __tw_{name}_check_keys(v){for(const k of Object.keys(v)){if(k!=="f1"&&k!=="f2")return k}return null}`
fn js_check_keys(
  js: &mut String,
  extern_fns: &mut Vec<TokenStream>,
  type_name: &str,
  fields: &[SchemaField],
) {
  use std::fmt::Write;
  let known: Vec<&str> = fields
    .iter()
    .filter(|f| !f.flags.contains(FieldFlags::SKIP_DE) && !f.flags.contains(FieldFlags::FLATTEN))
    .map(|f| f.wire_name.as_str())
    .collect();

  let fn_name = format!("__tw_{type_name}_check_keys");
  write!(js, "export function {fn_name}(v){{for(const k of Object.keys(v)){{").unwrap();
  if !known.is_empty() {
    js.push_str("if(");
    for (i, key) in known.iter().enumerate() {
      if i > 0 {
        js.push_str("&&");
      }
      write!(js, "k!==\"{key}\"").unwrap();
    }
    js.push_str(")return k");
  }
  js.push_str("}return null}\n");

  let fn_ident = format_ident!("{fn_name}");
  extern_fns.push(quote! {
    fn #fn_ident(v: &::wasm_bindgen::JsValue) -> ::wasm_bindgen::JsValue;
  });
}
