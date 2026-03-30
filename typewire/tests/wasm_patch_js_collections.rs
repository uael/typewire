#![cfg(target_arch = "wasm32")]

mod common;
use common::*;
use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

// ===========================================================================
// patch_js: LCS Vec
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_lcs_element_changed() {
  let old = LcsVecStruct { items: vec![1, 2, 3] };
  let js = old.to_js();

  LcsVecStruct { items: vec![1, 2, 4] }.patch_js(&js, |_| panic!("should patch in place"));

  let arr = get_items_arr(&js);
  assert_eq!(arr.length(), 3);
  assert_eq!(arr.get(0).as_f64().unwrap_throw(), 1.0);
  assert_eq!(arr.get(1).as_f64().unwrap_throw(), 2.0);
  assert_eq!(arr.get(2).as_f64().unwrap_throw(), 4.0);
}

#[wasm_bindgen_test]
fn test_patch_js_lcs_unchanged() {
  let old = LcsVecStruct { items: vec![1, 2, 3] };
  let js = old.to_js();
  let arr_before = get_items_arr(&js);

  LcsVecStruct { items: vec![1, 2, 3] }.patch_js(&js, |_| panic!("should not replace"));

  // Array identity preserved
  let arr_after = get_items_arr(&js);
  assert!(arr_before == arr_after, "unchanged array should keep identity");
}

#[wasm_bindgen_test]
fn test_patch_js_lcs_insert() {
  let old = LcsVecStruct { items: vec![1, 2, 3] };
  let js = old.to_js();

  LcsVecStruct { items: vec![1, 2, 99, 3] }.patch_js(&js, |_| panic!("should patch in place"));

  let arr = get_items_arr(&js);
  assert_eq!(arr.length(), 4);
  assert_eq!(arr.get(2).as_f64().unwrap_throw(), 99.0);
  assert_eq!(arr.get(3).as_f64().unwrap_throw(), 3.0);
}

#[wasm_bindgen_test]
fn test_patch_js_lcs_remove() {
  let old = LcsVecStruct { items: vec![1, 2, 3, 4, 5] };
  let js = old.to_js();

  LcsVecStruct { items: vec![1, 2, 5] }.patch_js(&js, |_| panic!("should patch in place"));

  let arr = get_items_arr(&js);
  assert_eq!(arr.length(), 3);
  assert_eq!(arr.get(0).as_f64().unwrap_throw(), 1.0);
  assert_eq!(arr.get(1).as_f64().unwrap_throw(), 2.0);
  assert_eq!(arr.get(2).as_f64().unwrap_throw(), 5.0);
}

#[wasm_bindgen_test]
fn test_patch_js_lcs_append() {
  let old = LcsVecStruct { items: vec![1, 2, 3] };
  let js = old.to_js();

  LcsVecStruct { items: vec![1, 2, 3, 4, 5] }.patch_js(&js, |_| panic!("should patch in place"));

  let arr = get_items_arr(&js);
  assert_eq!(arr.length(), 5);
  assert_eq!(arr.get(3).as_f64().unwrap_throw(), 4.0);
  assert_eq!(arr.get(4).as_f64().unwrap_throw(), 5.0);
}

#[wasm_bindgen_test]
fn test_patch_js_lcs_replace_all() {
  let old = LcsVecStruct { items: vec![1, 2, 3] };
  let js = old.to_js();

  // All elements different — replaces the whole array
  LcsVecStruct { items: vec![6, 7, 8] }.patch_js(&js, |_| {});

  let arr = get_items_arr(&js);
  assert_eq!(arr.length(), 3);
  assert_eq!(arr.get(0).as_f64().unwrap_throw(), 6.0);
}

#[wasm_bindgen_test]
fn test_patch_js_vec_atomic_default_unchanged() {
  // Same content → each element's patch_js sees no change → no mutations.
  let old = vec![1u32, 2, 3];
  let js = old.to_js();
  let mut replaced = false;
  vec![1u32, 2, 3].patch_js(&js, |_| replaced = true);
  assert!(!replaced, "Vec with same content should not replace");
}

#[wasm_bindgen_test]
fn test_patch_js_vec_atomic_default_changed() {
  // Positional delegation: patches elements in place, root array is NOT replaced.
  let old = vec![1u32, 2, 3];
  let js = old.to_js();
  let mut replaced = false;
  vec![1u32, 2, 4].patch_js(&js, |_| replaced = true);
  // Root set is NOT called — element 2 is patched in place via arr.set(2, ...)
  assert!(!replaced, "Vec should patch elements in place, not replace root");

  // Verify the element was actually updated
  let arr: js_sys::Array = js.into();
  assert_eq!(arr.get(2).as_f64().unwrap_throw(), 4.0);
}

