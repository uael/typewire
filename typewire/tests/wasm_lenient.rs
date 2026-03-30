#![cfg(target_arch = "wasm32")]

mod common;
use common::*;
use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

// ===========================================================================
// from_js_lenient — Vec<T>
// ===========================================================================

#[wasm_bindgen_test]
fn test_lenient_vec_all_valid() {
  let js = eval("[1, 2, 3]");
  let result = Vec::<u32>::from_js_lenient(js, "test").unwrap_throw();
  assert_eq!(result, vec![1, 2, 3]);
}

#[wasm_bindgen_test]
fn test_lenient_vec_some_invalid() {
  // Mix of valid u32 values and invalid (strings, booleans)
  let js = eval("[1, 'bad', 2, true, 3]");
  let result = Vec::<u32>::from_js_lenient(js, "test").unwrap_throw();
  assert_eq!(result, vec![1, 2, 3], "invalid elements should be skipped");
}

#[wasm_bindgen_test]
fn test_lenient_vec_all_invalid() {
  let js = eval("['a', 'b', 'c']");
  let result = Vec::<u32>::from_js_lenient(js, "test").unwrap_throw();
  assert!(result.is_empty(), "all invalid → empty vec");
}

#[wasm_bindgen_test]
fn test_lenient_vec_non_array() {
  // A string is not an array — should return empty vec, not error
  let js = JsValue::from_str("not an array");
  let result = Vec::<u32>::from_js_lenient(js, "test").unwrap_throw();
  assert!(result.is_empty(), "non-array input → empty vec");
}

#[wasm_bindgen_test]
fn test_lenient_vec_null() {
  let result = Vec::<u32>::from_js_lenient(JsValue::NULL, "test").unwrap_throw();
  assert!(result.is_empty(), "null → empty vec");
}

#[wasm_bindgen_test]
fn test_lenient_vec_undefined() {
  let result = Vec::<u32>::from_js_lenient(JsValue::UNDEFINED, "test").unwrap_throw();
  assert!(result.is_empty(), "undefined → empty vec");
}

#[wasm_bindgen_test]
fn test_lenient_vec_empty_array() {
  let js = eval("[]");
  let result = Vec::<u32>::from_js_lenient(js, "test").unwrap_throw();
  assert!(result.is_empty());
}

