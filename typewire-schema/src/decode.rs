//! Decodes binary schema records back into [`Schema`] values.
//!
//! This module reads the raw bytes of a `typewire_schemas` link section
//! and reconstructs the type metadata as [`Schema`] values with owned data.
//! It is the decode stage of the [typewire pipeline](https://docs.rs/typewire#pipeline).

use crate::{
  Enum, EnumFlags, Field, FieldDefault, FieldFlags, Scalar, Schema, Struct, StructFlags,
  StructShape, Tagging, Transparent, Variant, VariantFlags, VariantKind,
  coded::{FieldDefaultKind, StructShapeTag, Tag, TaggingKind, VariantKindTag},
};

// ---------------------------------------------------------------------------
// Error
// ---------------------------------------------------------------------------

/// Error returned when decoding a `typewire_schemas` section fails.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
  #[error("unexpected EOF: needed {needed} bytes, {remaining} remaining")]
  UnexpectedEof { needed: usize, remaining: usize },
  #[error("invalid tag byte: {0}")]
  InvalidTag(u8),
  #[error("invalid scalar discriminant: {0}")]
  InvalidScalar(u8),
  #[error("invalid struct shape: {0}")]
  InvalidStructShape(u8),
  #[error("invalid tagging kind: {0}")]
  InvalidTaggingKind(u8),
  #[error("invalid variant kind: {0}")]
  InvalidVariantKind(u8),
  #[error("invalid field default kind: {0}")]
  InvalidFieldDefault(u8),
  #[error("invalid UTF-8 at offset {offset}")]
  InvalidUtf8 { offset: usize },
  #[error("truncated record at offset {offset}: declared {declared} bytes, {remaining} remaining")]
  TruncatedRecord { offset: usize, declared: usize, remaining: usize },
}

// ---------------------------------------------------------------------------
// Byte reader
// ---------------------------------------------------------------------------

struct Reader<'a> {
  data: &'a [u8],
  pos: usize,
}

impl<'a> Reader<'a> {
  const fn new(data: &'a [u8]) -> Self {
    Self { data, pos: 0 }
  }

  const fn remaining(&self) -> usize {
    self.data.len().saturating_sub(self.pos)
  }

  fn read_u8(&mut self) -> Result<u8, Error> {
    if self.pos < self.data.len() {
      let v = self.data[self.pos];
      self.pos += 1;
      Ok(v)
    } else {
      Err(Error::UnexpectedEof { needed: 1, remaining: 0 })
    }
  }

  fn read_u32_le(&mut self) -> Result<u32, Error> {
    if self.pos + 4 <= self.data.len() {
      let bytes = &self.data[self.pos..self.pos + 4];
      self.pos += 4;
      Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
    } else {
      Err(Error::UnexpectedEof { needed: 4, remaining: self.remaining() })
    }
  }

  fn read_count(&mut self) -> Result<usize, Error> {
    let n = self.read_u32_le()? as usize;
    if n > self.remaining() {
      return Err(Error::UnexpectedEof { needed: n, remaining: self.remaining() });
    }
    Ok(n)
  }

  fn read_ident_str(&mut self) -> Result<String, Error> {
    let len = self.read_count()?;
    if self.pos + len > self.data.len() {
      return Err(Error::UnexpectedEof { needed: len, remaining: self.remaining() });
    }
    let offset = self.pos;
    let s = std::str::from_utf8(&self.data[self.pos..self.pos + len])
      .map_err(|_| Error::InvalidUtf8 { offset })?
      .to_string();
    self.pos += len;
    Ok(s)
  }

