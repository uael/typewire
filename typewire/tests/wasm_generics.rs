#![cfg(target_arch = "wasm32")]

use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

fn round_trip<T: Typewire + PartialEq + std::fmt::Debug>(val: &T) {
  let js = val.to_js();
  let back = T::from_js(js).unwrap_throw();
  assert_eq!(*val, back);
}

fn eval(code: &str) -> JsValue {
  js_sys::eval(code).unwrap_throw()
}

// ===========================================================================
// Single type parameter with inline bound
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
struct Wrapper<T: Clone> {
  value: T,
}

#[wasm_bindgen_test]
fn test_single_param_inline_bound() {
  round_trip(&Wrapper { value: 42u32 });
  round_trip(&Wrapper { value: "hello".to_string() });
  round_trip(&Wrapper { value: Some(true) });
}

#[wasm_bindgen_test]
fn test_single_param_patch_js() {
  let old = Wrapper { value: 10u32 };
  let js = old.to_js();
  let mut called = false;
  Wrapper { value: 10u32 }.patch_js(&js, |_| called = true);
  assert!(!called, "same value should not call set");

  Wrapper { value: 20u32 }.patch_js(&js, |_| panic!("should patch in place"));
  let v = js_sys::Reflect::get(&js, &JsValue::from_str("value")).unwrap_throw();
  assert_eq!(v.as_f64().unwrap_throw(), 20.0);
}

// ===========================================================================
// Where clause instead of inline bounds
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
struct WhereClause<T>
where
  T: Clone + std::fmt::Debug,
{
  inner: T,
}

#[wasm_bindgen_test]
fn test_where_clause() {
  round_trip(&WhereClause { inner: 99u32 });
  round_trip(&WhereClause { inner: vec![1u32, 2, 3] });
}

// ===========================================================================
// Multiple type parameters
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
struct Pair<A, B> {
  left: A,
  right: B,
}

#[wasm_bindgen_test]
fn test_multiple_params() {
  round_trip(&Pair { left: 42u32, right: "world".to_string() });
  round_trip(&Pair { left: true, right: vec![1u32, 2] });
}

#[wasm_bindgen_test]
fn test_multiple_params_patch_js() {
  let old = Pair { left: 1u32, right: "a".to_string() };
  let js = old.to_js();
  Pair { left: 1u32, right: "b".to_string() }.patch_js(&js, |_| panic!("should patch in place"));
  let r = js_sys::Reflect::get(&js, &JsValue::from_str("right")).unwrap_throw();
  assert_eq!(r.as_string().unwrap_throw(), "b");
  // left unchanged
  let l = js_sys::Reflect::get(&js, &JsValue::from_str("left")).unwrap_throw();
  assert_eq!(l.as_f64().unwrap_throw(), 1.0);
}

// ===========================================================================
// Multiple params with mixed inline + where bounds
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
struct MixedBounds<A: Clone, B>
where
  B: std::fmt::Debug,
{
  first: A,
  second: B,
}

#[wasm_bindgen_test]
fn test_mixed_bounds() {
  round_trip(&MixedBounds { first: 1u32, second: "two".to_string() });
}

// ===========================================================================
// Nested generics
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
struct Outer<T: Clone + PartialEq> {
  nested: Wrapper<T>,
  extra: u32,
}

#[wasm_bindgen_test]
fn test_nested_generics() {
  round_trip(&Outer { nested: Wrapper { value: 5u32 }, extra: 10 });
}

#[wasm_bindgen_test]
fn test_nested_generics_patch_js() {
  let old = Outer { nested: Wrapper { value: 1u32 }, extra: 10 };
  let js = old.to_js();
  Outer { nested: Wrapper { value: 2u32 }, extra: 10 }
    .patch_js(&js, |_| panic!("should patch in place"));
  let nested = js_sys::Reflect::get(&js, &JsValue::from_str("nested")).unwrap_throw();
  let v = js_sys::Reflect::get(&nested, &JsValue::from_str("value")).unwrap_throw();
  assert_eq!(v.as_f64().unwrap_throw(), 2.0);
}

// ===========================================================================
// Generic enum
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
enum GenericEnum<T> {
  Some(T),
  None,
}

#[wasm_bindgen_test]
fn test_generic_enum() {
  round_trip(&GenericEnum::Some(42u32));
  round_trip(&GenericEnum::<u32>::None);
}

