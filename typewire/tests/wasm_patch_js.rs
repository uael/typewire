#![cfg(target_arch = "wasm32")]

mod common;
use common::*;
use typewire::Typewire;
use wasm_bindgen::prelude::*;
use wasm_bindgen_test::*;

// ===========================================================================
// patch_js: undefined/null old value (field doesn't exist in JS yet)
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_struct_on_undefined() {
  // When old is undefined, struct patch_js should create the whole object via set
  let val = BasicStruct { name: "Alice".into(), age: 30 };
  let mut created = None;
  val.patch_js(&JsValue::UNDEFINED, |v| created = Some(v));
  let created = created.expect("should call set when old is undefined");
  assert_eq!(
    js_sys::Reflect::get(&created, &JsValue::from_str("name"))
      .unwrap_throw()
      .as_string()
      .unwrap_throw(),
    "Alice"
  );
}

#[wasm_bindgen_test]
fn test_patch_js_struct_on_null() {
  let val = BasicStruct { name: "Bob".into(), age: 25 };
  let mut created = None;
  val.patch_js(&JsValue::NULL, |v| created = Some(v));
  assert!(created.is_some(), "should call set when old is null");
}

#[wasm_bindgen_test]
fn test_patch_js_nested_struct_with_undefined_child() {
  // Parent exists but a child field is undefined — should create the child
  let parent = eval("({ name: 'test' })"); // inner field is absent
  let val = ContextOuter { name: "test".into(), inner: ContextInner { port: 8080 } };
  val.patch_js(&parent, |_| panic!("should not replace root"));
  // inner should now exist
  let inner = js_sys::Reflect::get(&parent, &JsValue::from_str("inner")).unwrap_throw();
  assert!(!inner.is_undefined(), "inner should have been created");
  assert_eq!(
    js_sys::Reflect::get(&inner, &JsValue::from_str("port")).unwrap_throw().as_f64().unwrap_throw(),
    8080.0
  );
}

#[wasm_bindgen_test]
fn test_patch_js_internal_enum_on_undefined() {
  let val = InternallyTagged::Named { field_one: 42, field_two: "hello".into() };
  let mut created = None;
  val.patch_js(&JsValue::UNDEFINED, |v| created = Some(v));
  let created = created.expect("should call set when old is undefined");
  assert_eq!(
    js_sys::Reflect::get(&created, &JsValue::from_str("type"))
      .unwrap_throw()
      .as_string()
      .unwrap_throw(),
    "Named"
  );
}

#[wasm_bindgen_test]
fn test_patch_js_external_enum_on_undefined() {
  let val = MixedExternalEnum::Struct { x: 10, y: 20 };
  let mut created = None;
  val.patch_js(&JsValue::UNDEFINED, |v| created = Some(v));
  assert!(created.is_some(), "should call set when old is undefined");
}

#[wasm_bindgen_test]
fn test_patch_js_adjacent_enum_on_undefined() {
  let val = AdjacentlyTagged::Named { a: 1, b: "hi".into() };
  let mut created = None;
  val.patch_js(&JsValue::UNDEFINED, |v| created = Some(v));
  assert!(created.is_some(), "should call set when old is undefined");
}

// ===========================================================================
// patch_js: primitives
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_primitive_unchanged() {
  let old = JsValue::from_f64(42.0);
  let mut called = false;
  42u32.patch_js(&old, |_| called = true);
  assert!(!called, "should not call set when value is the same");
}

#[wasm_bindgen_test]
fn test_patch_js_primitive_changed() {
  let old = JsValue::from_f64(42.0);
  let mut new_val = None;
  99u32.patch_js(&old, |v| new_val = Some(v));
  assert_eq!(new_val.unwrap_throw().as_f64().unwrap_throw(), 99.0);
}

#[wasm_bindgen_test]
fn test_patch_js_string_unchanged() {
  let old = JsValue::from_str("hello");
  let mut called = false;
  "hello".to_string().patch_js(&old, |_| called = true);
  assert!(!called);
}

#[wasm_bindgen_test]
fn test_patch_js_string_changed() {
  let old = JsValue::from_str("hello");
  let mut new_val = None;
  "world".to_string().patch_js(&old, |v| new_val = Some(v));
  assert_eq!(new_val.unwrap_throw().as_string().unwrap_throw(), "world");
}