  /// Read a type identity — a recursive structure dispatched by tag byte.
  ///
  /// Returns a `Schema` used as a type reference (one of the type-ref
  /// variants: Native, Primitive, Option, Box, Seq, Map, Tuple, Skipped).
  fn read_type_ident(&mut self) -> Result<Schema, Error> {
    // Disambiguation: Ident<N> starts with a u32le length whose upper
    // bytes are zero for any practical name (< 256 chars). Tag-prefixed
    // compound idents have a non-zero second byte.

    if self.remaining() < 2 {
      return Err(Error::UnexpectedEof { needed: 2, remaining: self.remaining() });
    }

    if self.remaining() >= 4 {
      let b1 = self.data[self.pos + 1];
      let b2 = self.data[self.pos + 2];
      let b3 = self.data[self.pos + 3];
      if b1 == 0 && b2 == 0 && b3 == 0 {
        let name = self.read_ident_str()?;
        return Ok(Schema::Native(name));
      }
    }

    let raw = self.read_u8()?;
    let tag = Tag::from_u8(raw).ok_or(Error::InvalidTag(raw))?;
    match tag {
      Tag::Primitive => {
        let raw = self.read_u8()?;
        let scalar = Scalar::from_u8(raw).ok_or(Error::InvalidScalar(raw))?;
        Ok(Schema::Primitive(scalar))
      }
      Tag::Option => Ok(Schema::Option(self.read_type_ident()?.into())),
      Tag::Box => Ok(Schema::Box(self.read_type_ident()?.into())),
      Tag::Seq => Ok(Schema::Seq(self.read_type_ident()?.into())),
      Tag::Map => {
        let key = self.read_type_ident()?.into();
        let value = self.read_type_ident()?.into();
        Ok(Schema::Map { key, value })
      }
      // TupleIdent reuses Tag::Struct with count + elements
      Tag::Struct => {
        let count = self.read_u8()? as usize;
        let mut elements = Vec::with_capacity(count);
        for _ in 0..count {
          elements.push(self.read_type_ident()?.into());
        }
        Ok(Schema::Tuple(elements))
      }
      Tag::Skipped => Ok(Schema::Skipped),
      _ => Err(Error::InvalidTag(raw)),
    }
  }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Parse a `typewire_schemas` section into a list of [`Schema`] definitions.
///
/// The section contains length-prefixed
/// [`coded::Record<T>`](crate::coded::Record) entries. Version
/// validation is handled separately (the CLI checks the
/// `typewire_version` section before calling this function).
///
/// # Errors
///
/// Returns an error if any record is malformed, truncated, or contains
/// invalid tags or scalars.
pub fn parse_section(data: &[u8]) -> Result<Vec<Schema>, Error> {
  let mut schemas = Vec::new();
  let mut reader = Reader::new(data);

  while reader.remaining() >= 4 {
    let offset = reader.pos;
    let record_len = reader.read_u32_le()? as usize;
    if reader.remaining() < record_len {
      return Err(Error::TruncatedRecord {
        offset,
        declared: record_len,
        remaining: reader.remaining(),
      });
    }
    let record_end = reader.pos + record_len;

    schemas.push(parse_record(&mut reader)?);
    reader.pos = record_end;
  }

  Ok(schemas)
}

// ---------------------------------------------------------------------------
// Record parsers
// ---------------------------------------------------------------------------

fn parse_record(r: &mut Reader<'_>) -> Result<Schema, Error> {
  let raw = r.read_u8()?;
  let tag = Tag::from_u8(raw).ok_or(Error::InvalidTag(raw))?;
  match tag {
    Tag::Native => Ok(Schema::Native(r.read_ident_str()?)),
    Tag::Primitive => {
      let raw = r.read_u8()?;
      Ok(Schema::Primitive(Scalar::from_u8(raw).ok_or(Error::InvalidScalar(raw))?))
    }
    Tag::Struct => parse_struct(r),
    Tag::Transparent => parse_transparent(r),
    Tag::Enum => parse_enum(r),
    Tag::IntoProxy => parse_into_proxy(r),
    Tag::FromProxy => parse_from_proxy(r),
    _ => Err(Error::InvalidTag(raw)),
  }
}

fn parse_struct(r: &mut Reader<'_>) -> Result<Schema, Error> {
  let ident = r.read_ident_str()?;
  let flags = StructFlags::from_bits_retain(r.read_u8()?);
  let raw = r.read_u8()?;
  let shape_tag = StructShapeTag::from_u8(raw).ok_or(Error::InvalidStructShape(raw))?;
  // The binary format encodes a generic-parameter count but not the names.
  // Generic types currently skip the link section entirely (see encode.rs),
  // so this is always 0 in practice. Stored as a placeholder for forward
  // compatibility.
  let generic_count = r.read_u32_le()? as usize;
  let field_count = r.read_count()?;

  let shape = match shape_tag {
    StructShapeTag::Named => {
      let mut fields = Vec::with_capacity(field_count);
      for _ in 0..field_count {
        fields.push(parse_flat_field(r)?);
      }
      StructShape::Named(fields)
    }
    StructShapeTag::Tuple => {
      let mut idents = Vec::with_capacity(field_count);
      for _ in 0..field_count {
        idents.push(r.read_type_ident()?.into());
      }
      StructShape::Tuple(idents)
    }
    StructShapeTag::Unit => StructShape::Unit,
  };

  let generics = (0..generic_count).map(|i| format!("T{i}")).collect();
  Ok(Schema::Struct(Struct { ident, generics, flags, shape }))
}

fn parse_transparent(r: &mut Reader<'_>) -> Result<Schema, Error> {
  let ident = r.read_ident_str()?;
  let atomic = r.read_u8()? != 0;
  let inner = r.read_type_ident()?;
  Ok(Schema::Transparent(Transparent {
    ident,
    generics: Vec::new(),
    atomic,
    field_ident: None,
    field_ty: inner.into(),
  }))
}

fn parse_enum(r: &mut Reader<'_>) -> Result<Schema, Error> {
  let ident = r.read_ident_str()?;
  let flags = EnumFlags::from_bits_retain(r.read_u8()?);
  let raw = r.read_u8()?;
  let tagging_kind = TaggingKind::from_u8(raw).ok_or(Error::InvalidTaggingKind(raw))?;
  let tag_key = r.read_ident_str()?;
  let content_key = r.read_ident_str()?;
  let generic_count = r.read_u32_le()? as usize;
  let variant_count = r.read_count()?;

  let tagging = match tagging_kind {
    TaggingKind::External => Tagging::External,
    TaggingKind::Internal => Tagging::Internal { tag: tag_key },
    TaggingKind::Adjacent => Tagging::Adjacent { tag: tag_key, content: content_key },
    TaggingKind::Untagged => Tagging::Untagged,
  };

  let mut variants = Vec::with_capacity(variant_count);
  for _ in 0..variant_count {
    variants.push(parse_flat_variant(r)?);
  }

  let generics = (0..generic_count).map(|i| format!("T{i}")).collect();
  Ok(Schema::Enum(Enum { ident, generics, flags, tagging, variants }))
}

fn parse_into_proxy(r: &mut Reader<'_>) -> Result<Schema, Error> {
  let ident = r.read_ident_str()?;
  let generic_count = r.read_u32_le()? as usize;
  let into_ty = r.read_type_ident()?;
  let generics = (0..generic_count).map(|i| format!("T{i}")).collect();
  Ok(Schema::IntoProxy(crate::IntoProxy { ident, generics, into_ty: into_ty.into() }))
}

fn parse_from_proxy(r: &mut Reader<'_>) -> Result<Schema, Error> {
  let ident = r.read_ident_str()?;
  let generic_count = r.read_u32_le()? as usize;
  let proxy = r.read_type_ident()?;
  let is_try = r.read_u8()? != 0;
  let generics = (0..generic_count).map(|i| format!("T{i}")).collect();
  Ok(Schema::FromProxy(crate::FromProxy { ident, generics, proxy: proxy.into(), is_try }))
}

fn parse_flat_field(r: &mut Reader<'_>) -> Result<Field, Error> {
  let ident = r.read_ident_str()?;
  let ty = r.read_type_ident()?;
  let wire_name = r.read_ident_str()?;
  let flags = FieldFlags::from_bits_retain(r.read_u8()?);
  let raw = r.read_u8()?;
  let default_kind = FieldDefaultKind::from_u8(raw).ok_or(Error::InvalidFieldDefault(raw))?;

  // The binary format does not encode the default function path, so both
  // `#[serde(default)]` and `#[serde(default = "path")]` decode as
  // `FieldDefault::Default`. TypeScript codegen cannot distinguish the two.
  let default = match default_kind {
    FieldDefaultKind::None => FieldDefault::None,
    FieldDefaultKind::Default | FieldDefaultKind::Path => FieldDefault::Default,
  };

  let alias_count = r.read_count()?;
  let mut aliases = Vec::with_capacity(alias_count);
  for _ in 0..alias_count {
    aliases.push(r.read_ident_str()?);
  }

  Ok(Field { ident, ty: ty.into(), wire_name, flags, aliases, default, skip_serializing_if: None })
}

fn parse_flat_variant(r: &mut Reader<'_>) -> Result<Variant, Error> {
  let ident = r.read_ident_str()?;
  let wire_name = r.read_ident_str()?;
  let flags = VariantFlags::from_bits_retain(r.read_u8()?);
  let raw = r.read_u8()?;
  let kind_tag = VariantKindTag::from_u8(raw).ok_or(Error::InvalidVariantKind(raw))?;
  let child_count = r.read_count()?;

  let kind = match kind_tag {
    VariantKindTag::Unit => VariantKind::Unit,
    VariantKindTag::Named => {
      let mut fields = Vec::with_capacity(child_count);
      for _ in 0..child_count {
        fields.push(parse_flat_field(r)?);
      }
      VariantKind::Named(fields)
    }
    VariantKindTag::Unnamed => {
      let mut idents = Vec::with_capacity(child_count);
      for _ in 0..child_count {
        idents.push(r.read_type_ident()?.into());
      }
      VariantKind::Unnamed(idents)
    }
  };

  let alias_count = r.read_count()?;
  let mut all_wire_names = Vec::with_capacity(1 + alias_count);
  all_wire_names.push(wire_name.clone());
  for _ in 0..alias_count {
    all_wire_names.push(r.read_ident_str()?);
  }

  Ok(Variant { ident, wire_name, all_wire_names, flags, kind })
}
