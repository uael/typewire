#![cfg(target_arch = "wasm32")]

mod common;
use common::*;
use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

// ===========================================================================
// IndexSet
// ===========================================================================

#[wasm_bindgen_test]
fn test_indexset_round_trip() {
  let mut s = indexmap::IndexSet::new();
  s.insert("a".to_string());
  s.insert("b".to_string());
  s.insert("c".to_string());
  round_trip(&s);

  // Verify it's a JS array
  let js = s.to_js();
  assert!(js_sys::Array::is_array(&js));
  let arr = js_sys::Array::from(&js);
  assert_eq!(arr.length(), 3);
}

#[wasm_bindgen_test]
fn test_indexset_patch_js_unchanged() {
  let mut s = indexmap::IndexSet::new();
  s.insert(1u32);
  s.insert(2u32);
  let js = s.to_js();
  let mut called = false;
  s.patch_js(&js, |_| called = true);
  assert!(!called, "same IndexSet should not call set");
}

#[wasm_bindgen_test]
fn test_indexset_patch_js_element_changed() {
  let mut old = indexmap::IndexSet::new();
  old.insert(1u32);
  old.insert(2u32);
  old.insert(3u32);
  let js = old.to_js();

  let mut new = indexmap::IndexSet::new();
  new.insert(1u32);
  new.insert(2u32);
  new.insert(99u32);
  // Same length — patched in place via patch_js_slice
  new.patch_js(&js, |_| panic!("same-length set should patch in place"));

  let arr = js_sys::Array::from(&js);
  assert_eq!(arr.get(2).as_f64().unwrap_throw(), 99.0);
  // Unchanged elements preserved
  assert_eq!(arr.get(0).as_f64().unwrap_throw(), 1.0);
}

// ===========================================================================
// IndexMap
// ===========================================================================

#[wasm_bindgen_test]
fn test_indexmap_round_trip() {
  let mut m = indexmap::IndexMap::new();
  m.insert("x".to_string(), 10u32);
  m.insert("y".to_string(), 20u32);
  round_trip(&m);

  // Verify it's a JS object
  let js = m.to_js();
  assert!(js.is_object());
  let x_val = js_sys::Reflect::get(&js, &JsValue::from_str("x")).unwrap_throw();
  assert_eq!(x_val.as_f64().unwrap_throw(), 10.0);
}

#[wasm_bindgen_test]
fn test_indexmap_empty() {
  let m: indexmap::IndexMap<String, u32> = indexmap::IndexMap::new();
  round_trip(&m);
}

#[wasm_bindgen_test]
fn test_patch_js_indexmap_unchanged() {
  let mut m = indexmap::IndexMap::new();
  m.insert("a".to_string(), 1u32);
  let js = m.to_js();
  let mut called = false;
  m.patch_js(&js, |_| called = true);
  assert!(!called, "same IndexMap should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_indexmap_value_changed() {
  let mut m = indexmap::IndexMap::new();
  m.insert("a".to_string(), 1u32);
  let js = m.to_js();

  let mut updated = indexmap::IndexMap::new();
  updated.insert("a".to_string(), 99u32);
  updated.patch_js(&js, |_| panic!("should patch in place"));

  let a = js_sys::Reflect::get(&js, &JsValue::from_str("a")).unwrap_throw();
  assert_eq!(a.as_f64().unwrap_throw(), 99.0);
}

#[wasm_bindgen_test]
fn test_patch_js_indexmap_key_added() {
  let mut m = indexmap::IndexMap::new();
  m.insert("a".to_string(), 1u32);
  let js = m.to_js();

  let mut updated = indexmap::IndexMap::new();
  updated.insert("a".to_string(), 1u32);
  updated.insert("b".to_string(), 2u32);
  updated.patch_js(&js, |_| panic!("should patch in place"));

  let b = js_sys::Reflect::get(&js, &JsValue::from_str("b")).unwrap_throw();
  assert_eq!(b.as_f64().unwrap_throw(), 2.0);
}

#[wasm_bindgen_test]
fn test_patch_js_indexmap_key_removed() {
  let mut m = indexmap::IndexMap::new();
  m.insert("a".to_string(), 1u32);
  m.insert("b".to_string(), 2u32);
  let js = m.to_js();

  let mut updated = indexmap::IndexMap::new();
  updated.insert("a".to_string(), 1u32);
  updated.patch_js(&js, |_| panic!("should patch in place"));

  let b = js_sys::Reflect::get(&js, &JsValue::from_str("b")).unwrap_throw();
  assert!(b.is_undefined(), "removed key should be undefined");
}

// ===========================================================================
// patch_js: IndexMap on array — must replace
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_indexmap_on_array_replaces() {
  let old_arr = vec![1u32, 2, 3].to_js();
  let mut map = indexmap::IndexMap::new();
  map.insert("a".to_string(), 1u32);
  let mut replaced = None;
  map.patch_js(&old_arr, |v| replaced = Some(v));
  assert!(replaced.is_some(), "IndexMap on array old should replace");
}
