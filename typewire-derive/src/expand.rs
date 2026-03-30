use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Fields};
use typewire_schema::{
  Enum as SchemaEnum, EnumFlags, Field as SchemaField, FieldDefault as SchemaFieldDefault,
  FieldFlags, FromBody, FromProxy, IntoProxy, Schema, Struct as SchemaStruct, StructFlags,
  StructShape, Tagging, Transparent, TypeShape, Variant as SchemaVariant, VariantFlags,
  VariantKind, encode,
};

use crate::{
  attr::{ContainerAttrs, ContainerDefault, FieldAttrs, VariantAttrs},
  case::RenameAll,
};

// ---------------------------------------------------------------------------
// Codegen trait — each platform implements this
// ---------------------------------------------------------------------------

/// Platform-specific code generator. Each method returns complete `fn` items
/// (with signature and body). The shared layer adds `#[cfg]` and wraps them
/// in the `impl Typewire` block.
pub trait Codegen {
  /// The `cfg` predicate gating this platform's methods
  /// (e.g. `target_arch = "wasm32"`).
  fn cfg_predicate() -> TokenStream;

  fn expand_struct(s: &SchemaStruct) -> Vec<TokenStream>;
  fn expand_transparent(t: &Transparent) -> Vec<TokenStream>;
  fn expand_enum(e: &SchemaEnum) -> Vec<TokenStream>;
  fn expand_into_proxy(p: &IntoProxy) -> Vec<TokenStream>;
  fn expand_from_proxy(p: &FromProxy) -> Vec<TokenStream>;
}

// ---------------------------------------------------------------------------
// expand<C>() — shared dispatch
// ---------------------------------------------------------------------------

