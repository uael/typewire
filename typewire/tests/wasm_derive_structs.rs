#![cfg(target_arch = "wasm32")]

mod common;
use common::*;
use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_basic_struct() {
  let s = BasicStruct { name: "Alice".into(), age: 30 };
  round_trip(&s);

  // Verify JS shape
  let js = s.to_js();
  let name = js_sys::Reflect::get(&js, &JsValue::from_str("name")).unwrap_throw();
  assert_eq!(name.as_string().unwrap_throw(), "Alice");
  let age = js_sys::Reflect::get(&js, &JsValue::from_str("age")).unwrap_throw();
  assert_eq!(age.as_f64().unwrap_throw(), 30.0);
}

#[wasm_bindgen_test]
fn test_rename_all_camel_case() {
  let s = CamelCaseStruct { first_name: "Bob".into(), last_name: "Smith".into(), is_active: true };
  round_trip(&s);

  let js = s.to_js();
  assert!(js_sys::Reflect::get(&js, &JsValue::from_str("firstName")).unwrap_throw().is_string());
  assert!(js_sys::Reflect::get(&js, &JsValue::from_str("lastName")).unwrap_throw().is_string());
  assert!(
    js_sys::Reflect::get(&js, &JsValue::from_str("isActive")).unwrap_throw().as_bool().is_some()
  );
}

#[wasm_bindgen_test]
fn test_rename_field() {
  let s = RenameFieldStruct { kind: "test".into(), value: 1 };
  round_trip(&s);

  let js = s.to_js();
  let ty = js_sys::Reflect::get(&js, &JsValue::from_str("type")).unwrap_throw();
  assert_eq!(ty.as_string().unwrap_throw(), "test");
}

#[wasm_bindgen_test]
fn test_skip_field() {
  let s = SkipFieldStruct { visible: "hello".into(), hidden: 99 };
  let js = s.to_js();

  // hidden should NOT be in the JS object
  let hidden = js_sys::Reflect::get(&js, &JsValue::from_str("hidden")).unwrap_throw();
  assert!(hidden.is_undefined());

  // Round-trip: hidden gets Default::default()
  let back = SkipFieldStruct::from_js(js).unwrap_throw();
  assert_eq!(back.visible, "hello");
  assert_eq!(back.hidden, 0); // default for u32
}

#[wasm_bindgen_test]
fn test_skip_serializing() {
  let s = SkipSerStruct { name: "test".into(), write_only: 42 };
  let js = s.to_js();
  // skip_serializing: not in JS output
  assert!(
    js_sys::Reflect::get(&js, &JsValue::from_str("write_only")).unwrap_throw().is_undefined()
  );
}

#[wasm_bindgen_test]
fn test_skip_deserializing() {
  let s = SkipDeStruct { name: "test".into(), read_only: 42 };
  let js = s.to_js();
  // skip_deserializing: IS in JS output for serialization
  let val = js_sys::Reflect::get(&js, &JsValue::from_str("read_only")).unwrap_throw();
  assert_eq!(val.as_f64().unwrap_throw(), 42.0);

  // But when deserializing, gets default
  let back = SkipDeStruct::from_js(js).unwrap_throw();
  assert_eq!(back.read_only, 0);
}

#[wasm_bindgen_test]
fn test_default_field() {
  // from_js with missing field → uses default
  let js = eval("({ name: 'hello' })");
  let s = DefaultFieldStruct::from_js(js).unwrap_throw();
  assert_eq!(s.name, "hello");
  assert_eq!(s.count, 0);

  // from_js with present field → uses value
  let js = eval("({ name: 'hi', count: 5 })");
  let s = DefaultFieldStruct::from_js(js).unwrap_throw();
  assert_eq!(s.count, 5);
}

#[wasm_bindgen_test]
fn test_default_path() {
  let js = eval("({ name: 'hello' })");
  let s = DefaultPathStruct::from_js(js).unwrap_throw();
  assert_eq!(s.count, 99);
}

#[wasm_bindgen_test]
fn test_container_default() {
  let js = eval("({})");
  let s = ContainerDefaultStruct::from_js(js).unwrap_throw();
  assert_eq!(s.name, ""); // Default::default() for String, not ContainerDefaultStruct::default()
  assert_eq!(s.count, 0); // Default::default() for u32
}

#[wasm_bindgen_test]
fn test_flatten() {
  let s = FlattenStruct { name: "test".into(), inner: Inner { x: 1, y: 2 } };
  let js = s.to_js();

  // Flattened: x and y should be at the top level
  let x = js_sys::Reflect::get(&js, &JsValue::from_str("x")).unwrap_throw();
  assert_eq!(x.as_f64().unwrap_throw(), 1.0);
  let y = js_sys::Reflect::get(&js, &JsValue::from_str("y")).unwrap_throw();
  assert_eq!(y.as_f64().unwrap_throw(), 2.0);

  round_trip(&s);
}

#[wasm_bindgen_test]
fn test_skip_serializing_if() {
  let zero = SkipSerIfStruct { name: "test".into(), count: 0 };
  let js = zero.to_js();
  assert!(js_sys::Reflect::get(&js, &JsValue::from_str("count")).unwrap_throw().is_undefined());

  let nonzero = SkipSerIfStruct { name: "test".into(), count: 5 };
  let js = nonzero.to_js();
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("count")).unwrap_throw().as_f64().unwrap_throw(),
    5.0
  );
}