#[wasm_bindgen_test]
fn test_patch_js_option_none_to_some() {
  let old = JsValue::NULL;
  let mut new_val = None;
  Some(42u32).patch_js(&old, |v| new_val = Some(v));
  assert_eq!(new_val.unwrap_throw().as_f64().unwrap_throw(), 42.0);
}

#[wasm_bindgen_test]
fn test_patch_js_option_some_to_none() {
  let old = JsValue::from_f64(42.0);
  let mut new_val = None;
  Option::<u32>::None.patch_js(&old, |v| new_val = Some(v));
  assert!(new_val.unwrap_throw().is_null());
}

// ===========================================================================
// patch_js: structs
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_struct_unchanged() {
  let val = BasicStruct { name: "Alice".into(), age: 30 };
  let js = val.to_js();
  // Patch with same values — JS object should be untouched
  val.patch_js(&js, |_| panic!("should not replace struct"));
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("name")).unwrap_throw().as_string().unwrap_throw(),
    "Alice"
  );
}

#[wasm_bindgen_test]
fn test_patch_js_struct_field_changed() {
  let old_val = BasicStruct { name: "Alice".into(), age: 30 };
  let js = old_val.to_js();
  let new_val = BasicStruct {
    name: "Bob".into(),
    age: 30, // unchanged
  };
  // Patch — should update name in place, age untouched
  new_val.patch_js(&js, |_| panic!("should not replace struct"));
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("name")).unwrap_throw().as_string().unwrap_throw(),
    "Bob"
  );
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("age")).unwrap_throw().as_f64().unwrap_throw(),
    30.0
  );
}

#[wasm_bindgen_test]
fn test_patch_js_struct_rename_all() {
  let old_val =
    CamelCaseStruct { first_name: "Alice".into(), last_name: "Smith".into(), is_active: true };
  let js = old_val.to_js();
  let new_val =
    CamelCaseStruct { first_name: "Alice".into(), last_name: "Jones".into(), is_active: true };
  new_val.patch_js(&js, |_| panic!("should not replace"));
  // Should use camelCase key
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("lastName"))
      .unwrap_throw()
      .as_string()
      .unwrap_throw(),
    "Jones"
  );
  // firstName unchanged
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("firstName"))
      .unwrap_throw()
      .as_string()
      .unwrap_throw(),
    "Alice"
  );
}

#[wasm_bindgen_test]
fn test_patch_js_nested_struct() {
  let old_val = ContextOuter { name: "test".into(), inner: ContextInner { port: 8080 } };
  let js = old_val.to_js();
  let inner_js_before = js_sys::Reflect::get(&js, &JsValue::from_str("inner")).unwrap_throw();

  let new_val = ContextOuter {
    name: "test".into(), // unchanged
    inner: ContextInner { port: 9090 },
  };
  new_val.patch_js(&js, |_| panic!("should not replace root"));

  // Inner object identity is preserved (same JS object)
  let inner_js_after = js_sys::Reflect::get(&js, &JsValue::from_str("inner")).unwrap_throw();
  assert!(inner_js_before == inner_js_after, "inner object identity should be preserved");
  // But port was updated
  assert_eq!(
    js_sys::Reflect::get(&inner_js_after, &JsValue::from_str("port"))
      .unwrap_throw()
      .as_f64()
      .unwrap_throw(),
    9090.0
  );
}

#[wasm_bindgen_test]
fn test_patch_js_optional_field() {
  let old_val =
    OptionalFields { required: "hello".into(), optional: Some(42), also_optional: None };
  let js = old_val.to_js();
  let new_val = OptionalFields {
    required: "hello".into(),
    optional: None,
    also_optional: Some("world".into()),
  };
  new_val.patch_js(&js, |_| panic!("should not replace"));
  assert!(js_sys::Reflect::get(&js, &JsValue::from_str("optional")).unwrap_throw().is_null());
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("also_optional"))
      .unwrap_throw()
      .as_string()
      .unwrap_throw(),
    "world"
  );
}