pub fn expand<C: Codegen>(input: &DeriveInput) -> TokenStream {
  let schema = match analyze(input) {
    Ok(s) => s,
    Err(e) => return e,
  };

  // Platform-specific method bodies
  let methods = match &schema {
    Schema::Struct(s) => C::expand_struct(s),
    Schema::Transparent(t) => C::expand_transparent(t),
    Schema::Enum(e) => C::expand_enum(e),
    Schema::IntoProxy(p) => C::expand_into_proxy(p),
    Schema::FromProxy(p) => C::expand_from_proxy(p),
  };

  // Gate each fn with the platform cfg
  let cfg = C::cfg_predicate();
  let gated = methods.iter().map(|m| quote! { #[cfg(#cfg)] #m });

  // Schema type + const + link section (gated behind `schemas` feature)
  let (schema_items, coded_section) =
    encode::generate_schema_and_section(&schema, cfg!(feature = "schemas"));

  let ident = schema.ident();

  // Add Typewire bound to type params
  let mut generics = schema.generics().clone();
  for param in &mut generics.params {
    if let syn::GenericParam::Type(ref mut tp) = *param {
      tp.bounds.push(syn::parse_quote!(::typewire::Typewire));
    }
  }
  let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

  quote! {
    impl #impl_generics ::typewire::Typewire for #ident #ty_generics #where_clause {
      #(#gated)*
      #schema_items
    }
    #coded_section
  }
}

// ---------------------------------------------------------------------------
// analyze() — DeriveInput → Schema
// ---------------------------------------------------------------------------

fn analyze(input: &DeriveInput) -> Result<Schema, TokenStream> {
  let container = ContainerAttrs::from_attrs(&input.attrs);

  // Handle #[serde(from/try_from/into)] delegation.
  // When both from/try_from AND into are present, use into for to_js
  // and from/try_from for from_js.
  if container.into.is_some() {
    return analyze_into_proxy(input, &container);
  }
  if let Some(ref proxy) = container.from {
    return analyze_from_proxy(input, proxy.clone(), false, &container);
  }
  if let Some(ref proxy) = container.try_from {
    return analyze_from_proxy(input, proxy.clone(), true, &container);
  }

  match &input.data {
    Data::Struct(data) => analyze_struct(input, data, &container),
    Data::Enum(data) => analyze_enum(input, data, &container),
    Data::Union(_) => Err(
      syn::Error::new_spanned(input, "Typewire cannot be derived for unions").to_compile_error(),
    ),
  }
}

// --- struct ---

fn analyze_struct(
  input: &DeriveInput,
  data: &syn::DataStruct,
  container: &ContainerAttrs,
) -> Result<Schema, TokenStream> {
  if container.transparent {
    return analyze_transparent(input, data, container);
  }
  Ok(Schema::Struct(analyze_struct_shape(input, data, container)?))
}

fn analyze_struct_shape(
  input: &DeriveInput,
  data: &syn::DataStruct,
  container: &ContainerAttrs,
) -> Result<SchemaStruct, TokenStream> {
  let mut flags = StructFlags::empty();
  if container.diffable.atomic {
    flags |= StructFlags::ATOMIC;
  }
  if matches!(container.default, ContainerDefault::Default) {
    flags |= StructFlags::CONTAINER_DEFAULT;
  }
  if container.deny_unknown_fields {
    flags |= StructFlags::DENY_UNKNOWN_FIELDS;
  }

  let shape = match &data.fields {
    Fields::Named(fields) => {
      let analyzed = analyze_named_fields(
        fields,
        container.rename_all,
        flags.contains(StructFlags::CONTAINER_DEFAULT),
      )?;
      StructShape::Named(analyzed)
    }
    Fields::Unnamed(fields) => {
      let types = fields.unnamed.iter().map(|f| f.ty.clone()).collect();
      StructShape::Tuple(types)
    }
    Fields::Unit => StructShape::Unit,
  };

  Ok(SchemaStruct { ident: input.ident.clone(), generics: input.generics.clone(), flags, shape })
}

// --- transparent ---

fn analyze_transparent(
  input: &DeriveInput,
  data: &syn::DataStruct,
  container: &ContainerAttrs,
) -> Result<Schema, TokenStream> {
  if data.fields.len() != 1 {
    return Err(
      syn::Error::new_spanned(input, "transparent struct must have exactly one field")
        .to_compile_error(),
    );
  }
  let fields: Vec<_> = data.fields.iter().collect();
  let field = fields[0];

  Ok(Schema::Transparent(Transparent {
    ident: input.ident.clone(),
    generics: input.generics.clone(),
    atomic: container.diffable.atomic,
    field_ident: field.ident.clone(),
    field_ty: field.ty.clone(),
  }))
}

// --- enum ---

fn analyze_enum(
  input: &DeriveInput,
  data: &syn::DataEnum,
  container: &ContainerAttrs,
) -> Result<Schema, TokenStream> {
  if container.transparent {
    return Err(
      syn::Error::new_spanned(input, "transparent is not supported on enums").to_compile_error(),
    );
  }
  Ok(Schema::Enum(analyze_enum_shape(input, data, container)?))
}

fn analyze_enum_shape(
  input: &DeriveInput,
  data: &syn::DataEnum,
  container: &ContainerAttrs,
) -> Result<SchemaEnum, TokenStream> {
  // Internally tagged enums (tag without content) cannot have multi-field
  // tuple variants. Adjacently tagged (tag + content) CAN — the content
  // key holds the variant payload separately.
  if container.tag.is_some() && container.content.is_none() {
    for v in &data.variants {
      if let Fields::Unnamed(fields) = &v.fields
        && fields.unnamed.len() > 1
      {
        return Err(
          syn::Error::new_spanned(
            v,
            "tuple variants with more than one field cannot be used \
                     with #[serde(tag = \"...\")]",
          )
          .to_compile_error(),
        );
      }
    }
  }

  let all_unit = data.variants.iter().all(|v| matches!(v.fields, Fields::Unit));

  let mut flags = EnumFlags::empty();
  if container.diffable.atomic {
    flags |= EnumFlags::ATOMIC;
  }
  if all_unit {
    flags |= EnumFlags::ALL_UNIT;
  }

  let tagging = match (&container.tag, &container.content, container.untagged) {
    (Some(tag), Some(content), _) => {
      Tagging::Adjacent { tag: tag.clone(), content: content.clone() }
    }
    (Some(tag), None, _) => Tagging::Internal { tag: tag.clone() },
    (None, _, true) => Tagging::Untagged,
    _ => Tagging::External,
  };

  let variants: Vec<SchemaVariant> =
    data.variants.iter().map(|v| analyze_variant(v, container)).collect::<Result<_, _>>()?;

  Ok(SchemaEnum {
    ident: input.ident.clone(),
    generics: input.generics.clone(),
    flags,
    tagging,
    variants,
  })
}

fn analyze_variant(
  v: &syn::Variant,
  container: &ContainerAttrs,
) -> Result<SchemaVariant, TokenStream> {
  let vattrs = VariantAttrs::from_attrs(&v.attrs);
  let wire_name = variant_wire_name(&v.ident, &vattrs, container.rename_all);
  let all_wire_names = variant_all_names(&wire_name, &vattrs);

  let mut flags = VariantFlags::empty();
  if vattrs.skip_serializing() {
    flags |= VariantFlags::SKIP_SER;
  }
  if vattrs.skip_deserializing() {
    flags |= VariantFlags::SKIP_DE;
  }
  if vattrs.other {
    flags |= VariantFlags::OTHER;
  }
  if vattrs.untagged {
    flags |= VariantFlags::UNTAGGED;
  }

  let rename_all = resolve_field_rename_all(&vattrs, container);

  let kind = match &v.fields {
    Fields::Unit => VariantKind::Unit,
    Fields::Named(fields) => {
      let analyzed = analyze_named_fields(fields, rename_all, false)?;
      VariantKind::Named(analyzed)
    }
    Fields::Unnamed(fields) => {
      let types = fields.unnamed.iter().map(|f| f.ty.clone()).collect();
      VariantKind::Unnamed(types)
    }
  };

  Ok(SchemaVariant { ident: v.ident.clone(), wire_name, all_wire_names, flags, kind })
}

// --- named fields ---

fn analyze_named_fields(
  fields: &syn::FieldsNamed,
  rename_all: Option<RenameAll>,
  has_container_default: bool,
) -> Result<Vec<SchemaField>, TokenStream> {
  fields
    .named
    .iter()
    .map(|f| {
      let fattrs = FieldAttrs::from_attrs(&f.attrs);
      let ident = f
        .ident
        .clone()
        .ok_or_else(|| syn::Error::new_spanned(f, "expected named field").to_compile_error())?;
      let ty = f.ty.clone();
      let rust_name = ident.to_string();
      let wire_name = field_wire_name(Some(&rust_name), &fattrs, rename_all);

      let mut flags = FieldFlags::empty();
      if fattrs.skip_serializing() {
        flags |= FieldFlags::SKIP_SER;
      }
      if fattrs.skip_deserializing() {
        flags |= FieldFlags::SKIP_DE;
      }
      if fattrs.flatten {
        flags |= FieldFlags::FLATTEN;
      }
      if fattrs.encoding.base64 {
        flags |= FieldFlags::BASE64;
      }
      if fattrs.encoding.display {
        flags |= FieldFlags::DISPLAY;
      }
      if fattrs.encoding.serde_bytes {
        flags |= FieldFlags::SERDE_BYTES;
      }
      if fattrs.lenient {
        flags |= FieldFlags::LENIENT;
      }

      let aliases: Vec<String> = fattrs.aliases.clone();

      let default = match &fattrs.default {
        crate::attr::FieldDefault::None if has_container_default => SchemaFieldDefault::Default,
        crate::attr::FieldDefault::None => SchemaFieldDefault::None,
        crate::attr::FieldDefault::Default => SchemaFieldDefault::Default,
        crate::attr::FieldDefault::Path(s) => {
          let path: syn::Path = syn::parse_str(s).map_err(|e| e.to_compile_error())?;
          SchemaFieldDefault::Path(path)
        }
      };

      let skip_serializing_if = match &fattrs.skip_serializing_if {
        Some(s) => {
          let path: syn::Path = syn::parse_str(s).map_err(|e| e.to_compile_error())?;
          Some(path)
        }
        None => None,
      };

      Ok(SchemaField { ident, ty, wire_name, flags, aliases, default, skip_serializing_if })
    })
    .collect()
}

// --- proxy types ---

fn analyze_into_proxy(
  input: &DeriveInput,
  container: &ContainerAttrs,
) -> Result<Schema, TokenStream> {
  let Some(into_ty) = container.into.clone() else {
    return Err(syn::Error::new_spanned(input, "missing into attribute").to_compile_error());
  };

  let from_body = if let Some(ref proxy) = container.from {
    FromBody::Proxy(proxy.clone())
  } else if let Some(ref proxy) = container.try_from {
    FromBody::TryProxy(proxy.clone())
  } else {
    FromBody::Own(build_type_shape(input, container)?)
  };

  Ok(Schema::IntoProxy(IntoProxy {
    ident: input.ident.clone(),
    generics: input.generics.clone(),
    into_ty,
    from_body,
  }))
}

fn analyze_from_proxy(
  input: &DeriveInput,
  proxy: syn::Path,
  is_try: bool,
  container: &ContainerAttrs,
) -> Result<Schema, TokenStream> {
  let own_shape = build_type_shape(input, container)?;

  Ok(Schema::FromProxy(FromProxy {
    ident: input.ident.clone(),
    generics: input.generics.clone(),
    proxy,
    is_try,
    own_shape,
  }))
}

fn build_type_shape(
  input: &DeriveInput,
  container: &ContainerAttrs,
) -> Result<TypeShape, TokenStream> {
  match &input.data {
    Data::Struct(data) => Ok(TypeShape::Struct(analyze_struct_shape(input, data, container)?)),
    Data::Enum(data) => Ok(TypeShape::Enum(analyze_enum_shape(input, data, container)?)),
    Data::Union(_) => Err(
      syn::Error::new_spanned(input, "Typewire cannot be derived for unions").to_compile_error(),
    ),
  }
}

// Shared helpers (used by analyze)
// ---------------------------------------------------------------------------

/// Resolve the JS name for a field.
pub fn field_wire_name(
  rust_name: Option<&str>,
  attrs: &FieldAttrs,
  rename_all: Option<RenameAll>,
) -> String {
  if let Some(ref rename) = attrs.rename {
    return rename.clone();
  }
  let base = rust_name.unwrap_or("0");
  rename_all.map_or_else(|| base.to_string(), |case| case.apply(base))
}

/// Resolve the JS tag name for a variant.
fn variant_wire_name(
  ident: &syn::Ident,
  attrs: &VariantAttrs,
  rename_all: Option<RenameAll>,
) -> String {
  if let Some(ref rename) = attrs.rename {
    return rename.clone();
  }
  rename_all.map_or_else(|| ident.to_string(), |case| case.apply(&ident.to_string()))
}

/// All names (primary + aliases) for variant matching.
fn variant_all_names(primary: &str, attrs: &VariantAttrs) -> Vec<String> {
  let mut names = vec![primary.to_string()];
  names.extend(attrs.aliases.iter().cloned());
  names
}

/// Determine the `rename_all` for fields inside a variant, considering
/// variant-level `rename_all` → container-level `rename_all_fields`.
fn resolve_field_rename_all(
  vattrs: &VariantAttrs,
  container: &ContainerAttrs,
) -> Option<RenameAll> {
  vattrs.rename_all.or(container.rename_all_fields)
}
