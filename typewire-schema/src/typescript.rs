//! TypeScript declaration emitter.
//!
//! Generates `.d.ts` content from [`Schema`] values parsed by
//! [`decode::parse_section`](crate::decode::parse_section).

use std::fmt::Write;

use crate::{
  Enum, EnumFlags, FieldDefault, FieldFlags, Scalar, Schema, Struct, StructShape, Tagging,
  Transparent, Variant, VariantFlags, VariantKind, decode,
};

/// Generate TypeScript declarations from raw `typewire_schemas` section bytes.
///
/// # Errors
///
/// Returns an error if the section contains malformed records.
pub fn generate(data: &[u8]) -> Result<String, decode::Error> {
  let schemas = decode::parse_section(data)?;

  // Collect and sort named type definitions for deterministic output
  let mut named: Vec<&Schema> = schemas
    .iter()
    .filter(|s| s.ident().is_some_and(|name| !name.is_empty()) && !s.is_type_ref())
    .collect();
  named.sort_by_key(|s| s.ident().unwrap_or(""));

  let mut out = String::new();
  for schema in &named {
    match schema {
      Schema::Struct(s) => emit_struct(s, &mut out),
      Schema::Transparent(t) => emit_transparent(t, &mut out),
      Schema::Enum(e) => emit_enum(e, &mut out),
      Schema::IntoProxy(p) => {
        let ts = ty_to_ts(&Schema::Native(p.into_ty.clone()));
        let _ = writeln!(out, "export type {} = {ts};\n", p.ident);
      }
      Schema::FromProxy(p) => {
        let ts = ty_to_ts(&Schema::Native(p.proxy.clone()));
        let _ = writeln!(out, "export type {} = {ts};\n", p.ident);
      }
      _ => {}
    }
  }

  Ok(out)
}

fn emit_struct(s: &Struct, out: &mut String) {
  match &s.shape {
    StructShape::Named(fields) => {
      let _ = writeln!(out, "export interface {} {{", s.ident);
      for f in fields {
        if f.flags.intersects(FieldFlags::SKIP_SER | FieldFlags::SKIP_DE) {
          continue;
        }
        let ts_type = ty_to_ts(&f.ty);
        let optional = !matches!(f.default, FieldDefault::None);
        let opt = if optional { "?" } else { "" };
        let _ = writeln!(out, "  {}{}: {};", f.wire_name, opt, ts_type);
      }
      let _ = writeln!(out, "}}\n");
    }
    StructShape::Tuple(idents) => {
      let types: Vec<_> = idents.iter().map(|t| ty_to_ts(t)).collect();
      let _ = writeln!(out, "export type {} = [{}];\n", s.ident, types.join(", "));
    }
    StructShape::Unit => {
      let _ = writeln!(out, "export type {} = null;\n", s.ident);
    }
  }
}

fn emit_transparent(t: &Transparent, out: &mut String) {
  let ts_type = ty_to_ts(&t.field_ty);
  let _ = writeln!(out, "export type {} = {};\n", t.ident, ts_type);
}

fn emit_enum(e: &Enum, out: &mut String) {
  if e.flags.contains(EnumFlags::ALL_UNIT) {
    let variants: Vec<_> = e
      .variants
      .iter()
      .filter(|v| !v.flags.intersects(VariantFlags::SKIP_SER | VariantFlags::SKIP_DE))
      .map(|v| format!("\"{}\"", v.wire_name))
      .collect();
    let _ = writeln!(out, "export type {} = {};\n", e.ident, variants.join(" | "));
    return;
  }

  let mut members = Vec::new();
  for v in &e.variants {
    if v.flags.intersects(VariantFlags::SKIP_SER | VariantFlags::SKIP_DE) {
      continue;
    }
    let member = match &e.tagging {
      Tagging::External => emit_ext_tagged_variant(v),
      Tagging::Internal { tag } => emit_int_tagged_variant(v, tag),
      Tagging::Adjacent { tag, content } => emit_adj_tagged_variant(v, tag, content),
      Tagging::Untagged => emit_untagged_variant(v),
    };
    members.push(member);
  }

  let _ = writeln!(out, "export type {} = {};\n", e.ident, members.join(" | "));
}

fn emit_ext_tagged_variant(v: &Variant) -> String {
  match &v.kind {
    VariantKind::Unit => format!("\"{}\"", v.wire_name),
    VariantKind::Named(fields) => {
      let inner: Vec<_> =
        fields.iter().map(|f| format!("{}: {}", f.wire_name, ty_to_ts(&f.ty))).collect();
      format!("{{ \"{}\": {{ {} }} }}", v.wire_name, inner.join("; "))
    }
    VariantKind::Unnamed(idents) => {
      if idents.len() == 1 {
        format!("{{ \"{}\": {} }}", v.wire_name, ty_to_ts(&idents[0]))
      } else {
        let types: Vec<_> = idents.iter().map(|t| ty_to_ts(t)).collect();
        format!("{{ \"{}\": [{}] }}", v.wire_name, types.join(", "))
      }
    }
  }
}