// ===========================================================================
// patch_js: enums — internally tagged
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_internal_enum_same_variant() {
  let old = InternallyTagged::Named { field_one: 1, field_two: "hello".into() };
  let js = old.to_js();

  let new = InternallyTagged::Named {
    field_one: 1,              // unchanged
    field_two: "world".into(), // changed
  };
  new.patch_js(&js, |_| panic!("should patch in place, not replace"));
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("fieldTwo"))
      .unwrap_throw()
      .as_string()
      .unwrap_throw(),
    "world"
  );
  // Tag preserved
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("type")).unwrap_throw().as_string().unwrap_throw(),
    "Named"
  );
}

#[wasm_bindgen_test]
fn test_patch_js_internal_enum_variant_change() {
  let old = InternallyTagged::Unit;
  let js = old.to_js();

  let new = InternallyTagged::Named { field_one: 42, field_two: "new".into() };
  let mut replaced = None;
  new.patch_js(&js, |v| replaced = Some(v));
  // Variant changed — should call set with a full replacement
  let replaced = replaced.expect("should replace when variant changes");
  assert_eq!(
    js_sys::Reflect::get(&replaced, &JsValue::from_str("type"))
      .unwrap_throw()
      .as_string()
      .unwrap_throw(),
    "Named"
  );
}

// ===========================================================================
// patch_js: enums — adjacently tagged
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_adjacent_enum_same_variant() {
  let old = AdjacentlyTagged::Named { a: 1, b: "hello".into() };
  let js = old.to_js();

  let new = AdjacentlyTagged::Named { a: 99, b: "hello".into() };
  new.patch_js(&js, |_| panic!("should patch in place"));
  let content = js_sys::Reflect::get(&js, &JsValue::from_str("c")).unwrap_throw();
  assert_eq!(
    js_sys::Reflect::get(&content, &JsValue::from_str("a")).unwrap_throw().as_f64().unwrap_throw(),
    99.0
  );
}

#[wasm_bindgen_test]
fn test_patch_js_adjacent_enum_variant_change() {
  let old = AdjacentlyTagged::Unit;
  let js = old.to_js();
  let mut replaced = false;
  AdjacentlyTagged::Single(42).patch_js(&js, |_| replaced = true);
  assert!(replaced, "should replace when variant changes");
}

// ===========================================================================
// patch_js: enums — externally tagged
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_external_enum_same_struct_variant() {
  let old = MixedExternalEnum::Struct { x: 1, y: 2 };
  let js = old.to_js();

  let new = MixedExternalEnum::Struct { x: 99, y: 2 };
  new.patch_js(&js, |_| panic!("should patch in place"));
  let content = js_sys::Reflect::get(&js, &JsValue::from_str("Struct")).unwrap_throw();
  assert_eq!(
    js_sys::Reflect::get(&content, &JsValue::from_str("x")).unwrap_throw().as_f64().unwrap_throw(),
    99.0
  );
}

#[wasm_bindgen_test]
fn test_patch_js_external_enum_variant_change() {
  let old = MixedExternalEnum::Unit;
  let js = old.to_js();
  let mut replaced = false;
  MixedExternalEnum::Newtype(42).patch_js(&js, |_| replaced = true);
  assert!(replaced, "should replace when variant changes");
}

// ===========================================================================
// patch_js: diffable(atomic)
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_atomic_struct_unchanged() {
  // Atomic structs deserialize old JsValue back to Self and compare via PartialEq.
  // Same content → equal → skip.
  let val = AtomicStruct { x: 1, y: 2 };
  let js = val.to_js();
  let mut called = false;
  val.patch_js(&js, |_| called = true);
  assert!(!called, "atomic struct: same content should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_atomic_struct_changed() {
  let old = AtomicStruct { x: 1, y: 2 };
  let js = old.to_js();
  let mut replaced = None;
  AtomicStruct { x: 1, y: 99 }.patch_js(&js, |v| replaced = Some(v));
  // Atomic: replaces the whole object even though only y changed
  let replaced = replaced.expect("atomic struct should replace entirely");
  assert_eq!(
    js_sys::Reflect::get(&replaced, &JsValue::from_str("y")).unwrap_throw().as_f64().unwrap_throw(),
    99.0
  );
}

// ===========================================================================
// patch_js: transparent (visit_transparent)
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_transparent_unchanged() {
  let val = TransparentWrapper("hello".into());
  let js = val.to_js();
  let mut called = false;
  val.patch_js(&js, |_| called = true);
  assert!(!called, "transparent: same inner value should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_transparent_changed() {
  let old = TransparentWrapper("hello".into());
  let js = old.to_js();
  let mut new_val = None;
  TransparentWrapper("world".into()).patch_js(&js, |v| new_val = Some(v));
  assert_eq!(new_val.unwrap_throw().as_string().unwrap_throw(), "world");
}

// ===========================================================================
// patch_js: #[serde(skip)] fields excluded
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_skip_field_excluded() {
  let old = SkipFieldStruct { visible: "old".into(), hidden: 999 };
  let js = old.to_js();
  // Verify hidden is not in JS
  assert!(js_sys::Reflect::get(&js, &JsValue::from_str("hidden")).unwrap_throw().is_undefined());

  let new = SkipFieldStruct {
    visible: "new".into(),
    hidden: 123, // different, but skipped
  };
  new.patch_js(&js, |_| panic!("should patch in place"));
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("visible"))
      .unwrap_throw()
      .as_string()
      .unwrap_throw(),
    "new"
  );
  // hidden still absent
  assert!(js_sys::Reflect::get(&js, &JsValue::from_str("hidden")).unwrap_throw().is_undefined());
}

