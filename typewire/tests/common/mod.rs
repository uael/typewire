#![allow(dead_code)]

use typewire::Typewire;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Round-trip: Rust → JS → Rust.
pub fn round_trip<T: Typewire + PartialEq + std::fmt::Debug>(val: &T) {
  let js = val.to_js();
  let back = T::from_js(js).unwrap_throw();
  assert_eq!(*val, back);
}

/// Evaluate a JS expression and return the JsValue.
pub fn eval(code: &str) -> JsValue {
  js_sys::eval(code).unwrap_throw()
}

// ===========================================================================
// Derive: structs
// ===========================================================================

#[derive(Debug, PartialEq, Typewire)]
pub struct BasicStruct {
  pub name: String,
  pub age: u32,
}

#[derive(Debug, PartialEq, Typewire)]
#[serde(rename_all = "camelCase")]
pub struct CamelCaseStruct {
  pub first_name: String,
  pub last_name: String,
  pub is_active: bool,
}

#[derive(Debug, PartialEq, Typewire)]
pub struct RenameFieldStruct {
  #[serde(rename = "type")]
  pub kind: String,
  pub value: u32,
}

#[derive(Debug, PartialEq, Typewire)]
pub struct SkipFieldStruct {
  pub visible: String,
  #[serde(skip)]
  pub hidden: u32,
}

#[derive(Debug, PartialEq, Typewire)]
pub struct SkipSerStruct {
  pub name: String,
  #[serde(skip_serializing)]
  pub write_only: u32,
}

#[derive(Debug, PartialEq, Typewire)]
pub struct SkipDeStruct {
  pub name: String,
  #[serde(skip_deserializing)]
  pub read_only: u32,
}

#[derive(Debug, PartialEq, Typewire)]
pub struct DefaultFieldStruct {
  pub name: String,
  #[serde(default)]
  pub count: u32,
}

pub fn default_count() -> u32 {
  99
}

#[derive(Debug, PartialEq, Typewire)]
pub struct DefaultPathStruct {
  pub name: String,
  #[serde(default = "default_count")]
  pub count: u32,
}

#[derive(Debug, PartialEq, Typewire)]
#[serde(default)]
pub struct ContainerDefaultStruct {
  pub name: String,
  pub count: u32,
}

impl Default for ContainerDefaultStruct {
  fn default() -> Self {
    Self { name: "default_name".into(), count: 42 }
  }
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct Inner {
  pub x: u32,
  pub y: u32,
}

#[derive(Debug, PartialEq, Typewire)]
pub struct FlattenStruct {
  pub name: String,
  #[serde(flatten)]
  pub inner: Inner,
}

pub fn is_zero(v: &u32) -> bool {
  *v == 0
}

#[derive(Debug, PartialEq, Typewire)]
pub struct SkipSerIfStruct {
  pub name: String,
  #[serde(skip_serializing_if = "is_zero")]
  pub count: u32,
}

#[derive(Debug, PartialEq, Typewire)]
pub struct AliasStruct {
  #[serde(alias = "userName", alias = "user")]
  pub name: String,
}

#[derive(Debug, PartialEq, Typewire)]
#[serde(deny_unknown_fields)]
pub struct StrictStruct {
  pub name: String,
}

#[derive(Debug, PartialEq, Typewire)]
#[serde(transparent)]
pub struct TransparentNamed {
  pub inner: u32,
}

#[derive(Debug, PartialEq, Typewire)]
#[serde(transparent)]
pub struct TransparentTuple(pub String);

#[derive(Debug, PartialEq, Typewire)]
pub struct TupleStruct(pub u32, pub String);

#[derive(Debug, PartialEq, Typewire)]
pub struct UnitStruct;

#[derive(Debug, PartialEq, Typewire)]
pub struct GenericStruct<T: Clone> {
  pub value: T,
}

// ===========================================================================
// Derive: enums — externally tagged (default)
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub enum UnitEnum {
  A,
  B,
  C,
}

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(rename_all = "snake_case")]
pub enum RenameAllEnum {
  FirstVariant,
  SecondVariant,
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub enum MixedExternalEnum {
  Unit,
  Newtype(u32),
  Tuple(u32, String),
  Struct { x: u32, y: u32 },
}

// ===========================================================================
// Derive: enums — internally tagged
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(tag = "type")]
pub enum InternallyTagged {
  Unit,
  #[serde(rename_all = "camelCase")]
  Named {
    field_one: u32,
    field_two: String,
  },
  Newtype(Inner),
}

