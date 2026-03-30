#![cfg(target_arch = "wasm32")]

mod common;
use common::*;
use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_unit_enum() {
  round_trip(&UnitEnum::A);
  round_trip(&UnitEnum::B);
  round_trip(&UnitEnum::C);

  // Serialized as string
  assert_eq!(UnitEnum::A.to_js().as_string().unwrap_throw(), "A");

  // Unknown variant → error
  assert!(UnitEnum::from_js(JsValue::from_str("Unknown")).is_err());
}

#[wasm_bindgen_test]
fn test_enum_rename_all() {
  assert_eq!(RenameAllEnum::FirstVariant.to_js().as_string().unwrap_throw(), "first_variant");
  let back = RenameAllEnum::from_js(JsValue::from_str("second_variant")).unwrap_throw();
  assert_eq!(back, RenameAllEnum::SecondVariant);
}

#[wasm_bindgen_test]
fn test_external_mixed() {
  // Unit → string
  let js = MixedExternalEnum::Unit.to_js();
  assert_eq!(js.as_string().unwrap_throw(), "Unit");
  round_trip(&MixedExternalEnum::Unit);

  // Newtype → { "Newtype": value }
  let js = MixedExternalEnum::Newtype(42).to_js();
  let inner = js_sys::Reflect::get(&js, &JsValue::from_str("Newtype")).unwrap_throw();
  assert_eq!(inner.as_f64().unwrap_throw(), 42.0);
  round_trip(&MixedExternalEnum::Newtype(42));

  // Tuple → { "Tuple": [values] }
  let js = MixedExternalEnum::Tuple(1, "hi".into()).to_js();
  let inner = js_sys::Reflect::get(&js, &JsValue::from_str("Tuple")).unwrap_throw();
  let arr: js_sys::Array = inner.into();
  assert_eq!(arr.length(), 2);
  round_trip(&MixedExternalEnum::Tuple(1, "hi".into()));

  // Struct → { "Struct": { x, y } }
  round_trip(&MixedExternalEnum::Struct { x: 10, y: 20 });
}

#[wasm_bindgen_test]
fn test_internally_tagged() {
  // Unit → { "type": "Unit" }
  let js = InternallyTagged::Unit.to_js();
  let tag = js_sys::Reflect::get(&js, &JsValue::from_str("type")).unwrap_throw();
  assert_eq!(tag.as_string().unwrap_throw(), "Unit");
  round_trip(&InternallyTagged::Unit);

  // Named → { "type": "Named", "fieldOne": ..., "fieldTwo": ... }
  let val = InternallyTagged::Named { field_one: 1, field_two: "hi".into() };
  let js = val.to_js();
  let tag = js_sys::Reflect::get(&js, &JsValue::from_str("type")).unwrap_throw();
  assert_eq!(tag.as_string().unwrap_throw(), "Named");
  let f1 = js_sys::Reflect::get(&js, &JsValue::from_str("fieldOne")).unwrap_throw();
  assert_eq!(f1.as_f64().unwrap_throw(), 1.0);
  round_trip(&val);

  // Newtype → merges tag into inner object
  let val = InternallyTagged::Newtype(Inner { x: 3, y: 4 });
  round_trip(&val);
}

#[wasm_bindgen_test]
fn test_adjacently_tagged() {
  // Unit → { "t": "Unit" } (no "c")
  let js = AdjacentlyTagged::Unit.to_js();
  let tag = js_sys::Reflect::get(&js, &JsValue::from_str("t")).unwrap_throw();
  assert_eq!(tag.as_string().unwrap_throw(), "Unit");
  assert!(js_sys::Reflect::get(&js, &JsValue::from_str("c")).unwrap_throw().is_undefined());
  round_trip(&AdjacentlyTagged::Unit);

  // Single → { "t": "Single", "c": 42 }
  let js = AdjacentlyTagged::Single(42).to_js();
  let c = js_sys::Reflect::get(&js, &JsValue::from_str("c")).unwrap_throw();
  assert_eq!(c.as_f64().unwrap_throw(), 42.0);
  round_trip(&AdjacentlyTagged::Single(42));

  // Multi → { "t": "Multi", "c": [1, "hi"] }
  round_trip(&AdjacentlyTagged::Multi(1, "hi".into()));

  // Named → { "t": "Named", "c": { a: 1, b: "hi" } }
  round_trip(&AdjacentlyTagged::Named { a: 1, b: "hi".into() });
}

#[wasm_bindgen_test]
fn test_untagged() {
  // Number
  round_trip(&Untagged::Num(42));
  assert_eq!(Untagged::Num(42).to_js().as_f64().unwrap_throw(), 42.0);

  // String
  round_trip(&Untagged::Text("hello".into()));

  // Struct
  round_trip(&Untagged::Pair { x: 1, y: 2 });

  // No match
  assert!(Untagged::from_js(JsValue::TRUE).is_err());
}

