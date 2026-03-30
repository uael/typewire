#![cfg(target_arch = "wasm32")]

mod common;
use common::*;
use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

#[wasm_bindgen_test]
fn test_base64_to_js() {
  // Directly test base64_encode works
  let encoded = typewire::base64_encode(&[72, 101, 108, 108, 111]);
  assert_eq!(encoded, "SGVsbG8=", "base64_encode should work");

  // Test struct to_js
  let val = Base64Struct {
        data: vec![72, 101, 108, 108, 111], // "Hello"
    };
  let js = val.to_js();
  let json = js_sys::JSON::stringify(&js).unwrap_throw().as_string().unwrap_throw();
  let data = js_sys::Reflect::get(&js, &JsValue::from_str("data")).unwrap_throw();
  assert!(data.is_string(), "data should be a base64 string but got: {json}",);
  assert_eq!(data.as_string().unwrap_throw(), "SGVsbG8=");
}

#[wasm_bindgen_test]
fn test_base64_from_js() {
  let js = eval("({ data: 'SGVsbG8=' })");
  let val = Base64Struct::from_js(js).unwrap_throw();
  assert_eq!(val.data, vec![72, 101, 108, 108, 111]);
}

#[wasm_bindgen_test]
fn test_base64_round_trip() {
  let val = Base64Struct { data: vec![0, 1, 255, 128] };
  let js = val.to_js();
  let back = Base64Struct::from_js(js).unwrap_throw();
  assert_eq!(val, back);
}

#[wasm_bindgen_test]
fn test_base64_patch_js() {
  let old = Base64Struct { data: vec![1, 2, 3] };
  let js = old.to_js();
  let new = Base64Struct { data: vec![4, 5, 6] };
  new.patch_js(&js, |_| panic!("should patch in place"));
  let data = js_sys::Reflect::get(&js, &JsValue::from_str("data")).unwrap_throw();
  assert_eq!(data.as_string().unwrap_throw(), "BAUG"); // base64 of [4,5,6]
}

