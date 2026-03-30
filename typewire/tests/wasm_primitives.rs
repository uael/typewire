#![cfg(target_arch = "wasm32")]

mod common;
use common::*;
use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_bool() {
  round_trip(&true);
  round_trip(&false);
  assert!(bool::from_js(JsValue::from_str("nope")).is_err());
}

#[wasm_bindgen_test]
fn test_small_integers() {
  round_trip(&0u8);
  round_trip(&255u8);
  round_trip(&0u16);
  round_trip(&65535u16);
  round_trip(&0u32);
  round_trip(&4294967295u32);
  round_trip(&0i8);
  round_trip(&(-128i8));
  round_trip(&127i8);
  round_trip(&0i16);
  round_trip(&(-32768i16));
  round_trip(&0i32);
  round_trip(&(-2147483648i32));
  // Out-of-range
  assert!(u8::from_js(JsValue::from_f64(256.0)).is_err());
  assert!(u8::from_js(JsValue::from_f64(-1.0)).is_err());
  assert!(u8::from_js(JsValue::from_f64(1.5)).is_err());
}

#[wasm_bindgen_test]
fn test_floats() {
  round_trip(&0.0f32);
  round_trip(&std::f32::consts::PI);
  round_trip(&0.0f64);
  round_trip(&std::f64::consts::E);
  assert!(f64::from_js(JsValue::from_str("nope")).is_err());
}

#[wasm_bindgen_test]
fn test_u64() {
  round_trip(&0u64);
  round_trip(&42u64);
  // to_js produces a JS number, not BigInt
  assert!(42u64.to_js().as_f64().is_some());
  // from_js accepts numbers
  assert_eq!(u64::from_js(JsValue::from_f64(42.0)).unwrap_throw(), 42);
  // from_js still accepts BigInt (backward compat)
  assert_eq!(u64::from_js(eval("42n")).unwrap_throw(), 42);
}

#[wasm_bindgen_test]
fn test_i64() {
  round_trip(&0i64);
  round_trip(&42i64);
  round_trip(&(-42i64));
  // to_js produces a JS number, not BigInt
  assert!((-5i64).to_js().as_f64().is_some());
  // from_js accepts negative numbers
  assert_eq!(i64::from_js(JsValue::from_f64(-5.0)).unwrap_throw(), -5);
  // from_js still accepts BigInt (backward compat)
  assert_eq!(i64::from_js(eval("-5n")).unwrap_throw(), -5);
}

#[wasm_bindgen_test]
fn test_u128() {
  round_trip(&0u128);
  round_trip(&42u128);
  assert!(42u128.to_js().as_f64().is_some());
  assert_eq!(u128::from_js(eval("42n")).unwrap_throw(), 42);
}

#[wasm_bindgen_test]
fn test_i128() {
  round_trip(&0i128);
  round_trip(&42i128);
  round_trip(&(-42i128));
  assert!((-5i128).to_js().as_f64().is_some());
  assert_eq!(i128::from_js(eval("-5n")).unwrap_throw(), -5);
}

#[wasm_bindgen_test]
fn test_usize_isize() {
  round_trip(&0usize);
  round_trip(&42usize);
  round_trip(&0isize);
  round_trip(&(-1isize));
}

#[wasm_bindgen_test]
fn test_char() {
  round_trip(&'a');
  round_trip(&'中');
  round_trip(&'🦀');
  assert!(char::from_js(JsValue::from_str("ab")).is_err());
  assert!(char::from_js(JsValue::from_str("")).is_err());
}

#[wasm_bindgen_test]
fn test_string() {
  round_trip(&String::new());
  round_trip(&"hello world".to_string());
  round_trip(&"日本語".to_string());
  assert!(String::from_js(JsValue::from_f64(42.0)).is_err());
}

#[wasm_bindgen_test]
fn test_unit() {
  round_trip(&());
  assert!(().to_js().is_null());
}

#[wasm_bindgen_test]
fn test_from_js_wrong_type_bool() {
  assert!(bool::from_js(JsValue::from_f64(1.0)).is_err());
}

#[wasm_bindgen_test]
fn test_from_js_wrong_type_u32() {
  assert!(u32::from_js(JsValue::from_str("hello")).is_err());
}

#[wasm_bindgen_test]
fn test_from_js_wrong_type_string() {
  assert!(String::from_js(JsValue::from_f64(42.0)).is_err());
}

#[wasm_bindgen_test]
fn test_from_js_cow_str() {
  let js = JsValue::from_str("hello");
  let val = std::borrow::Cow::<str>::from_js(js).unwrap_throw();
  assert_eq!(val, "hello");
}

#[wasm_bindgen_test]
fn test_float_nan_round_trip() {
  let nan = f64::from_js(eval("NaN")).unwrap_throw();
  assert!(nan.is_nan(), "NaN should round-trip as NaN");
  let nan32 = f32::from_js(eval("NaN")).unwrap_throw();
  assert!(nan32.is_nan(), "NaN should round-trip as f32 NaN");
}

#[wasm_bindgen_test]
fn test_float_infinity_round_trip() {
  let inf = f64::from_js(eval("Infinity")).unwrap_throw();
  assert!(inf.is_infinite() && inf.is_sign_positive());
  let neg_inf = f64::from_js(eval("-Infinity")).unwrap_throw();
  assert!(neg_inf.is_infinite() && neg_inf.is_sign_negative());
}

#[wasm_bindgen_test]
fn test_cow_str_patch_js() {
  use std::borrow::Cow;
  let old = JsValue::from_str("hello");
  let mut called = false;
  Cow::<str>::Owned("hello".into()).patch_js(&old, |_| called = true);
  assert!(!called, "same Cow<str> should not call set");

  let mut new_val = None;
  Cow::<str>::Owned("world".into()).patch_js(&old, |v| new_val = Some(v));
  assert_eq!(new_val.unwrap_throw().as_string().unwrap_throw(), "world");
}
