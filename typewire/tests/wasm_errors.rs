#![cfg(target_arch = "wasm32")]

mod common;
use common::*;
use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

// ===========================================================================
// Error context
// ===========================================================================

#[wasm_bindgen_test]
fn test_error_context() {
  // Missing nested field — error message includes type and field path.
  // Format: in `ContextOuter`: in `inner`: in `ContextInner`: missing field `port`
  let js = eval("({ name: 'test', inner: {} })");
  let err = ContextOuter::from_js(js).unwrap_err();
  let msg = err.to_string();
  assert!(msg.contains("`ContextOuter`"), "should mention outer type: {msg}");
  assert!(msg.contains("`inner`"), "should mention field name: {msg}");
  assert!(msg.contains("`ContextInner`"), "should mention inner type: {msg}");
  assert!(msg.contains("`port`"), "should mention missing field: {msg}");
}

#[wasm_bindgen_test]
fn test_error_context_deep_nesting() {
  let js = eval("({ middle: { inner: {} } })");
  let err = DeepErrorOuter::from_js(js).unwrap_err();
  let msg = err.to_string();
  assert!(msg.contains("`DeepErrorOuter`"), "should mention outermost: {msg}");
  assert!(msg.contains("`middle`"), "should mention middle field: {msg}");
  assert!(msg.contains("`DeepErrorMiddle`"), "should mention middle type: {msg}");
  assert!(msg.contains("`inner`"), "should mention inner field: {msg}");
  assert!(msg.contains("`ContextInner`"), "should mention innermost type: {msg}");
  assert!(msg.contains("`port`"), "should mention missing leaf field: {msg}");
}

// ===========================================================================
// Missing field errors
// ===========================================================================

#[wasm_bindgen_test]
fn test_missing_field_error() {
  let js = eval("({ })");
  let result = BasicStruct::from_js(js);
  assert!(result.is_err());
  let err = result.unwrap_err();
  let msg = err.to_string();
  assert!(msg.contains("name"), "error should mention missing field: {msg}");
}

// ===========================================================================
// Unknown variant errors
// ===========================================================================

#[wasm_bindgen_test]
fn test_unknown_variant_error() {
  let result = UnitEnum::from_js(JsValue::from_str("Nonexistent"));
  assert!(result.is_err());
  let msg = result.unwrap_err().to_string();
  assert!(msg.contains("Nonexistent"), "error should mention the variant: {msg}");
}

// ===========================================================================
// Error variant coverage
// ===========================================================================

#[wasm_bindgen_test]
fn test_error_unexpected_type() {
  let err = bool::from_js(JsValue::from_f64(1.0)).unwrap_err();
  let msg = err.to_string();
  assert!(msg.contains("expected"), "UnexpectedType error should contain 'expected': {msg}");
}

#[wasm_bindgen_test]
fn test_error_invalid_value_char() {
  let err = char::from_js(JsValue::from_str("ab")).unwrap_err();
  let msg = err.to_string();
  assert!(
    msg.contains("invalid value"),
    "InvalidValue error should contain 'invalid value': {msg}"
  );
}

#[wasm_bindgen_test]
fn test_error_out_of_range() {
  let err = u8::from_js(JsValue::from_f64(256.0)).unwrap_err();
  let msg = err.to_string();
  assert!(msg.contains("out of range"), "OutOfRange error should contain 'out of range': {msg}");
}

#[wasm_bindgen_test]
fn test_error_custom_from_try_from_proxy() {
  // ValidatedString uses try_from, which wraps the error in Error::Custom
  let js = JsValue::from_str("");
  let err = ValidatedString::from_js(js).unwrap_err();
  let msg = err.to_string();
  assert!(msg.contains("empty"), "Custom error from TryFrom should propagate message: {msg}");
}

#[wasm_bindgen_test]
fn test_error_in_context_method() {
  // Test the in_context method directly
  let inner = typewire::Error::MissingField { field: "port" };
  let wrapped = inner.in_context("ContextInner");
  let msg = wrapped.to_string();
  assert!(msg.contains("ContextInner"), "should include context: {msg}");
  assert!(msg.contains("port"), "should include inner error: {msg}");
}
