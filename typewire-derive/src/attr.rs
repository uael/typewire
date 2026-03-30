use syn::{Attribute, Expr, ExprLit, Lit, LitStr, Path, meta::ParseNestedMeta};

use crate::case::RenameAll;

// ---------------------------------------------------------------------------
// Shared sub-structs for grouping bools
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct DiffableOpts {
  pub atomic: bool,
  pub transparent: bool,
}

#[derive(Default)]
pub struct SkipOpts {
  pub all: bool,
  pub serializing: bool,
  pub deserializing: bool,
}

#[derive(Default)]
pub struct EncodingOpts {
  pub base64: bool,
  pub display: bool,
  pub serde_bytes: bool,
}

// ---------------------------------------------------------------------------
// Container attributes (#[serde(...)] on struct/enum)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct ContainerAttrs {
  pub rename_all: Option<RenameAll>,
  /// For enums: rename fields inside all variants.
  pub rename_all_fields: Option<RenameAll>,
  pub tag: Option<String>,
  pub content: Option<String>,
  pub untagged: bool,
  pub transparent: bool,
  pub default: ContainerDefault,
  pub deny_unknown_fields: bool,
  /// `#[serde(from = "Type")]`
  pub from: Option<Path>,
  /// `#[serde(try_from = "Type")]`
  pub try_from: Option<Path>,
  /// `#[serde(into = "Type")]`
  pub into: Option<Path>,
  /// `#[diffable]` options — `atomic` and `transparent`
  pub diffable: DiffableOpts,
}

#[derive(Default)]
pub enum ContainerDefault {
  #[default]
  None,
  Default,
}

impl ContainerAttrs {
  pub fn from_attrs(attrs: &[Attribute]) -> Self {
    let mut out = Self::default();
    for attr in diffable_attrs(attrs).chain(typewire_attrs(attrs)) {
      let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("atomic") {
          out.diffable.atomic = true;
        } else if meta.path.is_ident("visit_transparent") {
          out.diffable.transparent = true;
        } else {
          skip_meta_value(&meta);
        }
        Ok(())
      });
    }
    for attr in serde_attrs(attrs).chain(typewire_attrs(attrs)) {
      let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("rename_all") {
          let s = get_lit_str(&meta)?;
          out.rename_all = RenameAll::parse(&s.value());
        } else if meta.path.is_ident("rename_all_fields") {
          let s = get_lit_str(&meta)?;
          out.rename_all_fields = RenameAll::parse(&s.value());
        } else if meta.path.is_ident("tag") {
          let s = get_lit_str(&meta)?;
          out.tag = Some(s.value());
        } else if meta.path.is_ident("content") {
          let s = get_lit_str(&meta)?;
          out.content = Some(s.value());
        } else if meta.path.is_ident("untagged") {
          out.untagged = true;
        } else if meta.path.is_ident("transparent") {
          out.transparent = true;
        } else if meta.path.is_ident("default") {
          out.default = ContainerDefault::Default;
        } else if meta.path.is_ident("deny_unknown_fields") {
          out.deny_unknown_fields = true;
        } else if meta.path.is_ident("from") {
          let s = get_lit_str(&meta)?;
          out.from = Some(s.parse()?);
        } else if meta.path.is_ident("try_from") {
          let s = get_lit_str(&meta)?;
          out.try_from = Some(s.parse()?);
        } else if meta.path.is_ident("into") {
          let s = get_lit_str(&meta)?;
          out.into = Some(s.parse()?);
        } else {
          // Skip unknown attributes (bound, remote, etc.)
          skip_meta_value(&meta);
        }
        Ok(())
      });
    }
    out
  }
}

// ---------------------------------------------------------------------------
// Variant attributes (#[serde(...)] on enum variant)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct VariantAttrs {
  pub rename: Option<String>,
  pub aliases: Vec<String>,
  pub rename_all: Option<RenameAll>,
  pub skip: SkipOpts,
  pub other: bool,
  pub untagged: bool,
}

