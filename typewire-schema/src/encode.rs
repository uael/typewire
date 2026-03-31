//! Token generation for coded schema records.
//!
//! Converts [`Schema`] (with `syn` types) into `(type_tokens, value_tokens)`
//! pairs that construct flat, packed [`coded::Record<T>`](crate::coded::Record)
//! statics for link-section embedding.
//!
//! This module is only available when the `encode` feature is enabled. It is used
//! by `typewire-derive` to generate the `type Ident` + `const IDENT` trait
//! items and the `#[link_section = "typewire_schemas"]` static for each
//! `#[derive(Typewire)]` type.
//!
//! # Platform independence
//!
//! The schema embedding is **platform-independent** — it records type metadata
//! in the compiled binary regardless of target. Adding a new target language
//! only requires a new emitter (like the existing `typescript` module); the
//! schema encoding handled here requires no changes.

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
  Enum, Field, FieldDefault, FieldFlags, FromProxy, IntoProxy, Schema, Struct, StructShape,
  Tagging, Transparent, Variant, VariantFlags, VariantKind,
};

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Returns `(trait_items, link_section)`:
/// - `trait_items`: `type Ident = ...; const IDENT: Self::Ident = ...;`
/// - `link_section`: `const _: () = { #[link_section] #[used] static ... };`
///   (empty for generic types or when `emit_section` is `false`)
///
/// # Panics
///
/// Panics if any count (generics, fields, variants) exceeds `u32::MAX`.
#[must_use]
pub fn generate_schema_and_section(
  schema: &Schema,
  emit_section: bool,
) -> (TokenStream, TokenStream) {
  let c = quote! { ::typewire::schema::coded };
  let generic_count = u32::try_from(generics_to_strings(schema.generics()).len()).unwrap();

  let (record_type, record_value) = match schema {
    Schema::Struct(s) => coded_struct(s, &c, generic_count),
    Schema::Transparent(t) => coded_transparent(t, &c),
    Schema::Enum(e) => coded_enum(e, &c, generic_count),
    Schema::IntoProxy(p) => coded_into_proxy(p, &c, generic_count),
    Schema::FromProxy(p) => coded_from_proxy(p, &c, generic_count),
  };

  // Ident for derived types is just the type name
  let ident_str = schema.ident().to_string();
  let ident_len = ident_str.len();
  let ident_bytes = proc_macro2::Literal::byte_string(ident_str.as_bytes());

  let trait_items = quote! {
    type Ident = #c::Ident<#ident_len>;
    const IDENT: Self::Ident = #c::Ident::new(*#ident_bytes);
  };

  // Skip the link section for generic types (can't have statics in generic
  // contexts) or when schema embedding is disabled.
  let has_generic_params = !schema.generics().params.is_empty();

  let link_section = if has_generic_params || !emit_section {
    TokenStream::new()
  } else {
    // NB: link_section requires a string literal — must match coded::SECTION_NAME.
    quote! {
      const _: () = {
        #[cfg_attr(
          target_vendor = "apple",
          unsafe(link_section = "__DATA,typewire_schemas")
        )]
        #[cfg_attr(
          not(target_vendor = "apple"),
          unsafe(link_section = "typewire_schemas")
        )]
        #[used]
        static __TYPEWIRE_SCHEMA: #c::Record<#record_type> =
          #c::Record::new(#record_value);
      };
    }
  };

  (trait_items, link_section)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

pub(crate) fn generics_to_strings(generics: &syn::Generics) -> Vec<String> {
  generics
    .params
    .iter()
    .filter_map(
      |p| {
        if let syn::GenericParam::Type(tp) = p { Some(tp.ident.to_string()) } else { None }
      },
    )
    .collect()
}

/// Returns the `Types{N}` ident for a given count.
fn types_ident(n: usize) -> syn::Ident {
  syn::Ident::new(&format!("Types{n}"), proc_macro2::Span::call_site())
}

// ---------------------------------------------------------------------------
// Struct
// ---------------------------------------------------------------------------

