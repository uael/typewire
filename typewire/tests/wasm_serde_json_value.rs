#![cfg(target_arch = "wasm32")]

mod common;
use common::*;
use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

// ===========================================================================
// serde_json::Value round-trips
// ===========================================================================

#[wasm_bindgen_test]
fn test_serde_json_null() {
  let v = serde_json::Value::Null;
  round_trip(&v);
  assert!(v.to_js().is_null());
}

#[wasm_bindgen_test]
fn test_serde_json_bool() {
  round_trip(&serde_json::Value::Bool(true));
  round_trip(&serde_json::Value::Bool(false));
}

#[wasm_bindgen_test]
fn test_serde_json_number() {
  round_trip(&serde_json::json!(42));
  round_trip(&serde_json::json!(3.14));
  round_trip(&serde_json::json!(-100));
  round_trip(&serde_json::json!(0));
}

#[wasm_bindgen_test]
fn test_serde_json_string() {
  round_trip(&serde_json::json!("hello world"));
  round_trip(&serde_json::json!(""));
}

#[wasm_bindgen_test]
fn test_serde_json_array() {
  round_trip(&serde_json::json!([1, "two", true, null]));
  round_trip(&serde_json::json!([]));
}

#[wasm_bindgen_test]
fn test_serde_json_object() {
  round_trip(&serde_json::json!({"a": 1, "b": "two", "c": true}));
  round_trip(&serde_json::json!({}));
}

#[wasm_bindgen_test]
fn test_serde_json_nested() {
  let v = serde_json::json!({
      "users": [
          {"name": "Alice", "age": 30},
          {"name": "Bob", "age": 25}
      ],
      "count": 2,
      "active": true
  });
  round_trip(&v);
}

#[wasm_bindgen_test]
fn test_serde_json_from_js_nan() {
  let nan = eval("NaN");
  let result = serde_json::Value::from_js(nan);
  assert!(result.is_err(), "NaN is not a valid JSON number");
}

#[wasm_bindgen_test]
fn test_serde_json_from_js_undefined() {
  let v = serde_json::Value::from_js(JsValue::UNDEFINED).unwrap_throw();
  assert_eq!(v, serde_json::Value::Null);
}

#[wasm_bindgen_test]
fn test_serde_json_to_js_structure() {
  let v = serde_json::json!({"x": [1, 2]});
  let js = v.to_js();
  // Verify it's a real JS object, not a string
  assert!(js.is_object());
  let x = js_sys::Reflect::get(&js, &JsValue::from_str("x")).unwrap_throw();
  assert!(js_sys::Array::is_array(&x));
  let arr = js_sys::Array::from(&x);
  assert_eq!(arr.length(), 2);
  assert_eq!(arr.get(0).as_f64().unwrap_throw(), 1.0);
}

// ---------------------------------------------------------------------------
// serde_json::Value patch_js
// ---------------------------------------------------------------------------

