//! Tests that comptime records can be constructed, parsed, and emitted
//! correctly through the full pipeline: `Record<T>` → bytes → `Schema` → TypeScript.

#[cfg(feature = "typescript")]
mod ts {
  use typewire_schema::{
    EnumFlags, FieldDefault, FieldFlags, Scalar, Schema, Struct, StructFlags, StructShape, Tagging,
    VariantFlags, VariantKind, coded::*, decode, typescript, zerocopy::IntoBytes,
  };

  /// Helper: concatenate multiple records into a section.
  fn concat_records(records: &[&[u8]]) -> Vec<u8> {
    let mut buf = Vec::new();
    for r in records {
      buf.extend_from_slice(r);
    }
    buf
  }

  /// Helper: decode record bytes and generate TypeScript.
  fn generate_ts(bytes: &[u8]) -> String {
    let schemas = decode::parse_section(bytes).unwrap();
    typescript::generate(&schemas)
  }

  // -----------------------------------------------------------------------
  // Discriminant enums — from_u8 round-trips
  // -----------------------------------------------------------------------

  #[test]
  fn tag_from_u8_roundtrips() {
    for v in 0..=11u8 {
      assert!(Tag::from_u8(v).is_some(), "Tag::from_u8({v}) should be Some");
    }
    assert!(Tag::from_u8(12).is_none());
    assert!(Tag::from_u8(255).is_none());
    assert_eq!(Tag::from_u8(0), Some(Tag::Native));
    assert_eq!(Tag::from_u8(11), Some(Tag::Skipped));
  }

  #[test]
  fn scalar_from_u8_roundtrips() {
    for v in 0..=23u8 {
      assert!(Scalar::from_u8(v).is_some(), "Scalar::from_u8({v}) should be Some");
    }
    assert!(Scalar::from_u8(24).is_none());
    assert_eq!(Scalar::from_u8(0), Some(Scalar::bool));
    assert_eq!(Scalar::from_u8(13), Some(Scalar::f32));
  }

  #[test]
  fn tagging_kind_from_u8_roundtrips() {
    assert_eq!(TaggingKind::from_u8(0), Some(TaggingKind::External));
    assert_eq!(TaggingKind::from_u8(3), Some(TaggingKind::Untagged));
    assert!(TaggingKind::from_u8(4).is_none());
  }

  #[test]
  fn struct_shape_tag_from_u8_roundtrips() {
    assert_eq!(StructShapeTag::from_u8(0), Some(StructShapeTag::Named));
    assert_eq!(StructShapeTag::from_u8(2), Some(StructShapeTag::Unit));
    assert!(StructShapeTag::from_u8(3).is_none());
  }

  #[test]
  fn variant_kind_tag_from_u8_roundtrips() {
    assert_eq!(VariantKindTag::from_u8(0), Some(VariantKindTag::Unit));
    assert_eq!(VariantKindTag::from_u8(2), Some(VariantKindTag::Unnamed));
    assert!(VariantKindTag::from_u8(3).is_none());
  }

  #[test]
  fn field_default_kind_from_u8_roundtrips() {
    assert_eq!(FieldDefaultKind::from_u8(0), Some(FieldDefaultKind::None));
    assert_eq!(FieldDefaultKind::from_u8(2), Some(FieldDefaultKind::Path));
    assert!(FieldDefaultKind::from_u8(3).is_none());
  }

  // -----------------------------------------------------------------------
  // Primitive roundtrip
  // -----------------------------------------------------------------------

  #[test]
  fn primitive_roundtrip() {
    let record = Record::new(FlatPrimitive { tag: Tag::Primitive, scalar: Scalar::f32 });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    assert!(matches!(&schemas[0], Schema::Primitive(Scalar::f32)));
  }

  // -----------------------------------------------------------------------
  // Struct roundtrips
  // -----------------------------------------------------------------------