// ===========================================================================
// patch_js: untagged enum — always replaces
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_untagged_enum_always_replaces() {
  let old = Untagged::Num(42);
  let js = old.to_js();
  let mut replaced = false;
  // Same variant, different value
  Untagged::Num(99).patch_js(&js, |_| replaced = true);
  assert!(replaced, "untagged enum should always replace");
}

// ===========================================================================
// patch_js: verify unchanged fields preserve JS identity
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_unchanged_field_identity() {
  // Create a struct with a nested struct field
  let val = ContextOuter { name: "original".into(), inner: ContextInner { port: 8080 } };
  let js = val.to_js();
  let inner_before = js_sys::Reflect::get(&js, &JsValue::from_str("inner")).unwrap_throw();
  let name_before = js_sys::Reflect::get(&js, &JsValue::from_str("name")).unwrap_throw();

  // Patch with ONLY inner.port changed
  let new_val = ContextOuter {
    name: "original".into(), // same
    inner: ContextInner { port: 9090 },
  };
  new_val.patch_js(&js, |_| panic!("should not replace root"));

  let inner_after = js_sys::Reflect::get(&js, &JsValue::from_str("inner")).unwrap_throw();
  let name_after = js_sys::Reflect::get(&js, &JsValue::from_str("name")).unwrap_throw();

  // Inner object identity preserved (patched in place)
  assert!(inner_before == inner_after, "unchanged nested struct should keep identity");
  // Name is a primitive — JS === on same string value is true
  assert!(name_before == name_after, "unchanged primitive should be equal via ===");
  // But port was updated
  assert_eq!(
    js_sys::Reflect::get(&inner_after, &JsValue::from_str("port"))
      .unwrap_throw()
      .as_f64()
      .unwrap_throw(),
    9090.0
  );
}

// ===========================================================================
// patch_js: internally tagged newtype — same variant
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_internal_newtype_same_variant() {
  let old = InternallyTagged::Newtype(Inner { x: 1, y: 2 });
  let js = old.to_js();

  let new = InternallyTagged::Newtype(Inner { x: 99, y: 2 });
  new.patch_js(&js, |_| panic!("should patch newtype in place"));
  // x updated, y unchanged, tag preserved
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("type")).unwrap_throw().as_string().unwrap_throw(),
    "Newtype"
  );
}

// ===========================================================================
// patch_js: externally tagged newtype — same variant
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_external_newtype_same_variant() {
  let old = MixedExternalEnum::Newtype(42);
  let js = old.to_js();

  let mut replaced = None;
  MixedExternalEnum::Newtype(99).patch_js(&js, |v| replaced = Some(v));
  // Newtype wraps a primitive — the content changes, so the inner value
  // is replaced via the set callback on the content
  // For externally tagged: { "Newtype": 42 } → inner 42 gets patched to 99
  // Since 42 !== 99, the content is replaced
  let content = js_sys::Reflect::get(&js, &JsValue::from_str("Newtype")).unwrap_throw();
  // The patch_js for the inner u32 calls set, which Reflect::sets on old
  // Let's verify the new value is accessible (either in old or replaced)
  if let Some(v) = replaced {
    // If the whole thing was replaced, verify the replacement
    assert_eq!(
      js_sys::Reflect::get(&v, &JsValue::from_str("Newtype"))
        .unwrap_throw()
        .as_f64()
        .unwrap_throw(),
      99.0
    );
  } else {
    // Patched in place
    assert_eq!(content.as_f64().unwrap_throw(), 99.0);
  }
}