#[wasm_bindgen_test]
fn test_base64_invalid_from_js() {
  let js = eval("({ data: 'not-valid-base64!!!' })");
  let result = Base64Struct::from_js(js);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_display_to_js() {
  let val = DisplayStruct { id: MyId(12345) };
  let js = val.to_js();
  let id = js_sys::Reflect::get(&js, &JsValue::from_str("id")).unwrap_throw();
  assert_eq!(id.as_string().unwrap_throw(), "12345");
}

#[wasm_bindgen_test]
fn test_display_from_js() {
  let js = eval("({ id: '67890' })");
  let val = DisplayStruct::from_js(js).unwrap_throw();
  assert_eq!(val.id, MyId(67890));
}

#[wasm_bindgen_test]
fn test_display_round_trip() {
  let val = DisplayStruct { id: MyId(42) };
  let js = val.to_js();
  let back = DisplayStruct::from_js(js).unwrap_throw();
  assert_eq!(val, back);
}

#[wasm_bindgen_test]
fn test_display_invalid_from_js() {
  let js = eval("({ id: 'not_a_number' })");
  let result = DisplayStruct::from_js(js);
  assert!(result.is_err());
}

#[wasm_bindgen_test]
fn test_typewire_default_option() {
  assert_eq!(Option::<u32>::or_default(), Some(None));
}

#[wasm_bindgen_test]
fn test_typewire_default_primitive() {
  assert_eq!(u32::or_default(), None);
}

#[wasm_bindgen_test]
fn test_typewire_default_string() {
  assert_eq!(String::or_default(), None);
}

#[wasm_bindgen_test]
fn test_base64_patch_js_unchanged() {
  let val = Base64Struct { data: vec![1, 2, 3] };
  let js = val.to_js();
  let mut called = false;
  val.patch_js(&js, |_| called = true);
  assert!(!called, "same base64 data should not call set");
}

#[wasm_bindgen_test]
fn test_display_patch_js_unchanged() {
  let val = DisplayStruct { id: MyId(42) };
  let js = val.to_js();
  let mut called = false;
  val.patch_js(&js, |_| called = true);
  assert!(!called, "same display field should not call set");
}

#[wasm_bindgen_test]
fn test_display_patch_js_changed() {
  let old = DisplayStruct { id: MyId(42) };
  let js = old.to_js();
  DisplayStruct { id: MyId(99) }.patch_js(&js, |_| panic!("should patch in place"));
  let id = js_sys::Reflect::get(&js, &JsValue::from_str("id")).unwrap_throw();
  assert_eq!(id.as_string().unwrap_throw(), "99");
}

#[wasm_bindgen_test]
fn test_base64_empty_data() {
  let val = Base64Struct { data: vec![] };
  round_trip(&val);
  let js = val.to_js();
  let data = js_sys::Reflect::get(&js, &JsValue::from_str("data")).unwrap_throw();
  assert_eq!(data.as_string().unwrap_throw(), "");
}

#[wasm_bindgen_test]
fn test_base64_from_js_non_string() {
  let js = eval("({ data: 42 })");
  assert!(Base64Struct::from_js(js).is_err(), "non-string should fail for base64");
}

// ===========================================================================
// #[serde(with = "serde_bytes")] — Vec<u8> as Uint8Array
// ===========================================================================

#[wasm_bindgen_test]
fn test_serde_bytes_to_js() {
  let val = SerdeBytesStruct { data: vec![1, 2, 3, 4] };
  let js = val.to_js();
  let data = js_sys::Reflect::get(&js, &JsValue::from_str("data")).unwrap_throw();
  assert!(data.is_instance_of::<js_sys::Uint8Array>(), "serde_bytes should produce Uint8Array");
  let arr = js_sys::Uint8Array::from(data);
  assert_eq!(arr.to_vec(), vec![1, 2, 3, 4]);
}

#[wasm_bindgen_test]
fn test_serde_bytes_from_js_uint8array() {
  let js = eval("({ data: new Uint8Array([10, 20, 30]) })");
  let val = SerdeBytesStruct::from_js(js).unwrap_throw();
  assert_eq!(val.data, vec![10, 20, 30]);
}

#[wasm_bindgen_test]
fn test_serde_bytes_from_js_uint8clampedarray() {
  let js = eval("({ data: new Uint8ClampedArray([5, 6, 7]) })");
  let val = SerdeBytesStruct::from_js(js).unwrap_throw();
  assert_eq!(val.data, vec![5, 6, 7]);
}

#[wasm_bindgen_test]
fn test_serde_bytes_from_js_wrong_type() {
  let js = eval("({ data: [1, 2, 3] })");
  assert!(SerdeBytesStruct::from_js(js).is_err(), "plain Array should fail for serde_bytes");
}

#[wasm_bindgen_test]
fn test_serde_bytes_round_trip() {
  let val = SerdeBytesStruct { data: vec![0, 127, 255] };
  round_trip(&val);
}

#[wasm_bindgen_test]
fn test_serde_bytes_patch_js_unchanged() {
  let val = SerdeBytesStruct { data: vec![1, 2, 3] };
  let js = val.to_js();
  let mut called = false;
  val.patch_js(&js, |_| called = true);
  assert!(!called, "same serde_bytes data should not call set");
}

#[wasm_bindgen_test]
fn test_serde_bytes_patch_js_changed() {
  let old = SerdeBytesStruct { data: vec![1, 2, 3] };
  let js = old.to_js();
  SerdeBytesStruct { data: vec![4, 5, 6] }.patch_js(&js, |_| panic!("should patch in place"));
  let data = js_sys::Reflect::get(&js, &JsValue::from_str("data")).unwrap_throw();
  let arr = js_sys::Uint8Array::from(data);
  assert_eq!(arr.to_vec(), vec![4, 5, 6]);
}

#[wasm_bindgen_test]
fn test_serde_bytes_empty() {
  let val = SerdeBytesStruct { data: vec![] };
  round_trip(&val);
}