#[wasm_bindgen_test]
fn test_patch_js_serde_json_null_unchanged() {
  let v = serde_json::Value::Null;
  let mut called = false;
  v.patch_js(&JsValue::NULL, |_| called = true);
  assert!(!called);
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_null_from_undefined() {
  let v = serde_json::Value::Null;
  let mut called = false;
  v.patch_js(&JsValue::UNDEFINED, |_| called = true);
  assert!(!called, "undefined is nullish — should not replace");
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_bool_unchanged() {
  let v = serde_json::Value::Bool(true);
  let old = JsValue::from_bool(true);
  let mut called = false;
  v.patch_js(&old, |_| called = true);
  assert!(!called);
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_bool_changed() {
  let v = serde_json::Value::Bool(false);
  let old = JsValue::from_bool(true);
  let mut new_val = None;
  v.patch_js(&old, |v| new_val = Some(v));
  assert_eq!(new_val.unwrap_throw().as_bool().unwrap_throw(), false);
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_number_unchanged() {
  let v = serde_json::json!(42);
  let old = JsValue::from_f64(42.0);
  let mut called = false;
  v.patch_js(&old, |_| called = true);
  assert!(!called);
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_number_changed() {
  let v = serde_json::json!(99);
  let old = JsValue::from_f64(42.0);
  let mut new_val = None;
  v.patch_js(&old, |v| new_val = Some(v));
  assert_eq!(new_val.unwrap_throw().as_f64().unwrap_throw(), 99.0);
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_string_unchanged() {
  let v = serde_json::json!("hello");
  let old = JsValue::from_str("hello");
  let mut called = false;
  v.patch_js(&old, |_| called = true);
  assert!(!called);
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_string_changed() {
  let v = serde_json::json!("world");
  let old = JsValue::from_str("hello");
  let mut new_val = None;
  v.patch_js(&old, |v| new_val = Some(v));
  assert_eq!(new_val.unwrap_throw().as_string().unwrap_throw(), "world");
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_object_field_changed() {
  let old_val = serde_json::json!({"a": 1, "b": 2});
  let js = old_val.to_js();

  let new_val = serde_json::json!({"a": 1, "b": 99});
  new_val.patch_js(&js, |_| panic!("object should be patched in place"));

  // Object identity preserved, field updated
  let b = js_sys::Reflect::get(&js, &JsValue::from_str("b")).unwrap_throw();
  assert_eq!(b.as_f64().unwrap_throw(), 99.0);
  // Unchanged field still there
  let a = js_sys::Reflect::get(&js, &JsValue::from_str("a")).unwrap_throw();
  assert_eq!(a.as_f64().unwrap_throw(), 1.0);
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_object_key_added() {
  let old_val = serde_json::json!({"a": 1});
  let js = old_val.to_js();

  let new_val = serde_json::json!({"a": 1, "b": 2});
  new_val.patch_js(&js, |_| panic!("should patch in place"));

  let b = js_sys::Reflect::get(&js, &JsValue::from_str("b")).unwrap_throw();
  assert_eq!(b.as_f64().unwrap_throw(), 2.0);
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_object_key_removed() {
  let old_val = serde_json::json!({"a": 1, "b": 2});
  let js = old_val.to_js();

  let new_val = serde_json::json!({"a": 1});
  new_val.patch_js(&js, |_| panic!("should patch in place"));

  let b = js_sys::Reflect::get(&js, &JsValue::from_str("b")).unwrap_throw();
  assert!(b.is_undefined(), "removed key should be undefined");
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_array_element_changed() {
  let old_val = serde_json::json!([1, 2, 3]);
  let js = old_val.to_js();

  let new_val = serde_json::json!([1, 2, 99]);
  new_val.patch_js(&js, |_| panic!("same-length array should patch in place"));

  let arr = js_sys::Array::from(&js);
  assert_eq!(arr.get(2).as_f64().unwrap_throw(), 99.0);
  // Unchanged elements preserved
  assert_eq!(arr.get(0).as_f64().unwrap_throw(), 1.0);
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_nested_object_identity() {
  let old_val = serde_json::json!({"user": {"name": "Alice", "age": 30}});
  let js = old_val.to_js();
  let inner_before = js_sys::Reflect::get(&js, &JsValue::from_str("user")).unwrap_throw();

  // Only change a leaf inside the nested object
  let new_val = serde_json::json!({"user": {"name": "Alice", "age": 31}});
  new_val.patch_js(&js, |_| panic!("should patch in place"));

  // The inner object reference is preserved (patched in place)
  let inner_after = js_sys::Reflect::get(&js, &JsValue::from_str("user")).unwrap_throw();
  assert!(
    js_sys::Object::is(&inner_before, &inner_after),
    "nested object identity should be preserved"
  );
  // But the leaf was updated
  let age = js_sys::Reflect::get(&inner_after, &JsValue::from_str("age")).unwrap_throw();
  assert_eq!(age.as_f64().unwrap_throw(), 31.0);
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_variant_change_replaces() {
  // String → Object: different JS shape, must replace
  let old_val = serde_json::json!("hello");
  let js = old_val.to_js();

  let new_val = serde_json::json!({"key": "value"});
  let mut new_js = None;
  new_val.patch_js(&js, |v| new_js = Some(v));
  assert!(new_js.is_some(), "different variant should call set");

  // Object → Array: must replace
  let obj_js = new_val.to_js();
  let array_val = serde_json::json!([1, 2]);
  let mut replaced = None;
  array_val.patch_js(&obj_js, |v| replaced = Some(v));
  assert!(replaced.is_some(), "object→array should call set");
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_object_unchanged() {
  let v = serde_json::json!({"x": 1, "y": 2});
  let js = v.to_js();
  let mut called = false;
  v.patch_js(&js, |_| called = true);
  assert!(!called, "identical object should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_array_on_non_array() {
  // If old is not an array, patch_js_slice replaces
  let old = JsValue::from_str("not an array");
  let new_val = serde_json::json!([1, 2]);
  let mut replaced = None;
  new_val.patch_js(&old, |v| replaced = Some(v));
  assert!(replaced.is_some());
}

#[wasm_bindgen_test]
fn test_patch_js_serde_json_object_on_array() {
  // Old is an array but new is an object — must replace, not patch in place
  let old = serde_json::json!([1, 2]).to_js();
  let new_val = serde_json::json!({"a": 1});
  let mut replaced = None;
  new_val.patch_js(&old, |v| replaced = Some(v));
  assert!(replaced.is_some(), "object on array old should replace");
}