// ===========================================================================
// patch_js: adjacently tagged newtype — same variant
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_adjacent_newtype_same_variant() {
  let old = AdjacentlyTagged::Single(42);
  let js = old.to_js();

  AdjacentlyTagged::Single(99).patch_js(&js, |_| {});
  let content = js_sys::Reflect::get(&js, &JsValue::from_str("c")).unwrap_throw();
  assert_eq!(content.as_f64().unwrap_throw(), 99.0);
}

// ===========================================================================
// patch_js: #[serde(rename)] on field
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_rename_field() {
  let old = RenameFieldStruct { kind: "old".into(), value: 1 };
  let js = old.to_js();
  let new = RenameFieldStruct { kind: "updated".into(), value: 1 };
  new.patch_js(&js, |_| panic!("should patch in place"));
  // The renamed field uses "type" as JS key (not "kind")
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("type")).unwrap_throw().as_string().unwrap_throw(),
    "updated"
  );
}

// ===========================================================================
// patch_js: bool / Option unchanged identity
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_bool_unchanged() {
  let old = JsValue::from_bool(true);
  let mut called = false;
  true.patch_js(&old, |_| called = true);
  assert!(!called, "same bool should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_bool_changed() {
  let old = JsValue::from_bool(true);
  let mut called = false;
  false.patch_js(&old, |_| called = true);
  assert!(called, "different bool should call set");
}

#[wasm_bindgen_test]
fn test_patch_js_option_unchanged_none() {
  let old = JsValue::NULL;
  let mut called = false;
  Option::<u32>::None.patch_js(&old, |_| called = true);
  assert!(!called, "None to null should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_option_unchanged_some() {
  let old = JsValue::from_f64(42.0);
  let mut called = false;
  Some(42u32).patch_js(&old, |_| called = true);
  assert!(!called, "same Some value should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_option_some_to_some_delegate() {
  let old_inner = ContextInner { port: 8080 };
  let old_js = old_inner.to_js();

  let new_inner = Some(ContextInner { port: 9090 });
  new_inner.patch_js(&old_js, |_| panic!("should patch in place, not replace"));

  // The port field was updated in place on the same JS object.
  assert_eq!(
    js_sys::Reflect::get(&old_js, &JsValue::from_str("port"))
      .unwrap_throw()
      .as_f64()
      .unwrap_throw(),
    9090.0
  );
}

#[wasm_bindgen_test]
fn test_patch_js_option_some_same() {
  let old_inner = ContextInner { port: 8080 };
  let old_js = old_inner.to_js();
  let mut called = false;
  Some(ContextInner { port: 8080 }).patch_js(&old_js, |_| called = true);
  assert!(!called, "same Some value should not call set");
}

// ===========================================================================
// patch_js: struct with all fields changed
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_struct_all_fields_changed() {
  let old_val = BasicStruct { name: "Alice".into(), age: 30 };
  let js = old_val.to_js();
  let new_val = BasicStruct { name: "Bob".into(), age: 25 };
  new_val.patch_js(&js, |_| panic!("should patch in place"));
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("name")).unwrap_throw().as_string().unwrap_throw(),
    "Bob"
  );
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("age")).unwrap_throw().as_f64().unwrap_throw(),
    25.0
  );
}