// ===========================================================================
// patch_js: Box<T> delegation
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_box_unchanged() {
  let val: Box<u32> = Box::new(42);
  let js = val.to_js();
  let mut called = false;
  let same: Box<u32> = Box::new(42);
  same.patch_js(&js, |_| called = true);
  assert!(!called, "same boxed value should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_box_changed() {
  let val: Box<u32> = Box::new(42);
  let js = val.to_js();
  let mut called = false;
  let diff: Box<u32> = Box::new(99);
  diff.patch_js(&js, |_| called = true);
  assert!(called, "different boxed value should call set");
}

// ===========================================================================
// patch_js: HashMap/BTreeMap (delegating per-key patch_js)
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_hashmap_unchanged() {
  let mut map = std::collections::HashMap::new();
  map.insert("a".to_string(), 1u32);
  let js = map.to_js();
  let mut called = false;
  map.patch_js(&js, |_| called = true);
  assert!(!called, "same HashMap should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_hashmap_changed() {
  let mut map = std::collections::HashMap::new();
  map.insert("a".to_string(), 1u32);
  let js = map.to_js();
  map.insert("b".to_string(), 2);
  let mut called = false;
  map.patch_js(&js, |_| called = true);
  assert!(!called, "HashMap delegates in place, should not call set");
  // Verify the new key was added in place
  let b_val = js_sys::Reflect::get(&js, &JsValue::from_str("b")).unwrap_throw();
  assert_eq!(b_val.as_f64().unwrap_throw(), 2.0);
}

#[wasm_bindgen_test]
fn test_patch_js_btreemap_unchanged() {
  let mut map = std::collections::BTreeMap::new();
  map.insert("x".to_string(), 10u32);
  let js = map.to_js();
  let mut called = false;
  map.patch_js(&js, |_| called = true);
  assert!(!called, "same BTreeMap should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_hashmap_value_changed() {
  let mut map = std::collections::HashMap::new();
  map.insert("a".to_string(), 1u32);
  let js = map.to_js();

  // Change value for existing key
  let mut updated = std::collections::HashMap::new();
  updated.insert("a".to_string(), 99u32);
  updated.patch_js(&js, |_| panic!("should patch in place, not replace"));

  // The value was updated on the same JS object
  let a_val = js_sys::Reflect::get(&js, &JsValue::from_str("a")).unwrap_throw();
  assert_eq!(a_val.as_f64().unwrap_throw(), 99.0);
}

#[wasm_bindgen_test]
fn test_patch_js_hashmap_key_added() {
  let mut map = std::collections::HashMap::new();
  map.insert("a".to_string(), 1u32);
  let js = map.to_js();

  // Add a new key
  let mut updated = std::collections::HashMap::new();
  updated.insert("a".to_string(), 1u32);
  updated.insert("b".to_string(), 2u32);
  updated.patch_js(&js, |_| panic!("should patch in place, not replace"));

  // Both keys present on the same object
  let a_val = js_sys::Reflect::get(&js, &JsValue::from_str("a")).unwrap_throw();
  assert_eq!(a_val.as_f64().unwrap_throw(), 1.0);
  let b_val = js_sys::Reflect::get(&js, &JsValue::from_str("b")).unwrap_throw();
  assert_eq!(b_val.as_f64().unwrap_throw(), 2.0);
}

#[wasm_bindgen_test]
fn test_patch_js_hashmap_key_removed() {
  let mut map = std::collections::HashMap::new();
  map.insert("a".to_string(), 1u32);
  map.insert("b".to_string(), 2u32);
  let js = map.to_js();

  // Remove key "b"
  let mut updated = std::collections::HashMap::new();
  updated.insert("a".to_string(), 1u32);
  updated.patch_js(&js, |_| panic!("should patch in place, not replace"));

  // "a" still present, "b" deleted
  let a_val = js_sys::Reflect::get(&js, &JsValue::from_str("a")).unwrap_throw();
  assert_eq!(a_val.as_f64().unwrap_throw(), 1.0);
  let b_val = js_sys::Reflect::get(&js, &JsValue::from_str("b")).unwrap_throw();
  assert!(b_val.is_undefined(), "deleted key should be undefined");
}

#[wasm_bindgen_test]
fn test_patch_js_btreemap_delegate() {
  let mut map = std::collections::BTreeMap::new();
  map.insert("x".to_string(), 10u32);
  let js = map.to_js();

  // Change value for existing key
  let mut updated = std::collections::BTreeMap::new();
  updated.insert("x".to_string(), 42u32);
  updated.patch_js(&js, |_| panic!("should patch in place, not replace"));

  // The value was updated on the same JS object
  let x_val = js_sys::Reflect::get(&js, &JsValue::from_str("x")).unwrap_throw();
  assert_eq!(x_val.as_f64().unwrap_throw(), 42.0);
}

// ===========================================================================
// patch_js_map: array-is-object guard
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_hashmap_on_array_replaces() {
  // Old is a JS array but new is a HashMap (object) — must replace, not patch in place
  let old_arr = vec![1u32, 2, 3].to_js();

  let mut map = std::collections::HashMap::new();
  map.insert("a".to_string(), 1u32);
  let mut replaced = None;
  map.patch_js(&old_arr, |v| replaced = Some(v));
  assert!(replaced.is_some(), "HashMap on array old should replace");

  let obj = replaced.unwrap_throw();
  let a = js_sys::Reflect::get(&obj, &JsValue::from_str("a")).unwrap_throw();
  assert_eq!(a.as_f64().unwrap_throw(), 1.0);
}

#[wasm_bindgen_test]
fn test_patch_js_btreemap_on_array_replaces() {
  let old_arr = vec![1u32, 2, 3].to_js();

  let mut map = std::collections::BTreeMap::new();
  map.insert("x".to_string(), 42u32);
  let mut replaced = None;
  map.patch_js(&old_arr, |v| replaced = Some(v));
  assert!(replaced.is_some(), "BTreeMap on array old should replace");
}

// ===========================================================================
// patch_js: array [T; N] (element-by-element delegation)
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_fixed_array_unchanged() {
  let arr = [1u32, 2, 3];
  let js = arr.to_js();
  let mut called = false;
  [1u32, 2, 3].patch_js(&js, |_| called = true);
  assert!(!called, "same fixed array should not call root set");
}

#[wasm_bindgen_test]
fn test_patch_js_fixed_array_changed() {
  let arr = [1u32, 2, 3];
  let js = arr.to_js();
  // Element-by-element: root set is NOT called, elements are patched in place.
  let mut called = false;
  [1u32, 2, 4].patch_js(&js, |_| called = true);
  assert!(!called, "element delegation should not call root set");
  // Verify element was updated in place.
  let updated: js_sys::Array = js.clone().into();
  assert_eq!(updated.get(2).as_f64().unwrap_throw(), 4.0);
}

// ===========================================================================
// patch_js: LCS with struct elements (patch elements in place)
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_lcs_struct_element_patched() {
  let old = LcsStructVec {
    items: vec![LcsItem { id: 1, label: "a".into() }, LcsItem { id: 2, label: "b".into() }],
  };
  let js = old.to_js();
  let arr = js_sys::Reflect::get(&js, &JsValue::from_str("items")).unwrap_throw();
  let arr: js_sys::Array = arr.into();
  let elem0_before = arr.get(0);

  let new = LcsStructVec {
    items: vec![
      LcsItem { id: 1, label: "updated".into() }, // same id, different label
      LcsItem { id: 2, label: "b".into() },       // unchanged
    ],
  };
  new.patch_js(&js, |_| panic!("should patch in place"));

  let arr_after = js_sys::Reflect::get(&js, &JsValue::from_str("items")).unwrap_throw();
  let arr_after: js_sys::Array = arr_after.into();
  assert_eq!(arr_after.length(), 2);

  // First element was patched in place — same JS object identity
  let elem0_after = arr_after.get(0);
  assert!(
    js_sys::Object::is(&elem0_before, &elem0_after),
    "patched element should preserve object identity"
  );
  assert_eq!(
    js_sys::Reflect::get(&elem0_after, &JsValue::from_str("label"))
      .unwrap_throw()
      .as_string()
      .unwrap_throw(),
    "updated"
  );
}

// ===========================================================================
// patch_js: LCS empty to non-empty and vice versa
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_lcs_empty_to_filled() {
  let old = LcsVecStruct { items: vec![] };
  let js = old.to_js();
  LcsVecStruct { items: vec![1, 2, 3] }.patch_js(&js, |_| panic!("should splice in place"));
  let arr = get_items_arr(&js);
  assert_eq!(arr.length(), 3);
}

#[wasm_bindgen_test]
fn test_patch_js_lcs_filled_to_empty() {
  let old = LcsVecStruct { items: vec![1, 2, 3] };
  let js = old.to_js();
  LcsVecStruct { items: vec![] }.patch_js(&js, |_| panic!("should splice in place"));
  let arr = get_items_arr(&js);
  assert_eq!(arr.length(), 0);
}

// ===========================================================================
// patch_js: Vec<T> positional delegation
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_vec_element_delegate() {
  // Vec<ContextInner> — changing one element's field should delegate to
  // T::patch_js, preserving the JS object identity of that element.
  let old = vec![ContextInner { port: 8080 }, ContextInner { port: 9090 }];
  let js = old.to_js();
  let arr: js_sys::Array = js.clone().into();
  let elem0_before = arr.get(0);
  let elem1_before = arr.get(1);

  // Change port on second element only
  let new = vec![ContextInner { port: 8080 }, ContextInner { port: 3000 }];
  new.patch_js(&js, |_| panic!("should not replace root array"));

  let elem0_after = arr.get(0);
  let elem1_after = arr.get(1);

  // Both element JS objects keep identity (patched in place)
  assert!(elem0_before == elem0_after, "unchanged element should keep identity");
  assert!(elem1_before == elem1_after, "changed element should keep identity (patched in place)");

  // But the port value was updated
  assert_eq!(
    js_sys::Reflect::get(&elem1_after, &JsValue::from_str("port"))
      .unwrap_throw()
      .as_f64()
      .unwrap_throw(),
    3000.0
  );
}

#[wasm_bindgen_test]
fn test_patch_js_vec_grow() {
  // Vec grows by 2 elements — existing elements unchanged, new ones appended.
  let old = vec![ContextInner { port: 80 }];
  let js = old.to_js();
  let arr: js_sys::Array = js.clone().into();
  let elem0_before = arr.get(0);

  let new =
    vec![ContextInner { port: 80 }, ContextInner { port: 443 }, ContextInner { port: 8080 }];
  new.patch_js(&js, |_| panic!("should not replace root array"));

  assert_eq!(arr.length(), 3);
  // Existing element keeps identity
  assert!(elem0_before == arr.get(0), "existing element should keep identity");
  // New elements appended
  assert_eq!(
    js_sys::Reflect::get(&arr.get(1), &JsValue::from_str("port"))
      .unwrap_throw()
      .as_f64()
      .unwrap_throw(),
    443.0
  );
  assert_eq!(
    js_sys::Reflect::get(&arr.get(2), &JsValue::from_str("port"))
      .unwrap_throw()
      .as_f64()
      .unwrap_throw(),
    8080.0
  );
}

#[wasm_bindgen_test]
fn test_patch_js_vec_shrink() {
  // Vec shrinks by 2 elements — remaining elements patched, tail truncated.
  let old =
    vec![ContextInner { port: 80 }, ContextInner { port: 443 }, ContextInner { port: 8080 }];
  let js = old.to_js();
  let arr: js_sys::Array = js.clone().into();
  let elem0_before = arr.get(0);

  let new = vec![ContextInner { port: 80 }];
  new.patch_js(&js, |_| panic!("should not replace root array"));

  assert_eq!(arr.length(), 1);
  // Surviving element keeps identity
  assert!(elem0_before == arr.get(0), "surviving element should keep identity");
}

#[wasm_bindgen_test]
fn test_patch_js_vec_same() {
  // Same content — no mutations at all.
  let old = vec![ContextInner { port: 8080 }, ContextInner { port: 9090 }];
  let js = old.to_js();
  let arr: js_sys::Array = js.clone().into();
  let elem0_before = arr.get(0);
  let elem1_before = arr.get(1);

  let new = vec![ContextInner { port: 8080 }, ContextInner { port: 9090 }];
  new.patch_js(&js, |_| panic!("should not replace root array"));

  assert_eq!(arr.length(), 2);
  assert!(elem0_before == arr.get(0), "element 0 identity preserved");
  assert!(elem1_before == arr.get(1), "element 1 identity preserved");
}

// ===========================================================================
// patch_js: [T; N] element-by-element delegation preserves identity
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_fixed_array_delegate() {
  let old = [ContextInner { port: 8080 }, ContextInner { port: 3000 }];
  let js = old.to_js();
  let arr: js_sys::Array = js.clone().into();
  let elem0_before = arr.get(0);
  let elem1_before = arr.get(1);

  // Only change element 1
  let new = [ContextInner { port: 8080 }, ContextInner { port: 9090 }];
  new.patch_js(&js, |_| panic!("should not replace root array"));

  // Element 0 identity preserved (unchanged)
  assert!(elem0_before == arr.get(0), "element 0 identity preserved");
  // Element 1 identity preserved (patched in place)
  assert!(elem1_before == arr.get(1), "element 1 identity preserved");
  // Element 1 port was updated
  assert_eq!(
    js_sys::Reflect::get(&arr.get(1), &JsValue::from_str("port"))
      .unwrap_throw()
      .as_f64()
      .unwrap_throw(),
    9090.0
  );
}

// ===========================================================================
// patch_js: tuple element-by-element delegation
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_tuple_delegate() {
  let old = (42u32, "hello".to_string());
  let js = old.to_js();
  let arr: js_sys::Array = js.clone().into();

  // Only change the second element
  let new = (42u32, "world".to_string());
  new.patch_js(&js, |_| panic!("should not replace root array"));

  // First element unchanged
  assert_eq!(arr.get(0).as_f64().unwrap_throw(), 42.0);
  // Second element updated in place
  assert_eq!(arr.get(1).as_string().unwrap_throw(), "world");
}
