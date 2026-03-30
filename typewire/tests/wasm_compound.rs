#![cfg(target_arch = "wasm32")]

mod common;
use common::*;
use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_option() {
  let some: Option<u32> = Some(42);
  let none: Option<u32> = None;
  round_trip(&some);
  round_trip(&none);

  // None → null
  assert!(none.to_js().is_null());

  // null → None
  let from_null = Option::<u32>::from_js(JsValue::NULL).unwrap_throw();
  assert_eq!(from_null, None);

  // undefined → None
  let from_undef = Option::<u32>::from_js(JsValue::UNDEFINED).unwrap_throw();
  assert_eq!(from_undef, None);
}

#[wasm_bindgen_test]
fn test_vec() {
  let v: Vec<u32> = vec![1, 2, 3];
  round_trip(&v);
  let empty: Vec<String> = vec![];
  round_trip(&empty);
  assert!(Vec::<u32>::from_js(JsValue::from_str("nope")).is_err());
}

#[wasm_bindgen_test]
fn test_box() {
  let b: Box<u32> = Box::new(42);
  round_trip(&b);
}

#[wasm_bindgen_test]
fn test_array() {
  let a: [u32; 3] = [1, 2, 3];
  round_trip(&a);

  // Wrong length → error
  let js_arr = eval("[1, 2]");
  assert!(<[u32; 3]>::from_js(js_arr).is_err());
}

#[wasm_bindgen_test]
fn test_hashmap() {
  let mut m = std::collections::HashMap::new();
  m.insert("key".to_string(), 42u32);
  m.insert("other".to_string(), 7u32);
  round_trip(&m);

  // Verify it's a plain object
  let js = m.to_js();
  assert!(js.is_object());
  let key_val = js_sys::Reflect::get(&js, &JsValue::from_str("key")).unwrap_throw();
  assert_eq!(key_val.as_f64().unwrap_throw(), 42.0);
}

#[wasm_bindgen_test]
fn test_btreemap() {
  let mut m = std::collections::BTreeMap::new();
  m.insert("a".to_string(), true);
  m.insert("b".to_string(), false);
  round_trip(&m);
}

#[wasm_bindgen_test]
fn test_tuples() {
  round_trip(&(42u32,));
  round_trip(&(1u32, "hello".to_string()));
  round_trip(&(true, 42u32, "world".to_string()));
}

#[wasm_bindgen_test]
fn test_jsvalue_passthrough() {
  let original = JsValue::from_f64(42.0);
  let js = original.to_js();
  assert_eq!(js.as_f64().unwrap_throw(), 42.0);
  let back = JsValue::from_js(js).unwrap_throw();
  assert_eq!(back.as_f64().unwrap_throw(), 42.0);
}

#[wasm_bindgen_test]
fn test_tuples_three_elements() {
  round_trip(&(true, 42u32, "world".to_string()));
  // Verify JS shape is array
  let val = (1u32, 2u32, 3u32);
  let js = val.to_js();
  let arr = js_sys::Array::from(&js);
  assert_eq!(arr.length(), 3);
  assert_eq!(arr.get(0).as_f64().unwrap_throw(), 1.0);
  assert_eq!(arr.get(2).as_f64().unwrap_throw(), 3.0);
}

#[wasm_bindgen_test]
fn test_tuples_four_elements() {
  round_trip(&(1u32, "a".to_string(), true, 3.14f64));
}

#[wasm_bindgen_test]
fn test_array_from_js_wrong_length() {
  // Wrong length (too short) — error
  assert!(<[u32; 3]>::from_js(eval("[1]")).is_err());
  // Wrong length (too long) — error
  assert!(<[u32; 3]>::from_js(eval("[1, 2, 3, 4]")).is_err());
}

#[wasm_bindgen_test]
fn test_array_from_js_invalid_element() {
  // Valid length but invalid element type
  assert!(<[u32; 3]>::from_js(eval("[1, 'bad', 3]")).is_err());
}