// ===========================================================================
// patch_js: deeply nested (3 levels) — identity preserved all the way
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_deep_nesting_identity() {
  let old =
    Level1 { level2: Level2 { level3: Level3 { value: 1 }, tag: "a".into() }, name: "root".into() };
  let js = old.to_js();
  let l2_before = js_sys::Reflect::get(&js, &JsValue::from_str("level2")).unwrap_throw();
  let l3_before = js_sys::Reflect::get(&l2_before, &JsValue::from_str("level3")).unwrap_throw();

  // Only change the deepest leaf
  let new = Level1 {
    level2: Level2 { level3: Level3 { value: 99 }, tag: "a".into() },
    name: "root".into(),
  };
  new.patch_js(&js, |_| panic!("should not replace root"));

  let l2_after = js_sys::Reflect::get(&js, &JsValue::from_str("level2")).unwrap_throw();
  let l3_after = js_sys::Reflect::get(&l2_after, &JsValue::from_str("level3")).unwrap_throw();

  // Level2 and Level3 object identity preserved
  assert!(l2_before == l2_after, "level2 identity should be preserved");
  assert!(l3_before == l3_after, "level3 identity should be preserved");
  // But the leaf value changed
  assert_eq!(
    js_sys::Reflect::get(&l3_after, &JsValue::from_str("value"))
      .unwrap_throw()
      .as_f64()
      .unwrap_throw(),
    99.0
  );
}

// ===========================================================================
// patch_js: skip_serializing / skip_deserializing fields
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_skip_serializing_field() {
  let old = SkipSerStruct { name: "old".into(), write_only: 42 };
  let js = old.to_js();
  let new = SkipSerStruct { name: "new".into(), write_only: 99 };
  new.patch_js(&js, |_| panic!("should patch in place"));
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("name")).unwrap_throw().as_string().unwrap_throw(),
    "new"
  );
}

// ===========================================================================
// patch_js: #[serde(flatten)] field
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_flatten_field() {
  let old = FlattenStruct { name: "test".into(), inner: Inner { x: 1, y: 2 } };
  let js = old.to_js();
  let new = FlattenStruct { name: "test".into(), inner: Inner { x: 99, y: 2 } };
  new.patch_js(&js, |_| panic!("should patch in place"));
  // Flatten means x/y are at the top level
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("x")).unwrap_throw().as_f64().unwrap_throw(),
    99.0
  );
}

// ===========================================================================
// patch_js: #[serde(skip_serializing_if)] field — present vs absent
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_skip_serializing_if() {
  let old = SkipSerIfStruct { name: "hello".into(), count: 0 };
  let js = old.to_js();
  let new = SkipSerIfStruct { name: "hello".into(), count: 42 };
  new.patch_js(&js, |_| panic!("should patch in place"));
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("count")).unwrap_throw().as_f64().unwrap_throw(),
    42.0
  );
}

// ===========================================================================
// patch_js: #[serde(default)] field
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_default_field() {
  let old = DefaultFieldStruct { name: "test".into(), count: 0 };
  let js = old.to_js();
  let new = DefaultFieldStruct { name: "test".into(), count: 5 };
  new.patch_js(&js, |_| panic!("should patch in place"));
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("count")).unwrap_throw().as_f64().unwrap_throw(),
    5.0
  );
}