  #[test]
  fn named_struct_roundtrip() {
    type Fields = Types2<FlatField<1, 1, PrimitiveIdent>, FlatField<1, 1, PrimitiveIdent>>;
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
          ty: PrimitiveIdent::new(Scalar::f32),
          wire_name: Ident::new(*b"x"),
          flags: FieldFlags::empty(),
          default: FieldDefaultKind::None,
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
        FlatField {
          ident: Ident::new(*b"y"),
          ty: PrimitiveIdent::new(Scalar::f32),
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
    match &schemas[0] {
      Schema::Struct(Struct { ident, shape: StructShape::Named(fields), .. }) => {
        assert_eq!(ident, "Center");
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].wire_name, "x");
        assert_eq!(fields[1].wire_name, "y");
        assert!(matches!(*fields[0].ty, Schema::Primitive(Scalar::f32)));
      }
      other => panic!("expected named struct, got: {other:?}"),
    }
  }

  #[test]
  fn tuple_struct_roundtrip() {
    type Fields = Types2<PrimitiveIdent, PrimitiveIdent>;
    let record: Record<FlatStruct<4, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Pair"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Tuple,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(2),
      fields: Types2(PrimitiveIdent::new(Scalar::f32), PrimitiveIdent::new(Scalar::f64)),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::Struct(Struct { ident, shape: StructShape::Tuple(types), .. }) => {
        assert_eq!(ident, "Pair");
        assert_eq!(types.len(), 2);
        assert!(matches!(*types[0], Schema::Primitive(Scalar::f32)));
        assert!(matches!(*types[1], Schema::Primitive(Scalar::f64)));
      }
      other => panic!("expected tuple struct, got: {other:?}"),
    }
  }

  #[test]
  fn unit_struct_roundtrip() {
    let record: Record<FlatStruct<5>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Empty"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Unit,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(0),
      fields: Types0(),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::Struct(Struct { ident, shape: StructShape::Unit, .. }) => assert_eq!(ident, "Empty"),
      other => panic!("expected unit struct, got: {other:?}"),
    }
  }

  // -----------------------------------------------------------------------
  // Field alias roundtrip
  // -----------------------------------------------------------------------

  #[test]
  fn field_alias_roundtrip() {
    type AliasField = FlatField<4, 4, PrimitiveIdent, Types2<Ident<6>, Ident<4>>>;
    type Fields = Types1<AliasField>;
    let record: Record<FlatStruct<6, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Config"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(1),
      fields: Types1(FlatField {
        ident: Ident::new(*b"name"),
        ty: PrimitiveIdent::new(Scalar::str),
        wire_name: Ident::new(*b"name"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::None,
        alias_count: U32Le::new(2),
        aliases: Types2(Ident::new(*b"nombre"), Ident::new(*b"naam")),
      }),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::Struct(Struct { shape: StructShape::Named(fields), .. }) => {
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].wire_name, "name");
        assert_eq!(fields[0].aliases, vec!["nombre", "naam"]);
      }
      other => panic!("expected named struct, got: {other:?}"),
    }
  }

  // -----------------------------------------------------------------------
  // Variant alias roundtrip
  // -----------------------------------------------------------------------

  #[test]
  fn variant_alias_roundtrip() {
    type V1 = FlatVariant<3, 3, Types0, Types1<Ident<5>>>;
    type V2 = FlatVariant<5, 5>;
    let record: Record<FlatEnum<5, 0, 0, Types2<V1, V2>>> = Record::new(FlatEnum {
      tag: Tag::Enum,
      ident: Ident::new(*b"Color"),
      flags: EnumFlags::ALL_UNIT,
      tagging: TaggingKind::External,
      tag_key: Ident::new(*b""),
      content_key: Ident::new(*b""),
      generic_count: U32Le::new(0),
      variant_count: U32Le::new(2),
      variants: Types2(
        FlatVariant {
          ident: Ident::new(*b"Red"),
          wire_name: Ident::new(*b"red"),
          flags: VariantFlags::empty(),
          kind: VariantKindTag::Unit,
          child_count: U32Le::new(0),
          fields: Types0(),
          alias_count: U32Le::new(1),
          aliases: Types1(Ident::new(*b"rouge")),
        },
        FlatVariant {
          ident: Ident::new(*b"Green"),
          wire_name: Ident::new(*b"green"),
          flags: VariantFlags::empty(),
          kind: VariantKindTag::Unit,
          child_count: U32Le::new(0),
          fields: Types0(),
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
      ),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::Enum(e) => {
        assert_eq!(e.variants.len(), 2);
        assert_eq!(e.variants[0].wire_name, "red");
        assert_eq!(e.variants[0].all_wire_names, vec!["red", "rouge"]);
        assert_eq!(e.variants[1].wire_name, "green");
        assert_eq!(e.variants[1].all_wire_names, vec!["green"]);
      }
      other => panic!("expected enum, got: {other:?}"),
    }
  }

  // -----------------------------------------------------------------------
  // Transparent roundtrip
  // -----------------------------------------------------------------------

  #[test]
  fn transparent_roundtrip() {
    let record: Record<FlatTransparent<6, PrimitiveIdent>> = Record::new(FlatTransparent {
      tag: Tag::Transparent,
      ident: Ident::new(*b"UserId"),
      atomic: 0,
      inner: PrimitiveIdent::new(Scalar::str),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::Transparent(t) => {
        assert_eq!(t.ident, "UserId");
        assert!(!t.atomic);
        assert!(matches!(*t.field_ty, Schema::Primitive(Scalar::str)));
      }
      other => panic!("expected transparent, got: {other:?}"),
    }
  }

  #[test]
  fn transparent_atomic_roundtrip() {
    let record: Record<FlatTransparent<5, PrimitiveIdent>> = Record::new(FlatTransparent {
      tag: Tag::Transparent,
      ident: Ident::new(*b"Score"),
      atomic: 1,
      inner: PrimitiveIdent::new(Scalar::f64),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::Transparent(t) => {
        assert_eq!(t.ident, "Score");
        assert!(t.atomic);
      }
      other => panic!("expected transparent, got: {other:?}"),
    }
  }

  // -----------------------------------------------------------------------
  // Enum roundtrips — all 4 tagging strategies
  // -----------------------------------------------------------------------

  /// Helper: build an externally-tagged enum with 2 unit variants.
  const fn build_unit_enum_record()
  -> Record<FlatEnum<5, 0, 0, Types2<FlatVariant<3, 3>, FlatVariant<5, 5>>>> {
    Record::new(FlatEnum {
      tag: Tag::Enum,
      ident: Ident::new(*b"Color"),
      flags: EnumFlags::ALL_UNIT,
      tagging: TaggingKind::External,
      tag_key: Ident::new(*b""),
      content_key: Ident::new(*b""),
      generic_count: U32Le::new(0),
      variant_count: U32Le::new(2),
      variants: Types2(
        FlatVariant {
          ident: Ident::new(*b"Red"),
          wire_name: Ident::new(*b"red"),
          flags: VariantFlags::empty(),
          kind: VariantKindTag::Unit,
          child_count: U32Le::new(0),
          fields: Types0(),
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
        FlatVariant {
          ident: Ident::new(*b"Green"),
          wire_name: Ident::new(*b"green"),
          flags: VariantFlags::empty(),
          kind: VariantKindTag::Unit,
          child_count: U32Le::new(0),
          fields: Types0(),
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
      ),
    })
  }

  #[test]
  fn enum_external_unit_roundtrip() {
    let record = build_unit_enum_record();
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::Enum(e) => {
        assert_eq!(e.ident, "Color");
        assert!(e.flags.contains(EnumFlags::ALL_UNIT));
        assert_eq!(e.tagging, Tagging::External);
        assert_eq!(e.variants.len(), 2);
        assert_eq!(e.variants[0].wire_name, "red");
        assert_eq!(e.variants[1].wire_name, "green");
        assert!(matches!(e.variants[0].kind, VariantKind::Unit));
      }
      other => panic!("expected enum, got: {other:?}"),
    }
  }

  #[test]
  fn enum_internal_tagged_roundtrip() {
    type V = FlatVariant<4, 4, Types1<FlatField<5, 5, PrimitiveIdent>>>;
    let record: Record<FlatEnum<5, 4, 0, Types1<V>>> = Record::new(FlatEnum {
      tag: Tag::Enum,
      ident: Ident::new(*b"Event"),
      flags: EnumFlags::empty(),
      tagging: TaggingKind::Internal,
      tag_key: Ident::new(*b"type"),
      content_key: Ident::new(*b""),
      generic_count: U32Le::new(0),
      variant_count: U32Le::new(1),
      variants: Types1(FlatVariant {
        ident: Ident::new(*b"Move"),
        wire_name: Ident::new(*b"move"),
        flags: VariantFlags::empty(),
        kind: VariantKindTag::Named,
        child_count: U32Le::new(1),
        fields: Types1(FlatField {
          ident: Ident::new(*b"speed"),
          ty: PrimitiveIdent::new(Scalar::f64),
          wire_name: Ident::new(*b"speed"),
          flags: FieldFlags::empty(),
          default: FieldDefaultKind::None,
          alias_count: U32Le::new(0),
          aliases: Types0(),
        }),
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });

    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::Enum(e) => {
        assert_eq!(e.ident, "Event");
        assert_eq!(e.tagging, Tagging::Internal { tag: "type".into() });
        assert_eq!(e.variants.len(), 1);
        assert_eq!(e.variants[0].wire_name, "move");
        match &e.variants[0].kind {
          VariantKind::Named(fields) => {
            assert_eq!(fields.len(), 1);
            assert_eq!(fields[0].wire_name, "speed");
          }
          other => panic!("expected Named variant, got: {other:?}"),
        }
      }
      other => panic!("expected enum, got: {other:?}"),
    }
  }

  #[test]
  fn enum_adjacent_tagged_roundtrip() {
    type V = FlatVariant<4, 4, Types1<PrimitiveIdent>>;
    let record: Record<FlatEnum<6, 1, 4, Types1<V>>> = Record::new(FlatEnum {
      tag: Tag::Enum,
      ident: Ident::new(*b"Action"),
      flags: EnumFlags::empty(),
      tagging: TaggingKind::Adjacent,
      tag_key: Ident::new(*b"t"),
      content_key: Ident::new(*b"data"),
      generic_count: U32Le::new(0),
      variant_count: U32Le::new(1),
      variants: Types1(FlatVariant {
        ident: Ident::new(*b"Zoom"),
        wire_name: Ident::new(*b"zoom"),
        flags: VariantFlags::empty(),
        kind: VariantKindTag::Unnamed,
        child_count: U32Le::new(1),
        fields: Types1(PrimitiveIdent::new(Scalar::f64)),
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });

    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::Enum(e) => {
        assert_eq!(e.tagging, Tagging::Adjacent { tag: "t".into(), content: "data".into() });
        match &e.variants[0].kind {
          VariantKind::Unnamed(types) => {
            assert_eq!(types.len(), 1);
            assert!(matches!(*types[0], Schema::Primitive(Scalar::f64)));
          }
          other => panic!("expected Unnamed variant, got: {other:?}"),
        }
      }
      other => panic!("expected enum, got: {other:?}"),
    }
  }

  #[test]
  fn enum_untagged_roundtrip() {
    let record: Record<FlatEnum<5, 0, 0, Types1<FlatVariant<4, 4>>>> = Record::new(FlatEnum {
      tag: Tag::Enum,
      ident: Ident::new(*b"Value"),
      flags: EnumFlags::empty(),
      tagging: TaggingKind::Untagged,
      tag_key: Ident::new(*b""),
      content_key: Ident::new(*b""),
      generic_count: U32Le::new(0),
      variant_count: U32Le::new(1),
      variants: Types1(FlatVariant {
        ident: Ident::new(*b"None"),
        wire_name: Ident::new(*b"None"),
        flags: VariantFlags::empty(),
        kind: VariantKindTag::Unit,
        child_count: U32Le::new(0),
        fields: Types0(),
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });

    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::Enum(e) => {
        assert_eq!(e.tagging, Tagging::Untagged);
      }
      other => panic!("expected enum, got: {other:?}"),
    }
  }

  // -----------------------------------------------------------------------
  // Proxy roundtrips
  // -----------------------------------------------------------------------

  #[test]
  fn into_proxy_roundtrip() {
    // IntoProxy: MyType serializes via MyDto
    let record: Record<FlatIntoProxy<6, Ident<5>>> = Record::new(FlatIntoProxy {
      tag: Tag::IntoProxy,
      ident: Ident::new(*b"MyType"),
      generic_count: U32Le::new(0),
      into_ty: Ident::new(*b"MyDto"),
    });

    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::IntoProxy(p) => {
        assert_eq!(p.ident, "MyType");
        assert_eq!(*p.into_ty, Schema::Native("MyDto".into()));
      }
      other => panic!("expected IntoProxy, got: {other:?}"),
    }
  }

  #[test]
  fn from_proxy_roundtrip() {
    let record: Record<FlatFromProxy<7, Ident<7>>> = Record::new(FlatFromProxy {
      tag: Tag::FromProxy,
      ident: Ident::new(*b"MyModel"),
      generic_count: U32Le::new(0),
      proxy: Ident::new(*b"MyProxy"),
      is_try: 1,
    });

    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::FromProxy(p) => {
        assert_eq!(p.ident, "MyModel");
        assert_eq!(*p.proxy, Schema::Native("MyProxy".into()));
        assert!(p.is_try);
      }
      other => panic!("expected FromProxy, got: {other:?}"),
    }
  }

  // -----------------------------------------------------------------------
  // Compound type ident roundtrips
  // -----------------------------------------------------------------------

  #[test]
  fn option_ident_roundtrip() {
    // Struct with a single Option<f32> field
    type TyIdent = OptionIdent<PrimitiveIdent>;
    type Fields = Types1<FlatField<1, 1, TyIdent>>;
    let record: Record<FlatStruct<3, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Opt"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(1),
      fields: Types1(FlatField {
        ident: Ident::new(*b"v"),
        ty: OptionIdent::new(PrimitiveIdent::new(Scalar::f32)),
        wire_name: Ident::new(*b"v"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::Default,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    assert_eq!(schemas.len(), 1);
    match &schemas[0] {
      Schema::Struct(s) => {
        let fields = match &s.shape {
          StructShape::Named(f) => f,
          other => panic!("expected Named, got: {other:?}"),
        };
        assert_eq!(fields.len(), 1);
        match &*fields[0].ty {
          Schema::Option(inner) => {
            assert!(matches!(**inner, Schema::Primitive(Scalar::f32)));
          }
          other => panic!("expected Option type, got: {other:?}"),
        }
        assert!(matches!(fields[0].default, FieldDefault::Default));
      }
      other => panic!("expected struct, got: {other:?}"),
    }
  }

  #[test]
  fn seq_ident_roundtrip() {
    // Struct with a Vec<bool> field
    type TyIdent = SeqIdent<PrimitiveIdent>;
    type Fields = Types1<FlatField<4, 5, TyIdent>>;
    let record: Record<FlatStruct<4, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"List"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(1),
      fields: Types1(FlatField {
        ident: Ident::new(*b"data"),
        ty: SeqIdent::new(PrimitiveIdent::new(Scalar::bool)),
        wire_name: Ident::new(*b"items"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::None,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    match &schemas[0] {
      Schema::Struct(s) => {
        let fields = match &s.shape {
          StructShape::Named(f) => f,
          other => panic!("expected Named, got: {other:?}"),
        };
        // Check wire name differs from ident
        assert_eq!(fields[0].ident, "data");
        assert_eq!(fields[0].wire_name, "items");
        assert!(matches!(*fields[0].ty, Schema::Seq(_)));
      }
      other => panic!("expected struct, got: {other:?}"),
    }
  }

  #[test]
  fn map_ident_roundtrip() {
    // Struct with a HashMap<String, u32> field
    type TyIdent = MapIdent<PrimitiveIdent, PrimitiveIdent>;
    type Fields = Types1<FlatField<3, 3, TyIdent>>;
    let record: Record<FlatStruct<3, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Bag"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(1),
      fields: Types1(FlatField {
        ident: Ident::new(*b"map"),
        ty: MapIdent::new(PrimitiveIdent::new(Scalar::str), PrimitiveIdent::new(Scalar::u32)),
        wire_name: Ident::new(*b"map"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::None,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    match &schemas[0] {
      Schema::Struct(s) => {
        let fields = match &s.shape {
          StructShape::Named(f) => f,
          other => panic!("expected Named, got: {other:?}"),
        };
        match &*fields[0].ty {
          Schema::Map { key, value } => {
            assert!(matches!(**key, Schema::Primitive(Scalar::str)));
            assert!(matches!(**value, Schema::Primitive(Scalar::u32)));
          }
          other => panic!("expected Map type, got: {other:?}"),
        }
      }
      other => panic!("expected struct, got: {other:?}"),
    }
  }

  #[test]
  fn box_ident_roundtrip() {
    // OptionIdent has the same layout as the removed BoxIdent (tag + inner),
    // so we reuse it with Tag::Box to test backward-compatible decoding.
    type TyIdent = OptionIdent<Ident<4>>;
    type Fields = Types1<FlatField<5, 5, TyIdent>>;
    let record: Record<FlatStruct<4, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Tree"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(1),
      fields: Types1(FlatField {
        ident: Ident::new(*b"child"),
        ty: OptionIdent { tag: Tag::Box, inner: Ident::new(*b"Tree") },
        wire_name: Ident::new(*b"child"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::None,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    match &schemas[0] {
      Schema::Struct(s) => {
        let fields = match &s.shape {
          StructShape::Named(f) => f,
          other => panic!("expected Named, got: {other:?}"),
        };
        match &*fields[0].ty {
          Schema::Box(inner) => {
            assert!(matches!(**inner, Schema::Native(ref n) if n == "Tree"));
          }
          other => panic!("expected Box type, got: {other:?}"),
        }
      }
      other => panic!("expected struct, got: {other:?}"),
    }
  }

  #[test]
  fn skipped_field_roundtrip() {
    type Fields = Types1<FlatField<7, 7, SkippedIdent>>;
    let record: Record<FlatStruct<4, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Skip"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(1),
      fields: Types1(FlatField {
        ident: Ident::new(*b"skipped"),
        ty: SkippedIdent::SKIPPED,
        wire_name: Ident::new(*b"skipped"),
        flags: FieldFlags::SKIP_SER | FieldFlags::SKIP_DE,
        default: FieldDefaultKind::None,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    match &schemas[0] {
      Schema::Struct(s) => {
        let fields = match &s.shape {
          StructShape::Named(f) => f,
          other => panic!("expected Named, got: {other:?}"),
        };
        assert_eq!(fields.len(), 1);
        assert!(matches!(*fields[0].ty, Schema::Skipped));
        assert!(fields[0].flags.contains(FieldFlags::SKIP_SER | FieldFlags::SKIP_DE));
      }
      other => panic!("expected struct, got: {other:?}"),
    }
  }

  // -----------------------------------------------------------------------
  // Field flags and defaults
  // -----------------------------------------------------------------------

  #[test]
  fn field_with_default_roundtrip() {
    type Fields = Types1<FlatField<1, 1, PrimitiveIdent>>;
    let record: Record<FlatStruct<7, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Default"),
      flags: StructFlags::CONTAINER_DEFAULT,
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(1),
      fields: Types1(FlatField {
        ident: Ident::new(*b"n"),
        ty: PrimitiveIdent::new(Scalar::i32),
        wire_name: Ident::new(*b"n"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::Default,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let bytes = record.as_bytes();
    let schemas = decode::parse_section(bytes).unwrap();

    match &schemas[0] {
      Schema::Struct(s) => {
        assert!(s.flags.contains(StructFlags::CONTAINER_DEFAULT));
        let fields = match &s.shape {
          StructShape::Named(f) => f,
          other => panic!("expected Named, got: {other:?}"),
        };
        assert!(matches!(fields[0].default, FieldDefault::Default));
      }
      other => panic!("expected struct, got: {other:?}"),
    }
  }

  // -----------------------------------------------------------------------
  // TypeScript generation
  // -----------------------------------------------------------------------

  #[test]
  fn ts_named_struct() {
    type Fields = Types2<FlatField<1, 1, PrimitiveIdent>, FlatField<1, 1, PrimitiveIdent>>;
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
          ty: PrimitiveIdent::new(Scalar::f32),
          wire_name: Ident::new(*b"x"),
          flags: FieldFlags::empty(),
          default: FieldDefaultKind::None,
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
        FlatField {
          ident: Ident::new(*b"y"),
          ty: PrimitiveIdent::new(Scalar::f32),
          wire_name: Ident::new(*b"y"),
          flags: FieldFlags::empty(),
          default: FieldDefaultKind::None,
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
      ),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("export interface Center {"), "got:\n{ts}");
    assert!(ts.contains("x: number;"), "got:\n{ts}");
    assert!(ts.contains("y: number;"), "got:\n{ts}");
  }

  #[test]
  fn ts_optional_field() {
    type Fields = Types1<FlatField<1, 1, OptionIdent<PrimitiveIdent>>>;
    let record: Record<FlatStruct<3, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Opt"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(1),
      fields: Types1(FlatField {
        ident: Ident::new(*b"v"),
        ty: OptionIdent::new(PrimitiveIdent::new(Scalar::str)),
        wire_name: Ident::new(*b"v"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::Default,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("v?: string | null;"), "got:\n{ts}");
  }

  #[test]
  fn ts_tuple_struct() {
    type Fields = Types2<PrimitiveIdent, PrimitiveIdent>;
    let record: Record<FlatStruct<4, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Pair"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Tuple,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(2),
      fields: Types2(PrimitiveIdent::new(Scalar::f32), PrimitiveIdent::new(Scalar::f64)),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("export type Pair = [number, number];"), "got:\n{ts}");
  }

  #[test]
  fn ts_unit_struct() {
    let record: Record<FlatStruct<5>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Empty"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Unit,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(0),
      fields: Types0(),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("export type Empty = null;"), "got:\n{ts}");
  }

  #[test]
  fn ts_transparent() {
    let record: Record<FlatTransparent<6, PrimitiveIdent>> = Record::new(FlatTransparent {
      tag: Tag::Transparent,
      ident: Ident::new(*b"UserId"),
      atomic: 0,
      inner: PrimitiveIdent::new(Scalar::str),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("export type UserId = string;"), "got:\n{ts}");
  }

  #[test]
  fn ts_all_unit_enum() {
    let record = build_unit_enum_record();
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("export type Color = \"red\" | \"green\";"), "got:\n{ts}");
  }

  #[test]
  fn ts_externally_tagged_enum() {
    type V1 = FlatVariant<4, 4>;
    type V2 = FlatVariant<4, 4, Types1<FlatField<5, 5, PrimitiveIdent>>>;
    let record: Record<FlatEnum<5, 0, 0, Types2<V1, V2>>> = Record::new(FlatEnum {
      tag: Tag::Enum,
      ident: Ident::new(*b"Shape"),
      flags: EnumFlags::empty(),
      tagging: TaggingKind::External,
      tag_key: Ident::new(*b""),
      content_key: Ident::new(*b""),
      generic_count: U32Le::new(0),
      variant_count: U32Le::new(2),
      variants: Types2(
        FlatVariant {
          ident: Ident::new(*b"None"),
          wire_name: Ident::new(*b"none"),
          flags: VariantFlags::empty(),
          kind: VariantKindTag::Unit,
          child_count: U32Le::new(0),
          fields: Types0(),
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
        FlatVariant {
          ident: Ident::new(*b"Rect"),
          wire_name: Ident::new(*b"rect"),
          flags: VariantFlags::empty(),
          kind: VariantKindTag::Named,
          child_count: U32Le::new(1),
          fields: Types1(FlatField {
            ident: Ident::new(*b"width"),
            ty: PrimitiveIdent::new(Scalar::f32),
            wire_name: Ident::new(*b"width"),
            flags: FieldFlags::empty(),
            default: FieldDefaultKind::None,
            alias_count: U32Le::new(0),
            aliases: Types0(),
          }),
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
      ),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("\"none\""), "got:\n{ts}");
    assert!(ts.contains("\"rect\""), "got:\n{ts}");
    assert!(ts.contains("width: number"), "got:\n{ts}");
  }

  #[test]
  fn ts_internally_tagged_enum() {
    type V = FlatVariant<4, 4, Types1<FlatField<5, 5, PrimitiveIdent>>>;
    let record: Record<FlatEnum<5, 4, 0, Types1<V>>> = Record::new(FlatEnum {
      tag: Tag::Enum,
      ident: Ident::new(*b"Event"),
      flags: EnumFlags::empty(),
      tagging: TaggingKind::Internal,
      tag_key: Ident::new(*b"type"),
      content_key: Ident::new(*b""),
      generic_count: U32Le::new(0),
      variant_count: U32Le::new(1),
      variants: Types1(FlatVariant {
        ident: Ident::new(*b"Move"),
        wire_name: Ident::new(*b"move"),
        flags: VariantFlags::empty(),
        kind: VariantKindTag::Named,
        child_count: U32Le::new(1),
        fields: Types1(FlatField {
          ident: Ident::new(*b"speed"),
          ty: PrimitiveIdent::new(Scalar::f64),
          wire_name: Ident::new(*b"speed"),
          flags: FieldFlags::empty(),
          default: FieldDefaultKind::None,
          alias_count: U32Le::new(0),
          aliases: Types0(),
        }),
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("type: \"move\""), "got:\n{ts}");
    assert!(ts.contains("speed: number"), "got:\n{ts}");
  }

  #[test]
  fn ts_adjacently_tagged_enum() {
    type V = FlatVariant<4, 4, Types1<PrimitiveIdent>>;
    let record: Record<FlatEnum<6, 1, 4, Types1<V>>> = Record::new(FlatEnum {
      tag: Tag::Enum,
      ident: Ident::new(*b"Action"),
      flags: EnumFlags::empty(),
      tagging: TaggingKind::Adjacent,
      tag_key: Ident::new(*b"t"),
      content_key: Ident::new(*b"data"),
      generic_count: U32Le::new(0),
      variant_count: U32Le::new(1),
      variants: Types1(FlatVariant {
        ident: Ident::new(*b"Zoom"),
        wire_name: Ident::new(*b"zoom"),
        flags: VariantFlags::empty(),
        kind: VariantKindTag::Unnamed,
        child_count: U32Le::new(1),
        fields: Types1(PrimitiveIdent::new(Scalar::f64)),
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("t: \"zoom\""), "got:\n{ts}");
    assert!(ts.contains("data: number"), "got:\n{ts}");
  }

  #[test]
  fn ts_into_proxy() {
    let record: Record<FlatIntoProxy<6, Ident<5>>> = Record::new(FlatIntoProxy {
      tag: Tag::IntoProxy,
      ident: Ident::new(*b"MyType"),
      generic_count: U32Le::new(0),
      into_ty: Ident::new(*b"MyDto"),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("export type MyType = MyDto;"), "got:\n{ts}");
  }

  #[test]
  fn ts_from_proxy() {
    let record: Record<FlatFromProxy<7, Ident<7>>> = Record::new(FlatFromProxy {
      tag: Tag::FromProxy,
      ident: Ident::new(*b"MyModel"),
      generic_count: U32Le::new(0),
      proxy: Ident::new(*b"MyProxy"),
      is_try: 0,
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("export type MyModel = MyProxy;"), "got:\n{ts}");
  }

  #[test]
  fn ts_skipped_field_omitted() {
    type Fields = Types2<FlatField<7, 7, SkippedIdent>, FlatField<4, 4, PrimitiveIdent>>;
    let record: Record<FlatStruct<4, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Half"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(2),
      fields: Types2(
        FlatField {
          ident: Ident::new(*b"skipped"),
          ty: SkippedIdent::SKIPPED,
          wire_name: Ident::new(*b"skipped"),
          flags: FieldFlags::SKIP_SER,
          default: FieldDefaultKind::None,
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
        FlatField {
          ident: Ident::new(*b"kept"),
          ty: PrimitiveIdent::new(Scalar::bool),
          wire_name: Ident::new(*b"kept"),
          flags: FieldFlags::empty(),
          default: FieldDefaultKind::None,
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
      ),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(!ts.contains("skipped"), "skipped field should be omitted: {ts}");
    assert!(ts.contains("kept: boolean;"), "got:\n{ts}");
  }

  #[test]
  fn ts_vec_field() {
    type Fields = Types1<FlatField<5, 5, SeqIdent<PrimitiveIdent>>>;
    let record: Record<FlatStruct<5, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Items"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(1),
      fields: Types1(FlatField {
        ident: Ident::new(*b"items"),
        ty: SeqIdent::new(PrimitiveIdent::new(Scalar::str)),
        wire_name: Ident::new(*b"items"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::None,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("items: string[];"), "got:\n{ts}");
  }

  #[test]
  fn ts_map_field() {
    type TyIdent = MapIdent<PrimitiveIdent, PrimitiveIdent>;
    type Fields = Types1<FlatField<3, 3, TyIdent>>;
    let record: Record<FlatStruct<3, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"Bag"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(1),
      fields: Types1(FlatField {
        ident: Ident::new(*b"map"),
        ty: MapIdent::new(PrimitiveIdent::new(Scalar::str), PrimitiveIdent::new(Scalar::u32)),
        wire_name: Ident::new(*b"map"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::None,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("map: Record<string, number>;"), "got:\n{ts}");
  }

  // -----------------------------------------------------------------------
  // Multi-record section
  // -----------------------------------------------------------------------

  #[test]
  fn multi_record_section() {
    type Fields = Types1<FlatField<1, 1, PrimitiveIdent>>;

    let prim = Record::new(FlatPrimitive { tag: Tag::Primitive, scalar: Scalar::f32 });
    let prim_bytes = prim.as_bytes();

    let s: Record<FlatStruct<1, Fields>> = Record::new(FlatStruct {
      tag: Tag::Struct,
      ident: Ident::new(*b"A"),
      flags: StructFlags::empty(),
      shape: StructShapeTag::Named,
      generic_count: U32Le::new(0),
      field_count: U32Le::new(1),
      fields: Types1(FlatField {
        ident: Ident::new(*b"x"),
        ty: PrimitiveIdent::new(Scalar::f32),
        wire_name: Ident::new(*b"x"),
        flags: FieldFlags::empty(),
        default: FieldDefaultKind::None,
        alias_count: U32Le::new(0),
        aliases: Types0(),
      }),
    });
    let s_bytes = s.as_bytes();

    let t: Record<FlatTransparent<1, PrimitiveIdent>> = Record::new(FlatTransparent {
      tag: Tag::Transparent,
      ident: Ident::new(*b"B"),
      atomic: 0,
      inner: PrimitiveIdent::new(Scalar::bool),
    });
    let t_bytes = t.as_bytes();

    let records = concat_records(&[prim_bytes, s_bytes, t_bytes]);
    let ts = generate_ts(&records);

    // Should have both named types (primitive records are skipped in output)
    assert!(ts.contains("export interface A {"), "got:\n{ts}");
    assert!(ts.contains("export type B = boolean;"), "got:\n{ts}");
  }

  // -----------------------------------------------------------------------
  // Scalar → TypeScript mapping coverage
  // -----------------------------------------------------------------------

  #[test]
  fn all_scalar_types_map_correctly() {
    let cases: &[(Scalar, &str)] = &[
      (Scalar::bool, "boolean"),
      (Scalar::u8, "number"),
      (Scalar::u16, "number"),
      (Scalar::u32, "number"),
      (Scalar::u64, "number"),
      (Scalar::u128, "number"),
      (Scalar::i8, "number"),
      (Scalar::i16, "number"),
      (Scalar::i32, "number"),
      (Scalar::i64, "number"),
      (Scalar::i128, "number"),
      (Scalar::usize, "number"),
      (Scalar::isize, "number"),
      (Scalar::f32, "number"),
      (Scalar::f64, "number"),
      (Scalar::char, "string"),
      (Scalar::str, "string"),
      (Scalar::Unit, "null"),
      (Scalar::Url, "string"),
      (Scalar::Uuid, "string"),
      (Scalar::Bytes, "Uint8ClampedArray"),
      (Scalar::DateTime, "string"),
      (Scalar::SerdeJsonValue, "any"),
      (Scalar::FractionalIndex, "string"),
    ];

    for &(scalar, expected) in cases {
      // Build a transparent type that wraps the scalar.
      // Names are zero-padded to exactly 3 chars ("S00"–"S23").
      let name = format!("S{:02}", scalar as u8);
      let name_bytes: [u8; 3] = name.as_bytes().try_into().unwrap();

      let record: Record<FlatTransparent<3, PrimitiveIdent>> = Record::new(FlatTransparent {
        tag: Tag::Transparent,
        ident: Ident::new(name_bytes),
        atomic: 0,
        inner: PrimitiveIdent::new(scalar),
      });
      let bytes = record.as_bytes();
      let ts = generate_ts(bytes);

      let expected_line = format!("export type {name} = {expected};");
      assert!(
        ts.contains(&expected_line),
        "Scalar::{scalar:?}: expected `{expected_line}`, got:\n{ts}"
      );
    }
  }

  // -----------------------------------------------------------------------
  // Edge cases
  // -----------------------------------------------------------------------

  #[test]
  fn empty_section_produces_no_output() {
    let ts = generate_ts(&[]);
    assert!(ts.is_empty(), "expected empty, got: {ts}");
  }

  #[test]
  fn truncated_record_returns_error() {
    let mut data = Vec::new();
    data.extend_from_slice(&100u32.to_le_bytes());
    data.extend_from_slice(&[0u8; 5]);
    assert!(decode::parse_section(&data).is_err());
  }

  #[test]
  fn skipped_variant_omitted_from_ts() {
    type V1 = FlatVariant<3, 3>;
    type V2 = FlatVariant<6, 6>;
    let record: Record<FlatEnum<4, 0, 0, Types2<V1, V2>>> = Record::new(FlatEnum {
      tag: Tag::Enum,
      ident: Ident::new(*b"Pick"),
      flags: EnumFlags::ALL_UNIT,
      tagging: TaggingKind::External,
      tag_key: Ident::new(*b""),
      content_key: Ident::new(*b""),
      generic_count: U32Le::new(0),
      variant_count: U32Le::new(2),
      variants: Types2(
        FlatVariant {
          ident: Ident::new(*b"Yes"),
          wire_name: Ident::new(*b"yes"),
          flags: VariantFlags::empty(),
          kind: VariantKindTag::Unit,
          child_count: U32Le::new(0),
          fields: Types0(),
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
        FlatVariant {
          ident: Ident::new(*b"Hidden"),
          wire_name: Ident::new(*b"hidden"),
          flags: VariantFlags::SKIP_SER,
          kind: VariantKindTag::Unit,
          child_count: U32Le::new(0),
          fields: Types0(),
          alias_count: U32Le::new(0),
          aliases: Types0(),
        },
      ),
    });
    let bytes = record.as_bytes();
    let ts = generate_ts(bytes);
    assert!(ts.contains("\"yes\""), "got:\n{ts}");
    assert!(!ts.contains("hidden"), "hidden variant should be omitted: {ts}");
  }
}