/// Returns `(type_tokens, value_tokens)` for a struct's coded record.
fn coded_struct(s: &Struct, c: &TokenStream, generic_count: u32) -> (TokenStream, TokenStream) {
  let ident_str = s.ident.to_string();
  let ident_len = ident_str.len();
  let ident_bytes = proc_macro2::Literal::byte_string(ident_str.as_bytes());
  let flags_bits = s.flags.bits();

  match &s.shape {
    StructShape::Named(fields) => {
      let n = fields.len();
      let field_count = u32::try_from(n).unwrap();

      let (field_types, field_values): (Vec<_>, Vec<_>) =
        fields.iter().map(|f| coded_flat_field(f, c)).unzip();

      let types_name = types_ident(n);
      let record_type = quote! {
        #c::FlatStruct<#ident_len, #c::#types_name<#(#field_types),*>>
      };
      let record_value = quote! {
        #c::FlatStruct {
          tag: #c::Tag::Struct,
          ident: #c::Ident::new(*#ident_bytes),
          flags: ::typewire::schema::StructFlags::from_bits_retain(#flags_bits),
          shape: #c::StructShapeTag::Named,
          generic_count: #c::U32Le::new(#generic_count),
          field_count: #c::U32Le::new(#field_count),
          fields: #c::#types_name(#(#field_values),*),
        }
      };
      (record_type, record_value)
    }
    StructShape::Tuple(types) => {
      let n = types.len();
      let field_count = u32::try_from(n).unwrap();

      let ident_types: Vec<_> =
        types.iter().map(|ty| quote! { <#ty as ::typewire::Typewire>::Ident }).collect();
      let ident_values: Vec<_> =
        types.iter().map(|ty| quote! { <#ty as ::typewire::Typewire>::IDENT }).collect();

      let types_name = types_ident(n);
      let record_type = quote! {
        #c::FlatStruct<#ident_len, #c::#types_name<#(#ident_types),*>>
      };
      let record_value = quote! {
        #c::FlatStruct {
          tag: #c::Tag::Struct,
          ident: #c::Ident::new(*#ident_bytes),
          flags: ::typewire::schema::StructFlags::from_bits_retain(#flags_bits),
          shape: #c::StructShapeTag::Tuple,
          generic_count: #c::U32Le::new(#generic_count),
          field_count: #c::U32Le::new(#field_count),
          fields: #c::#types_name(#(#ident_values),*),
        }
      };
      (record_type, record_value)
    }
    StructShape::Unit => {
      let record_type = quote! { #c::FlatStruct<#ident_len> };
      let record_value = quote! {
        #c::FlatStruct {
          tag: #c::Tag::Struct,
          ident: #c::Ident::new(*#ident_bytes),
          flags: ::typewire::schema::StructFlags::from_bits_retain(#flags_bits),
          shape: #c::StructShapeTag::Unit,
          generic_count: #c::U32Le::new(#generic_count),
          field_count: #c::U32Le::new(0u32),
          fields: #c::Types0(),
        }
      };
      (record_type, record_value)
    }
  }
}

// ---------------------------------------------------------------------------
// Field
// ---------------------------------------------------------------------------

/// Returns `(type_tokens, value_tokens)` for a `FlatField`.
fn coded_flat_field(f: &Field, c: &TokenStream) -> (TokenStream, TokenStream) {
  let ident_str = f.ident.to_string();
  let ident_len = ident_str.len();
  let ident_bytes = proc_macro2::Literal::byte_string(ident_str.as_bytes());

  let wire_name = &f.wire_name;
  let wire_len = wire_name.len();
  let wire_bytes = proc_macro2::Literal::byte_string(wire_name.as_bytes());

  let ty = &f.ty;
  let flags_bits = f.flags.bits();

  let default_kind = match &f.default {
    FieldDefault::None => quote! { #c::FieldDefaultKind::None },
    FieldDefault::Default => quote! { #c::FieldDefaultKind::Default },
    FieldDefault::Path(_) => quote! { #c::FieldDefaultKind::Path },
  };

  let is_skipped = f.flags.intersects(FieldFlags::SKIP_SER | FieldFlags::SKIP_DE);

  let (ty_type, ty_value) = if is_skipped {
    (quote! { #c::SkippedIdent }, quote! { #c::SkippedIdent::SKIPPED })
  } else {
    (
      quote! { <#ty as ::typewire::Typewire>::Ident },
      quote! { <#ty as ::typewire::Typewire>::IDENT },
    )
  };

  let alias_count = u32::try_from(f.aliases.len()).unwrap();
  let (aliases_type, aliases_value) = coded_alias_list(&f.aliases, c);

  let field_type = quote! { #c::FlatField<#ident_len, #wire_len, #ty_type, #aliases_type> };
  let field_value = quote! {
    #c::FlatField {
      ident: #c::Ident::new(*#ident_bytes),
      ty: #ty_value,
      wire_name: #c::Ident::new(*#wire_bytes),
      flags: ::typewire::schema::FieldFlags::from_bits_retain(#flags_bits),
      default: #default_kind,
      alias_count: #c::U32Le::new(#alias_count),
      aliases: #aliases_value,
    }
  };
  (field_type, field_value)
}

/// Returns `(type_tokens, value_tokens)` for a packed alias list.
fn coded_alias_list(aliases: &[String], c: &TokenStream) -> (TokenStream, TokenStream) {
  if aliases.is_empty() {
    return (quote! { #c::Types0 }, quote! { #c::Types0() });
  }
  let n = aliases.len();
  let types_name = types_ident(n);
  let (alias_types, alias_values): (Vec<_>, Vec<_>) = aliases
    .iter()
    .map(|a| {
      let a_len = a.len();
      let a_bytes = proc_macro2::Literal::byte_string(a.as_bytes());
      (quote! { #c::Ident<#a_len> }, quote! { #c::Ident::new(*#a_bytes) })
    })
    .unzip();
  (quote! { #c::#types_name<#(#alias_types),*> }, quote! { #c::#types_name(#(#alias_values),*) })
}

// ---------------------------------------------------------------------------
// Transparent
// ---------------------------------------------------------------------------

/// Returns `(type_tokens, value_tokens)` for a transparent record.
fn coded_transparent(t: &Transparent, c: &TokenStream) -> (TokenStream, TokenStream) {
  let ident_str = t.ident.to_string();
  let ident_len = ident_str.len();
  let ident_bytes = proc_macro2::Literal::byte_string(ident_str.as_bytes());
  let atomic = u8::from(t.atomic);
  let field_ty = &t.field_ty;

  let record_type = quote! {
    #c::FlatTransparent<#ident_len, <#field_ty as ::typewire::Typewire>::Ident>
  };
  let record_value = quote! {
    #c::FlatTransparent {
      tag: #c::Tag::Transparent,
      ident: #c::Ident::new(*#ident_bytes),
      atomic: #atomic,
      inner: <#field_ty as ::typewire::Typewire>::IDENT,
    }
  };
  (record_type, record_value)
}

// ---------------------------------------------------------------------------
// Enum
// ---------------------------------------------------------------------------

/// Returns `(type_tokens, value_tokens)` for an enum record.
fn coded_enum(e: &Enum, c: &TokenStream, generic_count: u32) -> (TokenStream, TokenStream) {
  let ident_str = e.ident.to_string();
  let ident_len = ident_str.len();
  let ident_bytes = proc_macro2::Literal::byte_string(ident_str.as_bytes());
  let flags_bits = e.flags.bits();

  let (tagging_kind, tag_key_str, content_key_str) = match &e.tagging {
    Tagging::External => (quote! { #c::TaggingKind::External }, "", ""),
    Tagging::Internal { tag } => (quote! { #c::TaggingKind::Internal }, tag.as_str(), ""),
    Tagging::Adjacent { tag, content } => {
      (quote! { #c::TaggingKind::Adjacent }, tag.as_str(), content.as_str())
    }
    Tagging::Untagged => (quote! { #c::TaggingKind::Untagged }, "", ""),
  };

  let tag_key_len = tag_key_str.len();
  let tag_key_bytes = proc_macro2::Literal::byte_string(tag_key_str.as_bytes());
  let content_key_len = content_key_str.len();
  let content_key_bytes = proc_macro2::Literal::byte_string(content_key_str.as_bytes());

  let variant_count = u32::try_from(e.variants.len()).unwrap();
  let n = e.variants.len();

  let (variant_types, variant_values): (Vec<_>, Vec<_>) =
    e.variants.iter().map(|v| coded_flat_variant(v, c)).unzip();

  let types_name = types_ident(n);

  let record_type = quote! {
    #c::FlatEnum<#ident_len, #tag_key_len, #content_key_len,
      #c::#types_name<#(#variant_types),*>>
  };
  let record_value = quote! {
    #c::FlatEnum {
      tag: #c::Tag::Enum,
      ident: #c::Ident::new(*#ident_bytes),
      flags: ::typewire::schema::EnumFlags::from_bits_retain(#flags_bits),
      tagging: #tagging_kind,
      tag_key: #c::Ident::new(*#tag_key_bytes),
      content_key: #c::Ident::new(*#content_key_bytes),
      generic_count: #c::U32Le::new(#generic_count),
      variant_count: #c::U32Le::new(#variant_count),
      variants: #c::#types_name(#(#variant_values),*),
    }
  };
  (record_type, record_value)
}

// ---------------------------------------------------------------------------
// Variant
// ---------------------------------------------------------------------------

/// Returns `(type_tokens, value_tokens)` for a `FlatVariant`.
fn coded_flat_variant(v: &Variant, c: &TokenStream) -> (TokenStream, TokenStream) {
  let ident_str = v.ident.to_string();
  let ident_len = ident_str.len();
  let ident_bytes = proc_macro2::Literal::byte_string(ident_str.as_bytes());

  let wire_name = &v.wire_name;
  let wire_len = wire_name.len();
  let wire_bytes = proc_macro2::Literal::byte_string(wire_name.as_bytes());

  let flags_bits = v.flags.bits();
  let is_skipped = v.flags.intersects(VariantFlags::SKIP_SER | VariantFlags::SKIP_DE);

  // Variant aliases are all_wire_names[1..] (first is the primary wire_name).
  let variant_aliases: Vec<String> = v.all_wire_names.iter().skip(1).cloned().collect();
  let v_alias_count = u32::try_from(variant_aliases.len()).unwrap();
  let (v_aliases_type, v_aliases_value) = coded_alias_list(&variant_aliases, c);

  // Skipped variants emit no child types (avoids requiring Typewire on internal types)
  if is_skipped {
    let variant_type =
      quote! { #c::FlatVariant<#ident_len, #wire_len, #c::Types0, #v_aliases_type> };
    let variant_value = quote! {
      #c::FlatVariant {
        ident: #c::Ident::new(*#ident_bytes),
        wire_name: #c::Ident::new(*#wire_bytes),
        flags: ::typewire::schema::VariantFlags::from_bits_retain(#flags_bits),
        kind: #c::VariantKindTag::Unit,
        child_count: #c::U32Le::new(0u32),
        fields: #c::Types0(),
        alias_count: #c::U32Le::new(#v_alias_count),
        aliases: #v_aliases_value,
      }
    };
    return (variant_type, variant_value);
  }

  match &v.kind {
    VariantKind::Unit => {
      let variant_type =
        quote! { #c::FlatVariant<#ident_len, #wire_len, #c::Types0, #v_aliases_type> };
      let variant_value = quote! {
        #c::FlatVariant {
          ident: #c::Ident::new(*#ident_bytes),
          wire_name: #c::Ident::new(*#wire_bytes),
          flags: ::typewire::schema::VariantFlags::from_bits_retain(#flags_bits),
          kind: #c::VariantKindTag::Unit,
          child_count: #c::U32Le::new(0u32),
          fields: #c::Types0(),
          alias_count: #c::U32Le::new(#v_alias_count),
          aliases: #v_aliases_value,
        }
      };
      (variant_type, variant_value)
    }
    VariantKind::Named(fields) => {
      let n = fields.len();
      let child_count = u32::try_from(n).unwrap();
      let types_name = types_ident(n);

      let (field_types, field_values): (Vec<_>, Vec<_>) =
        fields.iter().map(|f| coded_flat_field(f, c)).unzip();

      let variant_type = quote! {
        #c::FlatVariant<
          #ident_len,
          #wire_len,
          #c::#types_name<#(#field_types),*>,
          #v_aliases_type,
        >
      };
      let variant_value = quote! {
        #c::FlatVariant {
          ident: #c::Ident::new(*#ident_bytes),
          wire_name: #c::Ident::new(*#wire_bytes),
          flags: ::typewire::schema::VariantFlags::from_bits_retain(#flags_bits),
          kind: #c::VariantKindTag::Named,
          child_count: #c::U32Le::new(#child_count),
          fields: #c::#types_name(#(#field_values),*),
          alias_count: #c::U32Le::new(#v_alias_count),
          aliases: #v_aliases_value,
        }
      };
      (variant_type, variant_value)
    }
    VariantKind::Unnamed(types) => {
      let n = types.len();
      let child_count = u32::try_from(n).unwrap();
      let types_name = types_ident(n);

      let ident_types: Vec<_> =
        types.iter().map(|ty| quote! { <#ty as ::typewire::Typewire>::Ident }).collect();
      let ident_values: Vec<_> =
        types.iter().map(|ty| quote! { <#ty as ::typewire::Typewire>::IDENT }).collect();

      let variant_type = quote! {
        #c::FlatVariant<
          #ident_len,
          #wire_len,
          #c::#types_name<#(#ident_types),*>,
          #v_aliases_type,
        >
      };
      let variant_value = quote! {
        #c::FlatVariant {
          ident: #c::Ident::new(*#ident_bytes),
          wire_name: #c::Ident::new(*#wire_bytes),
          flags: ::typewire::schema::VariantFlags::from_bits_retain(#flags_bits),
          kind: #c::VariantKindTag::Unnamed,
          child_count: #c::U32Le::new(#child_count),
          fields: #c::#types_name(#(#ident_values),*),
          alias_count: #c::U32Le::new(#v_alias_count),
          aliases: #v_aliases_value,
        }
      };
      (variant_type, variant_value)
    }
  }
}

// ---------------------------------------------------------------------------
// Proxy types
// ---------------------------------------------------------------------------

/// Returns `(type_tokens, value_tokens)` for an `IntoProxy` record.
fn coded_into_proxy(
  p: &IntoProxy,
  c: &TokenStream,
  generic_count: u32,
) -> (TokenStream, TokenStream) {
  let ident_str = p.ident.to_string();
  let ident_len = ident_str.len();
  let ident_bytes = proc_macro2::Literal::byte_string(ident_str.as_bytes());
  let into_ty = &p.into_ty;

  let record_type = quote! {
    #c::FlatIntoProxy<#ident_len, <#into_ty as ::typewire::Typewire>::Ident>
  };
  let record_value = quote! {
    #c::FlatIntoProxy {
      tag: #c::Tag::IntoProxy,
      ident: #c::Ident::new(*#ident_bytes),
      generic_count: #c::U32Le::new(#generic_count),
      into_ty: <#into_ty as ::typewire::Typewire>::IDENT,
    }
  };
  (record_type, record_value)
}

/// Returns `(type_tokens, value_tokens)` for a `FromProxy` record.
fn coded_from_proxy(
  p: &FromProxy,
  c: &TokenStream,
  generic_count: u32,
) -> (TokenStream, TokenStream) {
  let ident_str = p.ident.to_string();
  let ident_len = ident_str.len();
  let ident_bytes = proc_macro2::Literal::byte_string(ident_str.as_bytes());
  let proxy = &p.proxy;
  let is_try = u8::from(p.is_try);

  let record_type = quote! {
    #c::FlatFromProxy<#ident_len, <#proxy as ::typewire::Typewire>::Ident>
  };
  let record_value = quote! {
    #c::FlatFromProxy {
      tag: #c::Tag::FromProxy,
      ident: #c::Ident::new(*#ident_bytes),
      generic_count: #c::U32Le::new(#generic_count),
      proxy: <#proxy as ::typewire::Typewire>::IDENT,
      is_try: #is_try,
    }
  };
  (record_type, record_value)
}