// ===========================================================================
// patch_js: transparent struct
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_transparent_named_unchanged() {
  let val = TransparentNamed { inner: 42 };
  let js = val.to_js();
  let mut called = false;
  val.patch_js(&js, |_| called = true);
  assert!(!called, "same transparent value should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_transparent_named_changed() {
  let old = TransparentNamed { inner: 42 };
  let js = old.to_js();
  let mut new_val = None;
  TransparentNamed { inner: 99 }.patch_js(&js, |v| new_val = Some(v));
  assert_eq!(new_val.unwrap_throw().as_f64().unwrap_throw(), 99.0);
}

#[wasm_bindgen_test]
fn test_patch_js_transparent_tuple_unchanged() {
  let val = TransparentTuple("hello".into());
  let js = val.to_js();
  let mut called = false;
  val.patch_js(&js, |_| called = true);
  assert!(!called);
}

#[wasm_bindgen_test]
fn test_patch_js_transparent_tuple_changed() {
  let old = TransparentTuple("hello".into());
  let js = old.to_js();
  let mut new_val = None;
  TransparentTuple("world".into()).patch_js(&js, |v| new_val = Some(v));
  assert_eq!(new_val.unwrap_throw().as_string().unwrap_throw(), "world");
}

// ===========================================================================
// patch_js: tuple struct
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_tuple_struct_changed() {
  let old = TupleStruct(1, "hello".into());
  let js = old.to_js();
  // Tuple struct delegates element-by-element — patches in place
  TupleStruct(2, "hello".into()).patch_js(&js, |_| panic!("should patch in place"));
  let arr: js_sys::Array = js.into();
  assert_eq!(arr.get(0).as_f64().unwrap_throw(), 2.0);
  assert_eq!(arr.get(1).as_string().unwrap_throw(), "hello");
}

// ===========================================================================
// patch_js: unit struct
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_unit_struct() {
  let js = UnitStruct.to_js();
  let mut called = false;
  UnitStruct.patch_js(&js, |_| called = true);
  assert!(!called, "unit struct should be unchanged (both null)");
}

// ===========================================================================
// patch_js: unit enum variants
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_unit_enum_unchanged() {
  let val = UnitEnum::A;
  let js = val.to_js();
  let mut called = false;
  UnitEnum::A.patch_js(&js, |_| called = true);
  assert!(!called, "same unit enum variant should not call set");
}

#[wasm_bindgen_test]
fn test_patch_js_unit_enum_changed() {
  let old = UnitEnum::A;
  let js = old.to_js();
  let mut called = false;
  UnitEnum::B.patch_js(&js, |_| called = true);
  assert!(called, "different unit enum variant should call set");
}

// ===========================================================================
// patch_js: internally tagged unit variant unchanged
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_internal_unit_unchanged() {
  let val = InternallyTagged::Unit;
  let js = val.to_js();
  InternallyTagged::Unit.patch_js(&js, |_| panic!("same unit variant should not replace"));
}

// ===========================================================================
// patch_js: adjacently tagged unit variant unchanged
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_adjacent_unit_unchanged() {
  let val = AdjacentlyTagged::Unit;
  let js = val.to_js();
  AdjacentlyTagged::Unit.patch_js(&js, |_| panic!("same unit variant should not replace"));
}

// ===========================================================================
// patch_js: externally tagged struct variant — unchanged
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_external_struct_variant_unchanged() {
  let val = MixedExternalEnum::Struct { x: 10, y: 20 };
  let js = val.to_js();
  MixedExternalEnum::Struct { x: 10, y: 20 }
    .patch_js(&js, |_| panic!("unchanged should not replace"));
}

// ===========================================================================
// patch_js: internally tagged named variant — all fields unchanged
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_internal_named_unchanged() {
  let val = InternallyTagged::Named { field_one: 42, field_two: "hello".into() };
  let js = val.to_js();
  val.patch_js(&js, |_| panic!("unchanged should not replace"));
  // Verify object identity is same
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("fieldOne"))
      .unwrap_throw()
      .as_f64()
      .unwrap_throw(),
    42.0
  );
}

// ===========================================================================
// patch_js: enum with #[serde(rename)] on variant
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_variant_rename_unchanged() {
  let val = VariantRename::Original;
  let js = val.to_js();
  let mut called = false;
  VariantRename::Original.patch_js(&js, |_| called = true);
  assert!(!called, "same renamed variant should not call set");
}

// ===========================================================================
// patch_js: per-variant untagged
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_per_variant_untagged_same_variant() {
  let old = PerVariantUntagged::Tagged { value: 1 };
  let js = old.to_js();
  PerVariantUntagged::Tagged { value: 2 }.patch_js(&js, |_| panic!("should patch in place"));
  assert_eq!(
    js_sys::Reflect::get(&js, &JsValue::from_str("value")).unwrap_throw().as_f64().unwrap_throw(),
    2.0
  );
}

#[wasm_bindgen_test]
fn test_patch_js_per_variant_untagged_variant_change() {
  let old = PerVariantUntagged::Tagged { value: 1 };
  let js = old.to_js();
  let mut replaced = false;
  PerVariantUntagged::Fallback("hello".into()).patch_js(&js, |_| replaced = true);
  assert!(replaced, "variant change should replace");
}

// ===========================================================================
// patch_js: Unit type
// ===========================================================================

#[wasm_bindgen_test]
fn test_patch_js_unit_noop() {
  // patch_js on () should never call set, regardless of old value
  let mut called = false;
  ().patch_js(&JsValue::NULL, |_| called = true);
  assert!(!called, "unit patch_js should be no-op on null");

  ().patch_js(&JsValue::from_f64(42.0), |_| called = true);
  assert!(!called, "unit patch_js should be no-op on number");

  ().patch_js(&JsValue::from_str("anything"), |_| called = true);
  assert!(!called, "unit patch_js should be no-op on string");
}
