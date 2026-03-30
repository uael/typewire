//! Integration tests verifying that `#[derive(Typewire)]` produces ident values
//! that are byte-compatible with the decode pipeline.
//!
//! These tests bridge the gap between the derive (which runs at compile time)
//! and the decoder (which runs at runtime), ensuring the binary format is
//! consistent end-to-end.

use typewire::{Typewire, schema::zerocopy::IntoBytes};

// ---------------------------------------------------------------------------
// Derive produces correct ident types
// ---------------------------------------------------------------------------

#[derive(Clone, Typewire)]
#[expect(dead_code, reason = "type is only used to test derive output")]
struct Point {
  x: f32,
  y: f32,
}

#[test]
fn derived_struct_ident_is_named() {
  // The derive should produce Ident<5> with value "Point"
  let ident = <Point as Typewire>::IDENT;
  let bytes = ident.as_bytes();
  // Ident<5> layout: [u32le len=5][b"Point"]
  assert_eq!(bytes.len(), 4 + 5);
  assert_eq!(&bytes[0..4], &5u32.to_le_bytes());
  assert_eq!(&bytes[4..9], b"Point");
}

#[derive(Clone, Typewire)]
#[serde(transparent)]
#[expect(dead_code, reason = "type is only used to test derive output")]
struct Wrapper(f32);

#[test]
fn derived_transparent_ident() {
  let ident = <Wrapper as Typewire>::IDENT;
  let bytes = ident.as_bytes();
  assert_eq!(&bytes[0..4], &7u32.to_le_bytes());
  assert_eq!(&bytes[4..11], b"Wrapper");
}

#[derive(Clone, PartialEq, Typewire)]
#[expect(dead_code, reason = "type is only used to test derive output")]
enum Direction {
  Up,
  Down,
}

#[test]
fn derived_enum_ident() {
  let ident = <Direction as Typewire>::IDENT;
  let bytes = ident.as_bytes();
  assert_eq!(&bytes[0..4], &9u32.to_le_bytes());
  assert_eq!(&bytes[4..13], b"Direction");
}

// ---------------------------------------------------------------------------
// Built-in impls produce correct idents
// ---------------------------------------------------------------------------

#[test]
fn primitive_ident_is_tagged() {
  let ident = <f32 as Typewire>::IDENT;
  let bytes = ident.as_bytes();
  // PrimitiveIdent layout: [Tag::Primitive=1][Scalar::f32=13]
  assert_eq!(bytes, &[1, 13]);
}

#[test]
fn option_ident_wraps_inner() {
  let ident = <Option<f32> as Typewire>::IDENT;
  let bytes = ident.as_bytes();
  // OptionIdent<PrimitiveIdent> layout: [Tag::Option=2][Tag::Primitive=1][Scalar::f32=13]
  assert_eq!(bytes, &[2, 1, 13]);
}

#[test]
fn vec_ident_wraps_element() {
  let ident = <Vec<bool> as Typewire>::IDENT;
  let bytes = ident.as_bytes();
  // SeqIdent<PrimitiveIdent> layout: [Tag::Seq=4][Tag::Primitive=1][Scalar::bool=0]
  assert_eq!(bytes, &[4, 1, 0]);
}

#[test]
fn box_ident_matches_inner() {
  let ident = <Box<f64> as Typewire>::IDENT;
  let bytes = ident.as_bytes();
  // Box<T> on the wire is just T, so the ident is T::Ident directly.
  // PrimitiveIdent layout: [Tag::Primitive=1][Scalar::f64=14]
  assert_eq!(bytes, &[1, 14]);
}

// ---------------------------------------------------------------------------
// Derived struct link-section record roundtrips through decode
// ---------------------------------------------------------------------------

#[derive(Clone, Typewire)]
#[expect(dead_code, reason = "type is only used to test derive output")]
struct Center {
  x: f32,
  y: f32,
}

#[test]
#[cfg(feature = "cli")]
fn derived_struct_record_decodes() {
  // Build a Record<FlatStruct<...>> manually using the derived IDENT,
  // then verify decode::parse_section can read it.
  //
  // This test requires the `cli` feature (which enables `typescript`
  // and transitively `codegen`) so the decode module is available.
  // The link-section static produced by #[derive(Typewire)] on Center
  // is available as a symbol, but we can't easily access it at runtime.
  // Instead, verify the IDENT bytes are compatible with read_type_ident
  // by embedding them in a hand-crafted section alongside the struct's
  // FlatField records.
  use typewire::schema::{FieldFlags, StructFlags, coded::*, decode};

  type Fields =
    Types2<FlatField<1, 1, <f32 as Typewire>::Ident>, FlatField<1, 1, <f32 as Typewire>::Ident>>;

  let record: Record<FlatStruct<6, Fields>> = Record::new(FlatStruct {
    tag: Tag::Struct,
    ident: Ident::new(*b"Center"),
    flags: StructFlags::empty(),
    shape: StructShapeTag::Named,
    generic_count: U32Le::new(0),
    field_count: U32Le::new(2),
    fields: Types2(
      FlatField {
        ident: Ident::new(*b"x"),
        ty: <f32 as Typewire>::IDENT,
        wire_name: Ident::new(*b"x"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::None,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      },
      FlatField {
        ident: Ident::new(*b"y"),
        ty: <f32 as Typewire>::IDENT,
        wire_name: Ident::new(*b"y"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::None,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      },
    ),
  });

  let bytes = record.as_bytes();
  let schemas = decode::parse_section(bytes).unwrap();
  assert_eq!(schemas.len(), 1);
  assert_eq!(schemas[0].ident(), Some("Center"));
}
