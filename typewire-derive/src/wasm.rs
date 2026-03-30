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
          (|| { #from_js_body })()
            .map_err(|e: ::typewire::Error| e.in_context(#name_str))
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
          (|| { #from_js_body })()
            .map_err(|e: ::typewire::Error| e.in_context(#name_str))
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
}

// ---------------------------------------------------------------------------
// Struct codegen
// ---------------------------------------------------------------------------

fn struct_to_js_body(s: &SchemaStruct) -> TokenStream {
  match &s.shape {
    StructShape::Named(fields) => {
      let setters = fields.iter().filter_map(|f| {
        if f.flags.contains(FieldFlags::SKIP_SER) {
          return None;
        }
        let ident = &f.ident;
        let js_key = &f.wire_name;

        let ident_ts = quote! { &self.#ident };
        let to_js = field_to_js_expr(&ident_ts, f);

        if f.flags.contains(FieldFlags::FLATTEN) {
          return Some(quote! {
            {
              let inner = #to_js;
              if let Some(inner_obj) = inner.dyn_ref::<::js_sys::Object>() {
                let entries = ::js_sys::Object::entries(inner_obj);
                for i in 0..entries.length() {
                  let pair: ::js_sys::Array = entries.get(i).into();
                  let _ = ::js_sys::Reflect::set(&obj, &pair.get(0), &pair.get(1));
                }
              }
            }
          });
        }

        let setter = f.skip_serializing_if.as_ref().map_or_else(
          || {
            quote! {
              let _ = ::js_sys::Reflect::set(
                &obj,
                &::wasm_bindgen::JsValue::from_str(#js_key),
                &#to_js,
              );
            }
          },
          |pred_path| {
            quote! {
              if !#pred_path(&self.#ident) {
                let _ = ::js_sys::Reflect::set(
                  &obj,
                  &::wasm_bindgen::JsValue::from_str(#js_key),
                  &#to_js,
                );
              }
            }
          },
        );

        Some(setter)
      });

      quote! {
        let obj = ::js_sys::Object::new();
        #(#setters)*
        obj.into()
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
      let field_bindings = named_field_bindings(fields);
      let field_names: Vec<_> = fields.iter().map(|f| &f.ident).collect();

      // NOTE: Flattened fields are excluded from the deny check because their
      // sub-keys are not known at this level. This matches serde's behavior:
      // serde also cannot validate unknown keys inside flattened types.
      let deny_check = if s.flags.contains(StructFlags::DENY_UNKNOWN_FIELDS) {
        let known_keys: Vec<&str> = fields
          .iter()
          .filter(|f| {
            !f.flags.contains(FieldFlags::SKIP_DE) && !f.flags.contains(FieldFlags::FLATTEN)
          })
          .map(|f| f.wire_name.as_str())
          .collect();
        quote! {
          if let Some(__obj) = value.dyn_ref::<::js_sys::Object>() {
            let __keys = ::js_sys::Object::keys(__obj);
            for __i in 0..__keys.length() {
              let __k = __keys.get(__i);
              if let Some(ref __key) = __k.as_string() {
                let __known: &[&str] = &[#(#known_keys),*];
                if !__known.contains(&__key.as_str()) {
                  return Err(::typewire::Error::InvalidValue {
                    message: ::std::format!("unknown field `{__key}`"),
                  });
                }
              }
            }
          }
        }
      } else {
        quote! {}
      };

      quote! {
        let __obj = &value;
        #deny_check
        #(#field_bindings)*
        Ok(#name { #(#field_names,)* })
      }
    }
    StructShape::Tuple(types) => {
      let bindings = types.iter().enumerate().map(|(i, ty)| {
        let var = format_ident!("__field{i}");
        let idx = u32::try_from(i).unwrap();
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
      let field_patches = patch_self_fields(fields, &quote! { old });

      quote! {
        fn patch_js(&self, old: &::wasm_bindgen::JsValue, _set: impl FnOnce(::wasm_bindgen::JsValue)) {
          if old.is_undefined() || old.is_null() {
            _set(self.to_js());
            return;
          }
          #(#field_patches)*
        }
      }
    }
    StructShape::Tuple(types) => {
      let patches: Vec<_> = (0..types.len())
        .map(|i| {
          let idx = Index::from(i);
          let js_idx = u32::try_from(i).unwrap();
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
    Tagging::Adjacent { tag, content } => adj_tagged_to_js(e, tag, content),
    Tagging::External => ext_tagged_to_js(e),
  }
}

fn enum_from_js_body(e: &SchemaEnum) -> TokenStream {
  match &e.tagging {
    Tagging::Untagged => untagged_from_js(e),
    Tagging::Internal { tag } => int_tagged_from_js(e, tag),
    Tagging::Adjacent { tag, content } => adj_tagged_from_js(e, tag, content),
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
    Tagging::Internal { tag } => int_tagged_patch_js(e, tag),
    Tagging::Adjacent { tag, content } => adj_tagged_patch_js(e, tag, content),
    Tagging::External => ext_tagged_patch_js(e),
  }
}

// ---------------------------------------------------------------------------
// Externally tagged: `{ "VariantName": <content> }` or `"VariantName"`
// ---------------------------------------------------------------------------

fn ext_tagged_to_js_arms(e: &SchemaEnum) -> impl Iterator<Item = TokenStream> + '_ {
  let name = &e.ident;
  e.variants.iter().filter_map(move |v| {
    if v.flags.contains(VariantFlags::SKIP_SER) {
      return None;
    }

    let tag_str = &v.wire_name;
    let vname = &v.ident;

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
          quote! {
            { let arr = ::js_sys::Array::new(); #(#pushes)* arr.into() }
          }
        };
        Some(quote! {
          #name::#vname(#(#binds),*) => {
            let obj = ::js_sys::Object::new();
            let _ = ::js_sys::Reflect::set(
              &obj,
              &::wasm_bindgen::JsValue::from_str(#tag_str),
              &#content,
            );
            obj.into()
          }
        })
      }
      VariantKind::Named(fields) => {
        let (binds, setters) = named_fields_to_js(fields);
        Some(quote! {
          #name::#vname { #(#binds,)* .. } => {
            let obj = ::js_sys::Object::new();
            #(#setters)*
            let __wrapper = ::js_sys::Object::new();
            let _ = ::js_sys::Reflect::set(
              &__wrapper,
              &::wasm_bindgen::JsValue::from_str(#tag_str),
              &obj,
            );
            __wrapper.into()
          }
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

  let from_js_arms: Vec<_> = e
    .variants
    .iter()
    .filter(|v| {
      !v.flags.contains(VariantFlags::SKIP_DE)
        && !v.flags.contains(VariantFlags::UNTAGGED)
        && !v.flags.contains(VariantFlags::OTHER)
    })
    .map(|v| {
      let tag_str = &v.wire_name;
      let all_names = &v.all_wire_names;
      let vname = &v.ident;

      match &v.kind {
        VariantKind::Unit => quote! {
          #(#all_names)|* => return Ok(#name::#vname),
        },
        VariantKind::Unnamed(types) => {
          let body = unnamed_variant_from_js_ext(name, vname, types, tag_str);
          quote! {
            #(#all_names)|* => { return { #body }; }
          }
        }
        VariantKind::Named(fields) => {
          let body = named_variant_from_js_ext(name, vname, fields, tag_str);
          quote! {
            #(#all_names)|* => { return { #body }; }
          }
        }
      }
    })
    .collect();

  let untagged_fallbacks = untagged_variant_fallbacks(name, &e.variants);
  let other_fallback = other_variant_fallback(name, &e.variants);
  let has_fallbacks = !untagged_fallbacks.is_empty() || !other_fallback.is_empty();

  let all_unit = e.flags.contains(EnumFlags::ALL_UNIT);

  if has_fallbacks {
    let final_err = if other_fallback.is_empty() {
      quote! { Err(::typewire::Error::NoMatchingVariant) }
    } else {
      other_fallback
    };

    let unit_arms: Vec<_> = e
      .variants
      .iter()
      .filter(|v| {
        !v.flags.contains(VariantFlags::SKIP_DE)
          && !v.flags.contains(VariantFlags::UNTAGGED)
          && matches!(v.kind, VariantKind::Unit)
      })
      .map(|v| {
        let all_names = &v.all_wire_names;
        let vname = &v.ident;
        quote! { #(#all_names)|* => return Ok(#name::#vname), }
      })
      .collect();

    quote! {
      if let Some(s) = value.as_string() {
        match s.as_str() {
          #(#unit_arms)*
          _ => {}
        }
      }
      if let Some(__obj) = value.dyn_ref::<::js_sys::Object>() {
        let keys = ::js_sys::Object::keys(__obj);
        for __i in 0..keys.length() {
          if let Some(tag) = keys.get(__i).as_string() {
            match tag.as_str() {
              #(#from_js_arms)*
              _ => continue,
            }
          }
        }
      }
      #(#untagged_fallbacks)*
      #final_err
    }
  } else if all_unit {
    quote! {
      let s = value.as_string()
        .ok_or(::typewire::Error::UnexpectedType { expected: "string" })?;
      match s.as_str() {
        #(#from_js_arms)*
        other => Err(::typewire::Error::UnknownVariant { variant: other.into() }),
      }
    }
  } else {
    let unit_arms: Vec<_> = e
      .variants
      .iter()
      .filter(|v| !v.flags.contains(VariantFlags::SKIP_DE) && matches!(v.kind, VariantKind::Unit))
      .map(|v| {
        let all_names = &v.all_wire_names;
        let vname = &v.ident;
        quote! { #(#all_names)|* => return Ok(#name::#vname), }
      })
      .collect();
    quote! {
      if let Some(s) = value.as_string() {
        match s.as_str() {
          #(#unit_arms)*
          other => return Err(::typewire::Error::UnknownVariant { variant: other.into() }),
        }
      }
      let __obj = value.dyn_ref::<::js_sys::Object>()
        .ok_or(::typewire::Error::UnexpectedType { expected: "object or string" })?;
      let keys = ::js_sys::Object::keys(__obj);
      for __i in 0..keys.length() {
        if let Some(tag) = keys.get(__i).as_string() {
          match tag.as_str() {
            #(#from_js_arms)*
            _ => continue,
          }
        }
      }
      Err(::typewire::Error::UnknownVariant {
        variant: keys.get(0).as_string().unwrap_or_default(),
      })
    }
  }
}

fn ext_tagged_patch_js(e: &SchemaEnum) -> TokenStream {
  let name = &e.ident;
  let arms: Vec<_> = e
    .variants
    .iter()
    .filter_map(|v| {
      if v.flags.contains(VariantFlags::SKIP_SER) {
        return None;
      }
      let vname = &v.ident;
      let tag_val = &v.wire_name;

      match &v.kind {
        VariantKind::Unit => Some(quote! {
          #name::#vname => {
            if !old.as_string().is_some_and(|s| s == #tag_val) {
              _set(self.to_js());
            }
          }
        }),
        VariantKind::Named(fields) => {
          let binds = field_binds(fields);
          let patches = patch_bound_fields(fields, &quote! { __content });
          Some(quote! {
            #name::#vname { #(#binds,)* .. } => {
              let __key = ::wasm_bindgen::JsValue::from_str(#tag_val);
              let __content = ::js_sys::Reflect::get(old, &__key)
                .unwrap_or(::wasm_bindgen::JsValue::UNDEFINED);
              if __content.is_undefined() {
                _set(self.to_js());
              } else {
                #(#patches)*
              }
            }
          })
        }
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          Some(quote! {
            #name::#vname(__inner) => {
              let __key = ::wasm_bindgen::JsValue::from_str(#tag_val);
              let __content = ::js_sys::Reflect::get(old, &__key)
                .unwrap_or(::wasm_bindgen::JsValue::UNDEFINED);
              if __content.is_undefined() {
                _set(self.to_js());
              } else {
                <#ty as ::typewire::Typewire>::patch_js(__inner, &__content, |v| {
                  let _ = ::js_sys::Reflect::set(old, &__key, &v);
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
      match self {
        #(#arms)*
        #[allow(unreachable_patterns)]
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
  e.variants.iter().filter_map(move |v| {
    if v.flags.contains(VariantFlags::SKIP_SER) {
      return None;
    }
    let tag_val = &v.wire_name;
    let vname = &v.ident;

    match &v.kind {
      VariantKind::Unit => Some(quote! {
        #name::#vname => {
          let obj = ::js_sys::Object::new();
          let _ = ::js_sys::Reflect::set(
            &obj,
            &::wasm_bindgen::JsValue::from_str(#tag),
            &::wasm_bindgen::JsValue::from_str(#tag_val),
          );
          obj.into()
        }
      }),
      VariantKind::Named(fields) => {
        let (binds, setters) = named_fields_to_js(fields);
        Some(quote! {
          #name::#vname { #(#binds,)* .. } => {
            let obj = ::js_sys::Object::new();
            let _ = ::js_sys::Reflect::set(
              &obj,
              &::wasm_bindgen::JsValue::from_str(#tag),
              &::wasm_bindgen::JsValue::from_str(#tag_val),
            );
            #(#setters)*
            obj.into()
          }
        })
      }
      VariantKind::Unnamed(types) if types.len() == 1 => Some(quote! {
        #name::#vname(__inner) => {
          let obj_val = ::typewire::Typewire::to_js(__inner);
          let _ = ::js_sys::Reflect::set(
            &obj_val,
            &::wasm_bindgen::JsValue::from_str(#tag),
            &::wasm_bindgen::JsValue::from_str(#tag_val),
          );
          obj_val
        }
      }),
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

  let from_js_arms: Vec<_> = e
    .variants
    .iter()
    .filter(|v| {
      !v.flags.contains(VariantFlags::SKIP_DE)
        && !v.flags.contains(VariantFlags::UNTAGGED)
        && !v.flags.contains(VariantFlags::OTHER)
    })
    .filter_map(|v| {
      let all_names = &v.all_wire_names;
      let vname = &v.ident;

      match &v.kind {
        VariantKind::Unit => Some(quote! {
          #(#all_names)|* => return Ok(#name::#vname),
        }),
        VariantKind::Named(fields) => {
          let body = named_fields_from_js_obj(name, vname, fields);
          Some(quote! {
            #(#all_names)|* => { return { #body }; }
          })
        }
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          Some(quote! {
            #(#all_names)|* => {
              let inner = <#ty as ::typewire::Typewire>::from_js(value)?;
              return Ok(#name::#vname(inner));
            }
          })
        }
        VariantKind::Unnamed(_) => None,
      }
    })
    .collect();

  let untagged_fallbacks = untagged_variant_fallbacks(name, &e.variants);
  let other_fallback = other_variant_fallback(name, &e.variants);
  let has_fallbacks = !untagged_fallbacks.is_empty() || !other_fallback.is_empty();

  if has_fallbacks {
    let final_err = if other_fallback.is_empty() {
      quote! { Err(::typewire::Error::NoMatchingVariant) }
    } else {
      other_fallback
    };

    quote! {
      let __tag_val = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#tag))
        .ok()
        .and_then(|v| v.as_string());
      if let Some(ref tag_val) = __tag_val {
        match tag_val.as_str() {
          #(#from_js_arms)*
          _ => {}
        }
      }
      #(#untagged_fallbacks)*
      #final_err
    }
  } else {
    quote! {
      let tag_val = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#tag))
        .ok()
        .and_then(|v| v.as_string())
        .ok_or(::typewire::Error::MissingField { field: #tag })?;
      match tag_val.as_str() {
        #(#from_js_arms)*
        other => Err(::typewire::Error::UnknownVariant { variant: other.into() }),
      }
    }
  }
}

fn int_tagged_patch_js(e: &SchemaEnum, tag: &str) -> TokenStream {
  let name = &e.ident;
  let arms: Vec<_> = e
    .variants
    .iter()
    .filter_map(|v| {
      if v.flags.contains(VariantFlags::SKIP_SER) {
        return None;
      }
      let vname = &v.ident;
      let tag_val = &v.wire_name;

      match &v.kind {
        VariantKind::Unit => Some(quote! {
          #name::#vname => {
            if __old_tag.as_deref() != Some(#tag_val) {
              _set(self.to_js());
            }
          }
        }),
        VariantKind::Named(fields) => {
          let binds = field_binds(fields);
          let patches = patch_bound_fields(fields, &quote! { old });
          Some(quote! {
            #name::#vname { #(#binds,)* .. } => {
              if __old_tag.as_deref() != Some(#tag_val) {
                _set(self.to_js());
              } else {
                #(#patches)*
              }
            }
          })
        }
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          Some(quote! {
            #name::#vname(__inner) => {
              if __old_tag.as_deref() != Some(#tag_val) {
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
      let __old_tag = ::js_sys::Reflect::get(old, &::wasm_bindgen::JsValue::from_str(#tag))
        .ok()
        .and_then(|v| v.as_string());
      match self {
        #(#arms)*
        #[allow(unreachable_patterns)]
        _ => { _set(self.to_js()); }
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Adjacently tagged: `{ "t": "VariantName", "c": <content> }`
// ---------------------------------------------------------------------------

fn adj_tagged_to_js_arms<'a>(
  e: &'a SchemaEnum,
  tag: &'a str,
  content: &'a str,
) -> impl Iterator<Item = TokenStream> + 'a {
  let name = &e.ident;
  e.variants.iter().filter_map(move |v| {
    if v.flags.contains(VariantFlags::SKIP_SER) {
      return None;
    }
    let tag_val = &v.wire_name;
    let vname = &v.ident;

    match &v.kind {
      VariantKind::Unit => Some(quote! {
        #name::#vname => {
          let obj = ::js_sys::Object::new();
          let _ = ::js_sys::Reflect::set(
            &obj,
            &::wasm_bindgen::JsValue::from_str(#tag),
            &::wasm_bindgen::JsValue::from_str(#tag_val),
          );
          obj.into()
        }
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
          #name::#vname(#(#binds),*) => {
            let obj = ::js_sys::Object::new();
            let _ = ::js_sys::Reflect::set(
              &obj,
              &::wasm_bindgen::JsValue::from_str(#tag),
              &::wasm_bindgen::JsValue::from_str(#tag_val),
            );
            let _ = ::js_sys::Reflect::set(
              &obj,
              &::wasm_bindgen::JsValue::from_str(#content),
              &#content_val,
            );
            obj.into()
          }
        })
      }
      VariantKind::Named(fields) => {
        let (binds, setters) = named_fields_to_js(fields);
        Some(quote! {
          #name::#vname { #(#binds,)* .. } => {
            let obj = ::js_sys::Object::new();
            #(#setters)*
            let __wrapper = ::js_sys::Object::new();
            let _ = ::js_sys::Reflect::set(
              &__wrapper,
              &::wasm_bindgen::JsValue::from_str(#tag),
              &::wasm_bindgen::JsValue::from_str(#tag_val),
            );
            let _ = ::js_sys::Reflect::set(
              &__wrapper,
              &::wasm_bindgen::JsValue::from_str(#content),
              &obj,
            );
            __wrapper.into()
          }
        })
      }
    }
  })
}

fn adj_tagged_to_js(e: &SchemaEnum, tag: &str, content: &str) -> TokenStream {
  let to_js_arms = adj_tagged_to_js_arms(e, tag, content);
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

fn adj_tagged_from_js(e: &SchemaEnum, tag: &str, content: &str) -> TokenStream {
  let name = &e.ident;

  let from_js_arms: Vec<_> = e
    .variants
    .iter()
    .filter(|v| {
      !v.flags.contains(VariantFlags::SKIP_DE)
        && !v.flags.contains(VariantFlags::UNTAGGED)
        && !v.flags.contains(VariantFlags::OTHER)
    })
    .map(|v| {
      let all_names = &v.all_wire_names;
      let vname = &v.ident;

      match &v.kind {
        VariantKind::Unit => quote! {
          #(#all_names)|* => return Ok(#name::#vname),
        },
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          quote! {
            #(#all_names)|* => {
              let c = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#content))
                .map_err(|_| ::typewire::Error::MissingField { field: #content })?;
              let inner = <#ty as ::typewire::Typewire>::from_js(c)?;
              return Ok(#name::#vname(inner));
            }
          }
        }
        VariantKind::Unnamed(types) => {
          let bindings = types.iter().enumerate().map(|(i, ty)| {
            let var = format_ident!("__f{i}");
            let idx = u32::try_from(i).unwrap();
            quote! { let #var = <#ty as ::typewire::Typewire>::from_js(arr.get(#idx))?; }
          });
          let vars: Vec<_> = (0..types.len())
            .map(|i| format_ident!("__f{i}"))
            .collect();
          quote! {
            #(#all_names)|* => {
              let c = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#content))
                .map_err(|_| ::typewire::Error::MissingField { field: #content })?;
              let arr: ::js_sys::Array = c.try_into()
                .map_err(|_| ::typewire::Error::UnexpectedType { expected: "array" })?;
              #(#bindings)*
              return Ok(#name::#vname(#(#vars),*));
            }
          }
        }
        VariantKind::Named(fields) => {
          let body = named_fields_from_js_obj(name, vname, fields);
          quote! {
            #(#all_names)|* => {
              let value = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#content))
                .map_err(|_| ::typewire::Error::MissingField { field: #content })?;
              return { #body };
            }
          }
        }
      }
    })
    .collect();

  let untagged_fallbacks = untagged_variant_fallbacks(name, &e.variants);
  let other_fallback = other_variant_fallback(name, &e.variants);
  let has_fallbacks = !untagged_fallbacks.is_empty() || !other_fallback.is_empty();

  if has_fallbacks {
    let final_err = if other_fallback.is_empty() {
      quote! { Err(::typewire::Error::NoMatchingVariant) }
    } else {
      other_fallback
    };

    quote! {
      let __tag_val = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#tag))
        .ok()
        .and_then(|v| v.as_string());
      if let Some(ref tag_val) = __tag_val {
        match tag_val.as_str() {
          #(#from_js_arms)*
          _ => {}
        }
      }
      #(#untagged_fallbacks)*
      #final_err
    }
  } else {
    quote! {
      let tag_val = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#tag))
        .ok()
        .and_then(|v| v.as_string())
        .ok_or(::typewire::Error::MissingField { field: #tag })?;
      match tag_val.as_str() {
        #(#from_js_arms)*
        other => Err(::typewire::Error::UnknownVariant { variant: other.into() }),
      }
    }
  }
}

fn adj_tagged_patch_js(e: &SchemaEnum, tag: &str, content: &str) -> TokenStream {
  let name = &e.ident;
  let arms: Vec<_> = e
    .variants
    .iter()
    .filter_map(|v| {
      if v.flags.contains(VariantFlags::SKIP_SER) {
        return None;
      }
      let vname = &v.ident;
      let tag_val = &v.wire_name;

      match &v.kind {
        VariantKind::Unit => Some(quote! {
          #name::#vname => {
            if __old_tag.as_deref() != Some(#tag_val) {
              _set(self.to_js());
            }
          }
        }),
        VariantKind::Named(fields) => {
          let content_str = content;
          let binds = field_binds(fields);
          let patches = patch_bound_fields(fields, &quote! { __content });
          Some(quote! {
            #name::#vname { #(#binds,)* .. } => {
              if __old_tag.as_deref() != Some(#tag_val) {
                _set(self.to_js());
              } else {
                let __content = ::js_sys::Reflect::get(old, &::wasm_bindgen::JsValue::from_str(#content_str))
                  .unwrap_or(::wasm_bindgen::JsValue::UNDEFINED);
                #(#patches)*
              }
            }
          })
        }
        VariantKind::Unnamed(types) if types.len() == 1 => {
          let ty = &types[0];
          let content_str = content;
          Some(quote! {
            #name::#vname(__inner) => {
              if __old_tag.as_deref() != Some(#tag_val) {
                _set(self.to_js());
              } else {
                let __content_key = ::wasm_bindgen::JsValue::from_str(#content_str);
                let __content = ::js_sys::Reflect::get(old, &__content_key)
                  .unwrap_or(::wasm_bindgen::JsValue::UNDEFINED);
                <#ty as ::typewire::Typewire>::patch_js(__inner, &__content, |v| {
                  let _ = ::js_sys::Reflect::set(old, &__content_key, &v);
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
      let __old_tag = ::js_sys::Reflect::get(old, &::wasm_bindgen::JsValue::from_str(#tag))
        .ok()
        .and_then(|v| v.as_string());
      match self {
        #(#arms)*
        #[allow(unreachable_patterns)]
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
        let (binds, setters) = named_fields_to_js(fields);
        Some(quote! {
          #name::#vname { #(#binds,)* .. } => {
            let obj = ::js_sys::Object::new();
            #(#setters)*
            obj.into()
          }
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
          let len = types.len();
          let bindings = types.iter().enumerate().map(|(i, ty)| {
            let var = format_ident!("__f{i}");
            let idx = u32::try_from(i).unwrap();
            quote! { let #var = <#ty as ::typewire::Typewire>::from_js(arr.get(#idx))?; }
          });
          let vars: Vec<_> = (0..len).map(|i| format_ident!("__f{i}")).collect();
          let len_u32 = u32::try_from(len).unwrap();
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

/// Generate bind patterns and JS setters for named fields (used in enum variant `to_js`).
fn named_fields_to_js(fields: &[SchemaField]) -> (Vec<TokenStream>, Vec<TokenStream>) {
  let mut binds = Vec::new();
  let mut setters = Vec::new();

  for f in fields {
    if f.flags.contains(FieldFlags::SKIP_SER) {
      continue;
    }
    let ident = &f.ident;
    let js_key = &f.wire_name;

    binds.push(quote! { #ident });

    let ident_ts = quote! { #ident };
    let to_js = field_to_js_expr(&ident_ts, f);

    if f.flags.contains(FieldFlags::FLATTEN) {
      setters.push(quote! {
        {
          let inner = #to_js;
          if let Some(inner_obj) = inner.dyn_ref::<::js_sys::Object>() {
            let entries = ::js_sys::Object::entries(inner_obj);
            for i in 0..entries.length() {
              let pair: ::js_sys::Array = entries.get(i).into();
              let _ = ::js_sys::Reflect::set(&obj, &pair.get(0), &pair.get(1));
            }
          }
        }
      });
    } else if let Some(ref pred_path) = f.skip_serializing_if {
      setters.push(quote! {
        if !#pred_path(#ident) {
          let _ = ::js_sys::Reflect::set(
            &obj,
            &::wasm_bindgen::JsValue::from_str(#js_key),
            &#to_js,
          );
        }
      });
    } else {
      setters.push(quote! {
        let _ = ::js_sys::Reflect::set(
          &obj,
          &::wasm_bindgen::JsValue::from_str(#js_key),
          &#to_js,
        );
      });
    }
  }

  (binds, setters)
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

/// Handle an externally-tagged unnamed variant from `{ "Tag": content }`.
fn unnamed_variant_from_js_ext(
  enum_name: &Ident,
  vname: &Ident,
  types: &[syn::Type],
  tag_str: &str,
) -> TokenStream {
  if types.len() == 1 {
    let ty = &types[0];
    quote! {
      let c = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#tag_str))
        .map_err(|_| ::typewire::Error::MissingField { field: #tag_str })?;
      let inner = <#ty as ::typewire::Typewire>::from_js(c)?;
      Ok(#enum_name::#vname(inner))
    }
  } else {
    let bindings = types.iter().enumerate().map(|(i, ty)| {
      let var = format_ident!("__f{i}");
      let idx = u32::try_from(i).unwrap();
      quote! { let #var = <#ty as ::typewire::Typewire>::from_js(arr.get(#idx))?; }
    });
    let vars: Vec<_> = (0..types.len()).map(|i| format_ident!("__f{i}")).collect();
    quote! {
      let c = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#tag_str))
        .map_err(|_| ::typewire::Error::MissingField { field: #tag_str })?;
      let arr: ::js_sys::Array = c.try_into()
        .map_err(|_| ::typewire::Error::UnexpectedType { expected: "array" })?;
      #(#bindings)*
      Ok(#enum_name::#vname(#(#vars),*))
    }
  }
}

/// Handle an externally-tagged named variant from `{ "Tag": { fields } }`.
fn named_variant_from_js_ext(
  enum_name: &Ident,
  vname: &Ident,
  fields: &[SchemaField],
  tag_str: &str,
) -> TokenStream {
  let body = named_fields_from_js_obj(enum_name, vname, fields);
  quote! {
    let value = ::js_sys::Reflect::get(&value, &::wasm_bindgen::JsValue::from_str(#tag_str))
      .map_err(|_| ::typewire::Error::MissingField { field: #tag_str })?;
    #body
  }
}

// ---------------------------------------------------------------------------
// Patch codegen helpers
// ---------------------------------------------------------------------------

/// Generate `patch_js` calls for named fields, accessed via `self.field`.
fn patch_self_fields(fields: &[SchemaField], obj: &TokenStream) -> Vec<TokenStream> {
  patch_named_fields(fields, obj, |ident| quote! { &self.#ident })
}

/// Generate `patch_js` calls for named fields bound by an enum pattern.
fn patch_bound_fields(fields: &[SchemaField], obj: &TokenStream) -> Vec<TokenStream> {
  patch_named_fields(fields, obj, |ident| quote! { #ident })
}

/// Shared implementation for `patch_self_fields` and `patch_bound_fields`.
fn patch_named_fields(
  fields: &[SchemaField],
  obj: &TokenStream,
  field_ref: impl Fn(&Ident) -> TokenStream,
) -> Vec<TokenStream> {
  fields
    .iter()
    .filter_map(|f| {
      if f.flags.contains(FieldFlags::SKIP_SER) && f.flags.contains(FieldFlags::SKIP_DE) {
        return None;
      }
      let ident = &f.ident;
      let js_key = &f.wire_name;
      if f.flags.contains(FieldFlags::FLATTEN) {
        let field_ts = field_ref(ident);
        return Some(quote! {
          ::typewire::Typewire::patch_js(
            #field_ts,
            &#obj,
            |_| {},
          );
        });
      }
      let ident_ts = field_ref(ident);
      let to_js = field_to_js_expr(&ident_ts, f);
      let is_special = f.flags.intersects(
        FieldFlags::BASE64 | FieldFlags::DISPLAY | FieldFlags::SERDE_BYTES,
      );
      let patch_call = if is_special {
        quote! {
          {
            let __old_v = ::js_sys::Reflect::get(&#obj, &__k).unwrap_or(::wasm_bindgen::JsValue::UNDEFINED);
            let __new_v = #to_js;
            if __old_v != __new_v {
              let _ = ::js_sys::Reflect::set(&#obj, &__k, &__new_v);
            }
          }
        }
      } else {
        quote! {
          ::typewire::Typewire::patch_js(
            #ident_ts,
            &::js_sys::Reflect::get(&#obj, &__k).unwrap_or(::wasm_bindgen::JsValue::UNDEFINED),
            |v| { let _ = ::js_sys::Reflect::set(&#obj, &__k, &v); },
          );
        }
      };
      Some(quote! {
        {
          let __k = ::wasm_bindgen::JsValue::from_str(#js_key);
          #patch_call
        }
      })
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
          let len = u32::try_from(types.len()).unwrap();
          let bindings = types.iter().enumerate().map(|(i, ty)| {
            let var = format_ident!("__f{i}");
            let idx = u32::try_from(i).unwrap();
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