#[wasm_bindgen_test]
fn test_alias() {
  // Primary name
  let js = eval("({ name: 'Alice' })");
  let s = AliasStruct::from_js(js).unwrap_throw();
  assert_eq!(s.name, "Alice");

  // First alias
  let js = eval("({ userName: 'Bob' })");
  let s = AliasStruct::from_js(js).unwrap_throw();
  assert_eq!(s.name, "Bob");

  // Second alias
  let js = eval("({ user: 'Charlie' })");
  let s = AliasStruct::from_js(js).unwrap_throw();
  assert_eq!(s.name, "Charlie");
}

#[wasm_bindgen_test]
fn test_deny_unknown_fields() {
  // Valid
  let js = eval("({ name: 'Alice' })");
  let s = StrictStruct::from_js(js).unwrap_throw();
  assert_eq!(s.name, "Alice");

  // Unknown field → error
  let js = eval("({ name: 'Alice', extra: 42 })");
  assert!(StrictStruct::from_js(js).is_err());
}

#[wasm_bindgen_test]
fn test_transparent_named() {
  let s = TransparentNamed { inner: 42 };
  let js = s.to_js();
  assert_eq!(js.as_f64().unwrap_throw(), 42.0);
  round_trip(&s);
}

#[wasm_bindgen_test]
fn test_transparent_tuple() {
  let s = TransparentTuple("hello".into());
  let js = s.to_js();
  assert_eq!(js.as_string().unwrap_throw(), "hello");
  round_trip(&s);
}

#[wasm_bindgen_test]
fn test_tuple_struct() {
  let s = TupleStruct(42, "hello".into());
  round_trip(&s);

  let js = s.to_js();
  let arr: js_sys::Array = js.into();
  assert_eq!(arr.length(), 2);
}

#[wasm_bindgen_test]
fn test_unit_struct() {
  let s = UnitStruct;
  assert!(s.to_js().is_null());
  round_trip(&s);
}

#[wasm_bindgen_test]
fn test_generic_struct() {
  let s = GenericStruct { value: 42u32 };
  round_trip(&s);
  let s = GenericStruct { value: "hello".to_string() };
  round_trip(&s);
}

#[wasm_bindgen_test]
fn test_option_fields_default_to_none() {
  // Only the required field is present — optional fields default to None.
  let js = eval("({ required: 'hello' })");
  let val = OptionalFields::from_js(js).unwrap_throw();
  assert_eq!(val.required, "hello");
  assert_eq!(val.optional, None);
  assert_eq!(val.also_optional, None);

  // All fields present — optional fields are populated.
  let js = eval("({ required: 'hello', optional: 42, also_optional: 'world' })");
  let val = OptionalFields::from_js(js).unwrap_throw();
  assert_eq!(val.optional, Some(42));
  assert_eq!(val.also_optional, Some("world".into()));

  // Missing required field still errors.
  let js = eval("({ optional: 42 })");
  let err = OptionalFields::from_js(js).unwrap_err();
  let msg = err.to_string();
  assert!(msg.contains("required"), "should report missing required field: {msg}");
}

// ===========================================================================
// Derive: proxy types (from / try_from / into)
// ===========================================================================

#[wasm_bindgen_test]
fn test_from_into_proxy() {
  let val = FromIntoProxy { value: 42 };
  let js = val.to_js();
  let arr: js_sys::Array = js.clone().into();
  assert_eq!(arr.get(0).as_f64().unwrap_throw(), 42.0);

  let back = FromIntoProxy::from_js(js).unwrap_throw();
  assert_eq!(back.value, 42);
}

#[wasm_bindgen_test]
fn test_try_from_into_proxy() {
  let val = ValidatedString("hello".into());
  let js = val.to_js();
  assert_eq!(js.as_string().unwrap_throw(), "hello");
  let back = ValidatedString::from_js(js).unwrap_throw();
  assert_eq!(back, ValidatedString("hello".into()));

  // Error path: empty string is rejected by TryFrom.
  let js = JsValue::from_str("");
  let err = ValidatedString::from_js(js).unwrap_err();
  let msg = err.to_string();
  assert!(msg.contains("empty"), "TryFrom error should propagate: {msg}");
}

#[wasm_bindgen_test]
fn test_try_from_proxy() {
  let val = BoundedU32 { value: 42 };
  let js = val.to_js();
  let obj: js_sys::Object = js.clone().into();
  assert_eq!(
    js_sys::Reflect::get(&obj, &JsValue::from_str("value")).unwrap_throw().as_f64().unwrap_throw(),
    42.0
  );

  // from_js: uses TryFrom<u32> proxy — accepts a plain number, not an object.
  let js = JsValue::from_f64(50.0);
  let back = BoundedU32::from_js(js).unwrap_throw();
  assert_eq!(back.value, 50);

  // Error path: value > 100 is rejected by TryFrom.
  let js = JsValue::from_f64(200.0);
  let err = BoundedU32::from_js(js).unwrap_err();
  let msg = err.to_string();
  assert!(msg.contains("100"), "TryFrom error should propagate: {msg}");
}