#[wasm_bindgen_test]
fn test_generic_enum_patch_js() {
  let old = GenericEnum::Some(1u32);
  let js = old.to_js();
  let mut called = false;
  GenericEnum::<u32>::None.patch_js(&js, |_| called = true);
  assert!(called, "variant change should call set");
}

// ===========================================================================
// Generic enum — internally tagged
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(tag = "type")]
enum TaggedGeneric<T> {
  Value { data: T },
  Empty,
}

#[wasm_bindgen_test]
fn test_tagged_generic_enum() {
  round_trip(&TaggedGeneric::Value { data: 42u32 });
  round_trip(&TaggedGeneric::<u32>::Empty);
}

#[wasm_bindgen_test]
fn test_tagged_generic_enum_patch_js() {
  let old = TaggedGeneric::Value { data: 1u32 };
  let js = old.to_js();
  TaggedGeneric::Value { data: 2u32 }.patch_js(&js, |_| panic!("same variant should patch"));
  let d = js_sys::Reflect::get(&js, &JsValue::from_str("data")).unwrap_throw();
  assert_eq!(d.as_f64().unwrap_throw(), 2.0);
}

// ===========================================================================
// Generic transparent newtype
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(transparent)]
struct Newtype<T>(T);

#[wasm_bindgen_test]
fn test_generic_transparent() {
  round_trip(&Newtype(42u32));
  round_trip(&Newtype("hello".to_string()));
}

#[wasm_bindgen_test]
fn test_generic_transparent_patch_js() {
  let old = Newtype(10u32);
  let js = old.to_js();
  let mut called = false;
  Newtype(10u32).patch_js(&js, |_| called = true);
  assert!(!called, "same value should not call set");
}

// ===========================================================================
// Generic with default and rename_all
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(rename_all = "camelCase")]
struct GenericCamel<T> {
  field_value: T,
  #[serde(default)]
  extra_info: Option<String>,
}

#[wasm_bindgen_test]
fn test_generic_rename_all() {
  let val = GenericCamel { field_value: 42u32, extra_info: None };
  let js = val.to_js();
  // Should use camelCase
  let fv = js_sys::Reflect::get(&js, &JsValue::from_str("fieldValue")).unwrap_throw();
  assert_eq!(fv.as_f64().unwrap_throw(), 42.0);
  round_trip(&val);
}

#[wasm_bindgen_test]
fn test_generic_rename_all_from_js_missing_default() {
  let js = eval("({ fieldValue: 99 })");
  let val = GenericCamel::<u32>::from_js(js).unwrap_throw();
  assert_eq!(val.field_value, 99);
  assert_eq!(val.extra_info, None);
}

// ===========================================================================
// Generic tuple struct
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
struct GenericTuple<A, B>(A, B);

#[wasm_bindgen_test]
fn test_generic_tuple_struct() {
  round_trip(&GenericTuple(1u32, "two".to_string()));
}

#[wasm_bindgen_test]
fn test_generic_tuple_struct_patch_js() {
  let old = GenericTuple(1u32, "a".to_string());
  let js = old.to_js();
  GenericTuple(1u32, "b".to_string()).patch_js(&js, |_| panic!("should patch in place"));
  let arr: js_sys::Array = js.into();
  assert_eq!(arr.get(1).as_string().unwrap_throw(), "b");
}

// ===========================================================================
// Lifetime parameter (Cow<str>)
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
struct WithLifetime<'a> {
  text: std::borrow::Cow<'a, str>,
}

#[wasm_bindgen_test]
fn test_lifetime_param() {
  round_trip(&WithLifetime { text: std::borrow::Cow::Owned("hello".to_string()) });
}

// ===========================================================================
// Const generic
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
struct FixedArray<const N: usize> {
  items: [u32; N],
}

#[wasm_bindgen_test]
fn test_const_generic() {
  round_trip(&FixedArray { items: [1, 2, 3] });
  round_trip(&FixedArray::<0> { items: [] });
}

// ===========================================================================
// Const generic in enum
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(tag = "type", content = "data")]
enum FixedEnum<const N: usize> {
  Array([u32; N]),
  Single(u32),
}

#[wasm_bindgen_test]
fn test_const_generic_enum() {
  round_trip(&FixedEnum::Array([1, 2, 3]));
  round_trip(&FixedEnum::<1>::Single(42));
  round_trip(&FixedEnum::Array::<0>([]));
}