impl VariantAttrs {
  pub fn from_attrs(attrs: &[Attribute]) -> Self {
    let mut out = Self::default();
    for attr in serde_attrs(attrs).chain(typewire_attrs(attrs)) {
      let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("rename") {
          let s = get_lit_str(&meta)?;
          out.rename = Some(s.value());
        } else if meta.path.is_ident("alias") {
          let s = get_lit_str(&meta)?;
          out.aliases.push(s.value());
        } else if meta.path.is_ident("rename_all") {
          let s = get_lit_str(&meta)?;
          out.rename_all = RenameAll::parse(&s.value());
        } else if meta.path.is_ident("skip") {
          out.skip.all = true;
        } else if meta.path.is_ident("skip_serializing") {
          out.skip.serializing = true;
        } else if meta.path.is_ident("skip_deserializing") {
          out.skip.deserializing = true;
        } else if meta.path.is_ident("other") {
          out.other = true;
        } else if meta.path.is_ident("untagged") {
          out.untagged = true;
        } else {
          skip_meta_value(&meta);
        }
        Ok(())
      });
    }
    out
  }

  pub const fn skip_serializing(&self) -> bool {
    self.skip.all || self.skip.serializing
  }

  pub const fn skip_deserializing(&self) -> bool {
    self.skip.all || self.skip.deserializing
  }
}

// ---------------------------------------------------------------------------
// Field attributes (#[serde(...)] on struct/variant fields)
// ---------------------------------------------------------------------------

#[derive(Default)]
pub struct FieldAttrs {
  pub rename: Option<String>,
  pub aliases: Vec<String>,
  pub skip: SkipOpts,
  pub default: FieldDefault,
  pub flatten: bool,
  pub skip_serializing_if: Option<String>,
  pub encoding: EncodingOpts,
  /// `#[typewire(lenient)]` -- skip errors during `from_js` instead of propagating
  pub lenient: bool,
}

#[derive(Default)]
pub enum FieldDefault {
  #[default]
  None,
  Default,
  Path(String),
}

impl FieldAttrs {
  pub fn from_attrs(attrs: &[Attribute]) -> Self {
    let mut out = Self::default();
    for attr in serde_attrs(attrs).chain(typewire_attrs(attrs)) {
      let _ = attr.parse_nested_meta(|meta| {
        if meta.path.is_ident("rename") {
          let s = get_lit_str(&meta)?;
          out.rename = Some(s.value());
        } else if meta.path.is_ident("alias") {
          let s = get_lit_str(&meta)?;
          out.aliases.push(s.value());
        } else if meta.path.is_ident("skip") {
          out.skip.all = true;
        } else if meta.path.is_ident("skip_serializing") {
          out.skip.serializing = true;
        } else if meta.path.is_ident("skip_deserializing") {
          out.skip.deserializing = true;
        } else if meta.path.is_ident("default") {
          if let Ok(s) = get_lit_str(&meta) {
            out.default = FieldDefault::Path(s.value());
          } else {
            out.default = FieldDefault::Default;
          }
        } else if meta.path.is_ident("flatten") {
          out.flatten = true;
        } else if meta.path.is_ident("skip_serializing_if") {
          let s = get_lit_str(&meta)?;
          out.skip_serializing_if = Some(s.value());
        } else if meta.path.is_ident("with") {
          let s = get_lit_str(&meta)?;
          if s.value() == "serde_bytes" {
            out.encoding.serde_bytes = true;
          }
        } else if meta.path.is_ident("base64") {
          out.encoding.base64 = true;
        } else if meta.path.is_ident("display") {
          out.encoding.display = true;
        } else if meta.path.is_ident("lenient") {
          out.lenient = true;
        } else {
          // Skip serialize_with, deserialize_with, bound, etc.
          skip_meta_value(&meta);
        }
        Ok(())
      });
    }
    out
  }

  pub const fn skip_serializing(&self) -> bool {
    self.skip.all || self.skip.serializing
  }

  pub const fn skip_deserializing(&self) -> bool {
    self.skip.all || self.skip.deserializing
  }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn serde_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
  attrs.iter().filter(|a| a.path().is_ident("serde"))
}

fn diffable_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
  attrs.iter().filter(|a| a.path().is_ident("diffable"))
}

fn typewire_attrs(attrs: &[Attribute]) -> impl Iterator<Item = &Attribute> {
  attrs.iter().filter(|a| a.path().is_ident("typewire"))
}

fn get_lit_str(meta: &ParseNestedMeta<'_>) -> syn::Result<LitStr> {
  let expr: Expr = meta.value()?.parse()?;
  match expr {
    Expr::Lit(ExprLit { lit: Lit::Str(s), .. }) => Ok(s),
    _ => Err(meta.error("expected string literal")),
  }
}

fn skip_meta_value(meta: &ParseNestedMeta<'_>) {
  // Consume the `= "..."` or `(...)` if present, so the parser advances.
  if let Ok(value) = meta.value() {
    let _ = value.parse::<Expr>();
  }
}