#[wasm_bindgen_test]
fn test_lenient_vec_nested_structs() {
  #[derive(Debug, Clone, PartialEq, Typewire)]
  #[serde(rename_all = "camelCase")]
  struct Item {
    name: String,
    value: u32,
  }

  // One valid, one missing required field
  let js = eval(r#"[{"name":"a","value":1}, {"value":2}, {"name":"c","value":3}]"#);
  let result = Vec::<Item>::from_js_lenient(js, "items").unwrap_throw();
  assert_eq!(result.len(), 2, "item missing 'name' should be skipped");
  assert_eq!(result[0].name, "a");
  assert_eq!(result[1].name, "c");
}

// Verify that non-lenient Vec::from_js still fails on invalid elements
#[wasm_bindgen_test]
fn test_non_lenient_vec_fails_on_invalid() {
  let js = eval("[1, 'bad', 2]");
  assert!(Vec::<u32>::from_js(js).is_err(), "non-lenient from_js should propagate errors");
}

// ===========================================================================
// from_js_lenient — Option<T>
// ===========================================================================

#[wasm_bindgen_test]
fn test_lenient_option_valid() {
  let js = JsValue::from_f64(42.0);
  let result = Option::<u32>::from_js_lenient(js, "test").unwrap_throw();
  assert_eq!(result, Some(42));
}

#[wasm_bindgen_test]
fn test_lenient_option_invalid() {
  // "not a number" can't be parsed as u32
  let js = JsValue::from_str("not a number");
  let result = Option::<u32>::from_js_lenient(js, "test").unwrap_throw();
  assert_eq!(result, None, "invalid value → None, not error");
}

#[wasm_bindgen_test]
fn test_lenient_option_null() {
  let result = Option::<u32>::from_js_lenient(JsValue::NULL, "test").unwrap_throw();
  assert_eq!(result, None);
}

#[wasm_bindgen_test]
fn test_lenient_option_undefined() {
  let result = Option::<u32>::from_js_lenient(JsValue::UNDEFINED, "test").unwrap_throw();
  assert_eq!(result, None);
}

// Verify non-lenient Option::from_js propagates inner errors
#[wasm_bindgen_test]
fn test_non_lenient_option_fails_on_invalid() {
  let js = JsValue::from_str("not a number");
  assert!(Option::<u32>::from_js(js).is_err(), "non-lenient from_js should propagate inner errors");
}

// ===========================================================================
// from_js_lenient — HashMap<K, V>
// ===========================================================================

#[wasm_bindgen_test]
fn test_lenient_hashmap_all_valid() {
  let js = eval(r#"({"a": 1, "b": 2})"#);
  let result = std::collections::HashMap::<String, u32>::from_js_lenient(js, "test").unwrap_throw();
  assert_eq!(result.len(), 2);
  assert_eq!(result["a"], 1);
  assert_eq!(result["b"], 2);
}

#[wasm_bindgen_test]
fn test_lenient_hashmap_some_invalid_values() {
  let js = eval(r#"({"a": 1, "b": "bad", "c": 3})"#);
  let result = std::collections::HashMap::<String, u32>::from_js_lenient(js, "test").unwrap_throw();
  assert_eq!(result.len(), 2, "entry with invalid value should be skipped");
  assert_eq!(result["a"], 1);
  assert_eq!(result["c"], 3);
}

#[wasm_bindgen_test]
fn test_lenient_hashmap_all_invalid() {
  let js = eval(r#"({"a": "x", "b": "y"})"#);
  let result = std::collections::HashMap::<String, u32>::from_js_lenient(js, "test").unwrap_throw();
  assert!(result.is_empty(), "all invalid → empty map");
}

#[wasm_bindgen_test]
fn test_lenient_hashmap_non_object() {
  let js = JsValue::from_f64(42.0);
  let result = std::collections::HashMap::<String, u32>::from_js_lenient(js, "test").unwrap_throw();
  assert!(result.is_empty(), "non-object → empty map");
}

// ===========================================================================
// from_js_lenient — BTreeMap<K, V>
// ===========================================================================

#[wasm_bindgen_test]
fn test_lenient_btreemap_all_valid() {
  let js = eval(r#"({"x": 10, "y": 20})"#);
  let result =
    std::collections::BTreeMap::<String, u32>::from_js_lenient(js, "test").unwrap_throw();
  assert_eq!(result.len(), 2);
  assert_eq!(result["x"], 10);
}

#[wasm_bindgen_test]
fn test_lenient_btreemap_some_invalid() {
  let js = eval(r#"({"x": 10, "y": null, "z": 30})"#);
  let result =
    std::collections::BTreeMap::<String, u32>::from_js_lenient(js, "test").unwrap_throw();
  assert_eq!(result.len(), 2, "null value for u32 should be skipped");
  assert_eq!(result["x"], 10);
  assert_eq!(result["z"], 30);
}

#[wasm_bindgen_test]
fn test_lenient_btreemap_non_object() {
  let js = eval("[]");
  let result =
    std::collections::BTreeMap::<String, u32>::from_js_lenient(js, "test").unwrap_throw();
  assert!(result.is_empty(), "array is not a valid object → empty map");
}

// ===========================================================================
// from_js_lenient — IndexSet<T>
// ===========================================================================

#[wasm_bindgen_test]
fn test_lenient_indexset_some_invalid() {
  let js = eval("[1, 'bad', 2, null, 3]");
  let result = indexmap::IndexSet::<u32>::from_js_lenient(js, "test").unwrap_throw();
  assert_eq!(result.len(), 3);
  assert!(result.contains(&1));
  assert!(result.contains(&2));
  assert!(result.contains(&3));
}

#[wasm_bindgen_test]
fn test_lenient_indexset_non_array() {
  let js = JsValue::from_f64(99.0);
  let result = indexmap::IndexSet::<u32>::from_js_lenient(js, "test").unwrap_throw();
  assert!(result.is_empty());
}

// ===========================================================================
// from_js_lenient — IndexMap<K, V>
// ===========================================================================

#[wasm_bindgen_test]
fn test_lenient_indexmap_some_invalid() {
  let js = eval(r#"({"a": 1, "b": "bad", "c": 3})"#);
  let result = indexmap::IndexMap::<String, u32>::from_js_lenient(js, "test").unwrap_throw();
  assert_eq!(result.len(), 2);
  assert_eq!(result["a"], 1);
  assert_eq!(result["c"], 3);
}

#[wasm_bindgen_test]
fn test_lenient_indexmap_non_object() {
  let js = JsValue::from_str("nope");
  let result = indexmap::IndexMap::<String, u32>::from_js_lenient(js, "test").unwrap_throw();
  assert!(result.is_empty());
}

// ===========================================================================
// from_js_lenient — default impl (non-collection types)
// ===========================================================================

#[wasm_bindgen_test]
fn test_lenient_default_delegates_to_from_js() {
  // For non-collection types, from_js_lenient delegates to from_js
  let js = JsValue::from_f64(42.0);
  let result = u32::from_js_lenient(js, "test").unwrap_throw();
  assert_eq!(result, 42);
}

#[wasm_bindgen_test]
fn test_lenient_default_propagates_error() {
  // For non-collection types, from_js_lenient still propagates errors
  let js = JsValue::from_str("not a number");
  assert!(
    u32::from_js_lenient(js, "test").is_err(),
    "default from_js_lenient should propagate errors for non-collection types"
  );
}

// ===========================================================================
// #[typewire(lenient)] derive attribute
// ===========================================================================

#[derive(Debug, Clone, PartialEq, Typewire)]
#[serde(rename_all = "camelCase")]
struct WithLenientVec {
  name: String,
  #[typewire(lenient)]
  items: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Typewire)]
#[serde(rename_all = "camelCase")]
struct WithLenientOption {
  name: String,
  #[typewire(lenient)]
  score: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Typewire)]
#[serde(rename_all = "camelCase")]
struct MixedLenient {
  #[typewire(lenient)]
  items: Vec<u32>,
  required: String,
  #[typewire(lenient)]
  optional: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Typewire)]
#[serde(rename_all = "camelCase")]
struct WithStrictVec {
  items: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Typewire)]
#[serde(rename_all = "camelCase")]
struct WithStrictOption {
  value: Option<u32>,
}

#[wasm_bindgen_test]
fn test_derive_lenient_vec_skips_invalid() {
  let js = eval(r#"({"name": "test", "items": [1, "bad", 2, null, 3]})"#);
  let result = WithLenientVec::from_js(js).unwrap_throw();
  assert_eq!(result.name, "test");
  assert_eq!(result.items, vec![1, 2, 3], "invalid elements skipped");
}

#[wasm_bindgen_test]
fn test_derive_lenient_vec_all_invalid() {
  let js = eval(r#"({"name": "test", "items": ["a", "b"]})"#);
  let result = WithLenientVec::from_js(js).unwrap_throw();
  assert_eq!(result.items, Vec::<u32>::new(), "all invalid → empty vec");
}

#[wasm_bindgen_test]
fn test_derive_lenient_vec_missing_field() {
  // Vec with lenient + typewire_default → defaults to empty vec when absent
  let js = eval(r#"({"name": "test"})"#);
  // Vec doesn't have typewire_default, so missing field is still an error
  assert!(WithLenientVec::from_js(js).is_err(), "missing non-default field should still error");
}

#[wasm_bindgen_test]
fn test_derive_lenient_option_valid() {
  let js = eval(r#"({"name": "test", "score": 99})"#);
  let result = WithLenientOption::from_js(js).unwrap_throw();
  assert_eq!(result.score, Some(99));
}

#[wasm_bindgen_test]
fn test_derive_lenient_option_invalid_becomes_none() {
  let js = eval(r#"({"name": "test", "score": "not a number"})"#);
  let result = WithLenientOption::from_js(js).unwrap_throw();
  assert_eq!(result.score, None, "invalid value → None via lenient");
}

#[wasm_bindgen_test]
fn test_derive_lenient_option_null() {
  let js = eval(r#"({"name": "test", "score": null})"#);
  let result = WithLenientOption::from_js(js).unwrap_throw();
  assert_eq!(result.score, None);
}

#[wasm_bindgen_test]
fn test_derive_lenient_option_missing() {
  let js = eval(r#"({"name": "test"})"#);
  let result = WithLenientOption::from_js(js).unwrap_throw();
  assert_eq!(result.score, None, "missing field → None via typewire_default");
}

#[wasm_bindgen_test]
fn test_derive_mixed_lenient_and_strict() {
  let js = eval(r#"({"items": [1, "x", 2], "required": "hello", "optional": "bad"})"#);
  let result = MixedLenient::from_js(js).unwrap_throw();
  assert_eq!(result.items, vec![1, 2], "lenient vec skips invalid");
  assert_eq!(result.required, "hello", "strict field works normally");
  assert_eq!(result.optional, None, "lenient option defaults to None");
}

#[wasm_bindgen_test]
fn test_derive_mixed_strict_field_still_errors() {
  // missing required (non-lenient) field should still error
  let js = eval(r#"({"items": [1, 2], "optional": 5})"#);
  assert!(
    MixedLenient::from_js(js).is_err(),
    "missing strict field should error even with lenient siblings"
  );
}

#[wasm_bindgen_test]
fn test_derive_strict_vec_fails_on_invalid() {
  let js = eval(r#"({"items": [1, "bad", 2]})"#);
  assert!(WithStrictVec::from_js(js).is_err(), "non-lenient vec should fail on invalid element");
}

#[wasm_bindgen_test]
fn test_derive_strict_option_fails_on_invalid() {
  let js = eval(r#"({"value": "not a number"})"#);
  assert!(
    WithStrictOption::from_js(js).is_err(),
    "non-lenient option should fail on invalid inner value"
  );
}

// ===========================================================================
// from_js_lenient — proxy types (try_from)
// ===========================================================================

#[wasm_bindgen_test]
fn test_lenient_vec_proxy_valid() {
  // All values within [0, 100] — should all pass
  let js = eval(r#"({"scores": [10, 50, 100]})"#);
  let result = LenientBoundedStruct::from_js(js).unwrap_throw();
  assert_eq!(result.scores.len(), 3);
}

#[wasm_bindgen_test]
fn test_lenient_vec_proxy_some_out_of_range() {
  // 200 is out of range for BoundedU32 [0, 100] — should be skipped in lenient mode
  let js = eval(r#"({"scores": [10, 200, 50]})"#);
  let result = LenientBoundedStruct::from_js(js).unwrap_throw();
  assert_eq!(result.scores.len(), 2, "out-of-range value should be skipped in lenient vec");
}

#[wasm_bindgen_test]
fn test_lenient_vec_proxy_all_out_of_range() {
  let js = eval(r#"({"scores": [200, 300, 999]})"#);
  let result = LenientBoundedStruct::from_js(js).unwrap_throw();
  assert!(result.scores.is_empty(), "all out-of-range → empty vec");
}