// ===========================================================================
// Derive: enums — adjacently tagged
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(tag = "t", content = "c")]
pub enum AdjacentlyTagged {
  Unit,
  Single(u32),
  Multi(u32, String),
  Named { a: u32, b: String },
}

// ===========================================================================
// Derive: enums — untagged
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(untagged)]
pub enum Untagged {
  Num(u32),
  Text(String),
  Pair { x: u32, y: u32 },
}

// ===========================================================================
// Derive: enum variant attributes
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub enum VariantRename {
  #[serde(rename = "custom_name")]
  Original,
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub enum VariantAlias {
  #[serde(alias = "legacy", alias = "old")]
  Current,
}

#[derive(Debug, PartialEq, Clone, Typewire)]
#[allow(dead_code)]
pub enum VariantSkip {
  Visible,
  #[serde(skip)]
  Hidden,
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub enum VariantOther {
  Known,
  #[serde(other)]
  Unknown,
}

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(tag = "type")]
pub enum PerVariantUntagged {
  Tagged {
    value: u32,
  },
  #[serde(untagged)]
  Fallback(String),
}

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(rename_all_fields = "camelCase")]
pub enum RenameAllFields {
  A { field_name: u32 },
  B { other_field: String },
}

// ===========================================================================
// Derive: proxy types (from / try_from / into)
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct ProxyInner(pub u32);

#[derive(Debug, PartialEq, Clone)]
pub struct FromProxy {
  pub value: u32,
}

impl From<ProxyInner> for FromProxy {
  fn from(p: ProxyInner) -> Self {
    Self { value: p.0 }
  }
}

impl From<FromProxy> for ProxyInner {
  fn from(f: FromProxy) -> Self {
    Self(f.value)
  }
}

// Note: from+into together
#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(from = "ProxyInner", into = "ProxyInner")]
pub struct FromIntoProxy {
  pub value: u32,
}

impl From<ProxyInner> for FromIntoProxy {
  fn from(p: ProxyInner) -> Self {
    Self { value: p.0 }
  }
}

impl From<FromIntoProxy> for ProxyInner {
  fn from(f: FromIntoProxy) -> Self {
    Self(f.value)
  }
}

// --- try_from + into (fallible proxy, the most common pattern in photogram) ---

/// Validates the inner string is non-empty (mirrors photogram's `NonEmptyString`).
#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(try_from = "String", into = "String")]
pub struct ValidatedString(pub String);

impl TryFrom<String> for ValidatedString {
  type Error = &'static str;
  fn try_from(s: String) -> Result<Self, Self::Error> {
    if s.is_empty() { Err("string must not be empty") } else { Ok(Self(s)) }
  }
}

impl From<ValidatedString> for String {
  fn from(v: ValidatedString) -> Self {
    v.0
  }
}

// --- try_from alone (to_js uses struct codegen, from_js uses fallible proxy) ---

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(try_from = "u32")]
pub struct BoundedU32 {
  pub value: u32,
}

impl TryFrom<u32> for BoundedU32 {
  type Error = &'static str;
  fn try_from(n: u32) -> Result<Self, Self::Error> {
    if n > 100 { Err("must be <= 100") } else { Ok(Self { value: n }) }
  }
}

// ===========================================================================
// Derive: implicit Option default (or_default)
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct OptionalFields {
  pub required: String,
  pub optional: Option<u32>,
  pub also_optional: Option<String>,
}

// ===========================================================================
// Derive: error context
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct ContextInner {
  pub port: u32,
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct ContextOuter {
  pub name: String,
  pub inner: ContextInner,
}

// ===========================================================================
// patch_js: diffable(visit_transparent) — delegates to inner
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(transparent)]
#[diffable(visit_transparent)]
pub struct TransparentWrapper(pub String);

// ===========================================================================
// patch_js: diffable(atomic) — whole value replaced, no field-level patch
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
#[diffable(atomic)]
pub struct AtomicStruct {
  pub x: u32,
  pub y: u32,
}