#[wasm_bindgen_test]
fn test_variant_rename() {
  assert_eq!(VariantRename::Original.to_js().as_string().unwrap_throw(), "custom_name");
  let back = VariantRename::from_js(JsValue::from_str("custom_name")).unwrap_throw();
  assert_eq!(back, VariantRename::Original);
}

#[wasm_bindgen_test]
fn test_variant_alias() {
  // Primary
  round_trip(&VariantAlias::Current);

  // Alias
  let v = VariantAlias::from_js(JsValue::from_str("legacy")).unwrap_throw();
  assert_eq!(v, VariantAlias::Current);
  let v = VariantAlias::from_js(JsValue::from_str("old")).unwrap_throw();
  assert_eq!(v, VariantAlias::Current);
}

#[wasm_bindgen_test]
fn test_variant_skip() {
  round_trip(&VariantSkip::Visible);
  // Hidden variant cannot be deserialized
  assert!(VariantSkip::from_js(JsValue::from_str("Hidden")).is_err());
}

#[wasm_bindgen_test]
fn test_variant_other() {
  round_trip(&VariantOther::Known);
  // Unknown strings fall through to Other
  let v = VariantOther::from_js(JsValue::from_str("anything")).unwrap_throw();
  assert_eq!(v, VariantOther::Unknown);
}

#[wasm_bindgen_test]
fn test_per_variant_untagged() {
  // Tagged variant works normally
  let val = PerVariantUntagged::Tagged { value: 42 };
  round_trip(&val);

  // Untagged variant: try content matching when tag doesn't match
  let js = JsValue::from_str("hello");
  let v = PerVariantUntagged::from_js(js).unwrap_throw();
  assert_eq!(v, PerVariantUntagged::Fallback("hello".into()));
}

#[wasm_bindgen_test]
fn test_rename_all_fields() {
  let val = RenameAllFields::A { field_name: 42 };
  let js = val.to_js();
  let inner = js_sys::Reflect::get(&js, &JsValue::from_str("A")).unwrap_throw();
  let f = js_sys::Reflect::get(&inner, &JsValue::from_str("fieldName")).unwrap_throw();
  assert_eq!(f.as_f64().unwrap_throw(), 42.0);
  round_trip(&val);
  round_trip(&RenameAllFields::B { other_field: "hi".into() });
}

#[wasm_bindgen_test]
fn test_externally_tagged_with_extra_keys() {
  // When an externally tagged enum is nested inside an internally tagged one,
  // the JS object has the parent's tag field ("type") alongside the variant key ("ok").
  // The externally tagged from_js must skip unknown keys (like "type").
  let js = eval("({ type: 'inner', ok: {} })");
  let outer = OuterTagged::from_js(js).unwrap_throw();
  assert_eq!(outer, OuterTagged::Inner(InnerExternalEnum::Ok {}));
}

#[wasm_bindgen_test]
fn test_externally_tagged_with_extra_keys_error_variant() {
  let js = eval("({ type: 'inner', error: { message: 'fail' } })");
  let outer = OuterTagged::from_js(js).unwrap_throw();
  assert_eq!(outer, OuterTagged::Inner(InnerExternalEnum::Error { message: "fail".into() }));
}

#[wasm_bindgen_test]
fn test_externally_tagged_no_matching_key() {
  // No known variant key → error
  let js = eval("({ unknown: 123 })");
  let result = InnerExternalEnum::from_js(js);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_untagged_no_matching_variant_error_message() {
  let err = Untagged::from_js(JsValue::TRUE).unwrap_err();
  let msg = err.to_string();
  assert!(
    msg.contains("no matching variant"),
    "untagged enum should report 'no matching variant': {msg}"
  );
}

// ===========================================================================
// Untagged enum: ambiguous variants (first-match wins)
// ===========================================================================

#[wasm_bindgen_test]
fn test_untagged_ambiguous_first_match_wins() {
  // Both Count(u32) and Amount(u32) accept a number — first one (Count) wins
  let val = AmbiguousUntagged::from_js(JsValue::from_f64(42.0)).unwrap_throw();
  assert_eq!(val, AmbiguousUntagged::Count(42));
}

#[wasm_bindgen_test]
fn test_untagged_ambiguous_fallback() {
  // A string doesn't match u32 variants, falls through to Label
  let val = AmbiguousUntagged::from_js(JsValue::from_str("hello")).unwrap_throw();
  assert_eq!(val, AmbiguousUntagged::Label("hello".into()));
}

#[wasm_bindgen_test]
fn test_untagged_ambiguous_no_match() {
  // boolean matches neither u32 nor String
  let err = AmbiguousUntagged::from_js(JsValue::TRUE).unwrap_err();
  let msg = err.to_string();
  assert!(msg.contains("no matching variant"), "should report no matching variant: {msg}");
}