fn emit_int_tagged_variant(v: &Variant, tag_key: &str) -> String {
  match &v.kind {
    VariantKind::Unit => {
      format!("{{ {tag_key}: \"{}\" }}", v.wire_name)
    }
    VariantKind::Named(fields) => {
      let mut parts = vec![format!("{tag_key}: \"{}\"", v.wire_name)];
      for f in fields {
        parts.push(format!("{}: {}", f.wire_name, ty_to_ts(&f.ty)));
      }
      format!("{{ {} }}", parts.join("; "))
    }
    VariantKind::Unnamed(idents) => {
      if idents.len() == 1 {
        format!("{{ {tag_key}: \"{}\" }} & {}", v.wire_name, ty_to_ts(&idents[0]))
      } else {
        format!("{{ {tag_key}: \"{}\" }}", v.wire_name)
      }
    }
  }
}

fn emit_adj_tagged_variant(v: &Variant, tag_key: &str, content_key: &str) -> String {
  match &v.kind {
    VariantKind::Unit => {
      format!("{{ {tag_key}: \"{}\" }}", v.wire_name)
    }
    VariantKind::Named(fields) => {
      let inner: Vec<_> =
        fields.iter().map(|f| format!("{}: {}", f.wire_name, ty_to_ts(&f.ty))).collect();
      format!("{{ {tag_key}: \"{}\"; {content_key}: {{ {} }} }}", v.wire_name, inner.join("; "))
    }
    VariantKind::Unnamed(idents) => {
      if idents.len() == 1 {
        format!("{{ {tag_key}: \"{}\"; {content_key}: {} }}", v.wire_name, ty_to_ts(&idents[0]))
      } else {
        let types: Vec<_> = idents.iter().map(|t| ty_to_ts(t)).collect();
        format!("{{ {tag_key}: \"{}\"; {content_key}: [{}] }}", v.wire_name, types.join(", "))
      }
    }
  }
}

fn emit_untagged_variant(v: &Variant) -> String {
  match &v.kind {
    VariantKind::Unit => "null".to_string(),
    VariantKind::Named(fields) => {
      let parts: Vec<_> =
        fields.iter().map(|f| format!("{}: {}", f.wire_name, ty_to_ts(&f.ty))).collect();
      format!("{{ {} }}", parts.join("; "))
    }
    VariantKind::Unnamed(idents) => {
      if idents.len() == 1 {
        ty_to_ts(&idents[0])
      } else {
        let types: Vec<_> = idents.iter().map(|t| ty_to_ts(t)).collect();
        format!("[{}]", types.join(", "))
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Type resolution from Schema used as type reference
// ---------------------------------------------------------------------------

fn ty_to_ts(schema: &Schema) -> String {
  match schema {
    Schema::Native(name) => name.clone(),
    Schema::Primitive(scalar) => scalar_to_ts(*scalar).to_string(),
    Schema::Option(inner) => {
      let ts = ty_to_ts(inner);
      format!("{ts} | null")
    }
    Schema::Box(inner) => ty_to_ts(inner),
    Schema::Seq(element) => {
      let ts = ty_to_ts(element);
      format!("{}[]", wrap_if_union(&ts))
    }
    Schema::Map { key, value } => {
      let key_ts = ty_to_ts(key);
      let val_ts = ty_to_ts(value);
      format!("Record<{key_ts}, {val_ts}>")
    }
    Schema::Tuple(elements) => {
      let types: Vec<_> = elements.iter().map(|t| ty_to_ts(t)).collect();
      format!("[{}]", types.join(", "))
    }
    Schema::Skipped => "unknown".to_string(),
    other => other.ident().unwrap_or("unknown").to_string(),
  }
}

const fn scalar_to_ts(scalar: Scalar) -> &'static str {
  match scalar {
    Scalar::bool => "boolean",
    Scalar::u8
    | Scalar::u16
    | Scalar::u32
    | Scalar::u64
    | Scalar::u128
    | Scalar::i8
    | Scalar::i16
    | Scalar::i32
    | Scalar::i64
    | Scalar::i128
    | Scalar::usize
    | Scalar::isize
    | Scalar::f32
    | Scalar::f64 => "number",
    Scalar::char
    | Scalar::str
    | Scalar::Url
    | Scalar::Uuid
    | Scalar::DateTime
    | Scalar::FractionalIndex => "string",
    Scalar::Unit => "null",
    Scalar::Bytes => "Uint8ClampedArray",
    Scalar::SerdeJsonValue => "any",
  }
}

fn wrap_if_union(ts: &str) -> String {
  if ts.contains('|') { format!("({ts})") } else { ts.to_string() }
}