// ===========================================================================
// patch_js: deeply nested (3 levels)
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct Level3 {
  pub value: u32,
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct Level2 {
  pub level3: Level3,
  pub tag: String,
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct Level1 {
  pub level2: Level2,
  pub name: String,
}

// ===========================================================================
// patch_js: LCS vec
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct LcsVecStruct {
  pub items: Vec<u32>,
}

/// Helper: get the "items" JS array from a LcsVecStruct JsValue.
pub fn get_items_arr(js: &JsValue) -> js_sys::Array {
  js_sys::Reflect::get(js, &JsValue::from_str("items")).unwrap_throw().into()
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct LcsItem {
  pub id: u32,
  pub label: String,
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct LcsStructVec {
  pub items: Vec<LcsItem>,
}

// ===========================================================================
// #[serde(with = "serde_bytes")] field
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct SerdeBytesStruct {
  #[serde(with = "serde_bytes")]
  pub data: Vec<u8>,
}

// ===========================================================================
// #[typewire(base64)] field
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct Base64Struct {
  #[typewire(base64)]
  pub data: Vec<u8>,
}

// ===========================================================================
// #[typewire(display)] field
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Copy, Typewire)]
#[serde(transparent)]
pub struct MyId(pub u64);

impl std::fmt::Display for MyId {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.0)
  }
}

impl std::str::FromStr for MyId {
  type Err = std::num::ParseIntError;
  fn from_str(s: &str) -> Result<Self, Self::Err> {
    Ok(Self(s.parse()?))
  }
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct DisplayStruct {
  #[typewire(display)]
  pub id: MyId,
}

// ===========================================================================
// Error context: deep nesting
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct DeepErrorOuter {
  pub middle: DeepErrorMiddle,
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct DeepErrorMiddle {
  pub inner: ContextInner,
}

// ===========================================================================
// Externally tagged enum with extra keys (internally tagged parent)
// ===========================================================================

/// Simulates ThumbnailResult/GenerateResult pattern: an internally tagged
/// wrapper around an externally tagged inner enum.
#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum OuterTagged {
  Inner(InnerExternalEnum),
}

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(rename_all = "camelCase")]
pub enum InnerExternalEnum {
  Ok {},
  #[serde(rename_all = "camelCase")]
  Error {
    message: String,
  },
}

// ===========================================================================
// patch_js: flatten struct
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct FlattenPatchStruct {
  pub name: String,
  #[serde(flatten)]
  pub inner: Inner,
}

// ===========================================================================
// Lenient: proxy types (try_from)
// ===========================================================================

/// A u32 constrained to [0, 100], used for lenient proxy tests.
#[derive(Debug, PartialEq, Clone)]
pub struct Clamped100(pub u32);

impl From<Clamped100> for u32 {
  fn from(b: Clamped100) -> Self {
    b.0
  }
}

impl TryFrom<u32> for Clamped100 {
  type Error = String;
  fn try_from(v: u32) -> Result<Self, Self::Error> {
    if v <= 100 { Ok(Self(v)) } else { Err(format!("{v} is out of range [0, 100]")) }
  }
}

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(try_from = "u32", into = "u32")]
pub struct Clamped100Wire(#[serde(skip)] pub Clamped100);

impl TryFrom<u32> for Clamped100Wire {
  type Error = String;
  fn try_from(v: u32) -> Result<Self, Self::Error> {
    Clamped100::try_from(v).map(Self)
  }
}

impl From<Clamped100Wire> for u32 {
  fn from(b: Clamped100Wire) -> Self {
    b.0.0
  }
}

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct LenientBoundedStruct {
  #[typewire(lenient)]
  pub scores: Vec<Clamped100Wire>,
}

// ===========================================================================
// Untagged enum: ambiguous variants
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
#[serde(untagged)]
pub enum AmbiguousUntagged {
  /// Tried first — matches any u32
  Count(u32),
  /// Tried second — also wants u32 but won't match because Count wins
  Amount(u32),
  /// Fallback for everything else
  Label(String),
}

// ===========================================================================
// Option<Vec<T>> for patch_js identity tests
// ===========================================================================

#[derive(Debug, PartialEq, Clone, Typewire)]
pub struct OptVecStruct {
  pub items: Option<Vec<u32>>,
}
