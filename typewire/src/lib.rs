mod error;

pub use error::Error;
#[cfg(feature = "derive")]
pub use typewire_derive::Typewire;
pub use typewire_schema as schema;

/// Base64-encode bytes to a string. Used by `#[typewire(base64)]`.
#[cfg(feature = "base64")]
#[must_use]
pub fn base64_encode(bytes: &[u8]) -> String {
  use base64::Engine as _;
  base64::engine::general_purpose::STANDARD.encode(bytes)
}

/// Base64-decode a string to bytes. Used by `#[typewire(base64)]`.
///
/// # Errors
///
/// Returns an error if the input is not valid base64.
#[cfg(feature = "base64")]
pub fn base64_decode(s: &str) -> Result<Vec<u8>, base64::DecodeError> {
  use base64::Engine as _;
  base64::engine::general_purpose::STANDARD.decode(s)
}

/// Bidirectional conversion between Rust types and JavaScript values.
pub trait Typewire: Sized {
  type Ident: Copy + 'static;
  const IDENT: Self::Ident;

  /// Returns the implicit default for this type when a field is absent.
  ///
  /// Most types return `None` (no default — the field is required).
  /// `Option<T>` returns `Some(None)`, making optional fields implicitly
  /// default to `None` without requiring `#[serde(default)]`.
  #[must_use]
  fn or_default() -> Option<Self> {
    None
  }

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue;

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error>;

  /// Lenient variant of `from_js` used by `#[typewire(lenient)]` fields.
  ///
  /// The default delegates to `from_js`. Collection types (`Vec`, `Option`,
  /// maps) override this to skip invalid elements / default to `None`
  /// instead of propagating errors, logging warnings for each skip.
  #[cfg(target_arch = "wasm32")]
  fn from_js_lenient(value: wasm_bindgen::JsValue, _field: &str) -> Result<Self, Error> {
    Self::from_js(value)
  }

  /// Patches an existing JS value in place.
  ///
  /// Compares `self` (the new value) against `old` (the existing JS value).
  /// If they differ, calls `set` with the new JS representation.
  ///
  /// Structs override this to recurse into fields, preserving JS object
  /// identity. There is no default — every type must either derive or
  /// manually implement `patch_js`.
  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue));
}

/// Atomic `patch_js`: deserializes `old` via `from_js`, compares with
/// `PartialEq`, and calls `set(new.to_js())` only if different.
///
/// Used by `#[diffable(atomic)]` types and by the derive for tuple structs,
/// unit structs, and enums with only unit variants.
#[cfg(target_arch = "wasm32")]
pub fn patch_js_atomic<T: Typewire + PartialEq>(
  new: &T,
  old: &wasm_bindgen::JsValue,
  set: impl FnOnce(wasm_bindgen::JsValue),
) {
  match T::from_js(old.clone()) {
    Ok(ref old_val) if new == old_val => {}
    _ => set(new.to_js()),
  }
}

// ---------------------------------------------------------------------------
// Link section statics for built-in types
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Primitive implementations (only compiled on wasm32)
// ---------------------------------------------------------------------------

#[cfg(target_arch = "wasm32")]
mod wasm {
  use wasm_bindgen::JsValue;

  /// Returns `true` if the value is `null` or `undefined`.
  pub(crate) fn is_nullish(v: &JsValue) -> bool {
    v.is_null() || v.is_undefined()
  }

  /// Extract an `f64` from a JS number value.
  pub(crate) fn as_safe_f64(v: &JsValue) -> Option<f64> {
    v.as_f64()
  }
}

#[cfg(target_arch = "wasm32")]
impl Typewire for wasm_bindgen::JsValue {
  type Ident = schema::coded::Ident<3>;
  const IDENT: Self::Ident = schema::coded::Ident::new(*b"any");

  fn to_js(&self) -> wasm_bindgen::JsValue {
    self.clone()
  }

  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    Ok(value)
  }

  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    if old != self {
      set(self.clone());
    }
  }
}

impl Typewire for bool {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::bool);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_bool(*self)
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    value.as_bool().ok_or(Error::UnexpectedType { expected: "boolean" })
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

macro_rules! impl_typewire_small_int {
    ($($ty:ident),*) => {$(
        impl Typewire for $ty {
            type Ident = schema::coded::PrimitiveIdent;
            const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(
                schema::Scalar::$ty,
            );

            #[cfg(target_arch = "wasm32")]
            fn to_js(&self) -> wasm_bindgen::JsValue {
                wasm_bindgen::JsValue::from_f64(f64::from(*self))
            }

            #[cfg(target_arch = "wasm32")]
            fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
                let n = wasm::as_safe_f64(&value)
                    .ok_or(Error::UnexpectedType { expected: "number" })?;
                #[expect(
                    clippy::cast_possible_truncation,
                    reason = "checked by range comparison below"
                )]
                let v = n as $ty;
                if f64::from(v) == n {
                    Ok(v)
                } else {
                    Err(Error::OutOfRange)
                }
            }

            #[cfg(target_arch = "wasm32")]
            fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
                patch_js_atomic(self, old, set);
            }
        }
    )*};
}

impl_typewire_small_int!(u8, u16, u32, i8, i16, i32);

impl Typewire for f32 {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::f32);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_f64(f64::from(*self))
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    let n = wasm::as_safe_f64(&value).ok_or(Error::UnexpectedType { expected: "number" })?;
    #[expect(clippy::cast_possible_truncation, reason = "f64 → f32 narrowing is intentional")]
    Ok(n as f32)
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

impl Typewire for f64 {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::f64);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_f64(*self)
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    wasm::as_safe_f64(&value).ok_or(Error::UnexpectedType { expected: "number" })
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

macro_rules! impl_typewire_lossy {
    ($($ty:ident),*) => {$(
        impl Typewire for $ty {
            type Ident = schema::coded::PrimitiveIdent;
            const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(
                schema::Scalar::$ty,
            );

            #[cfg(target_arch = "wasm32")]
            fn to_js(&self) -> wasm_bindgen::JsValue {
                let number = *self as f64;
                if number as $ty != *self {
                    log::warn!("lossy conversion of {self} to JS number: {number}");
                }

                wasm_bindgen::JsValue::from(number)
            }

            #[cfg(target_arch = "wasm32")]
            fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
                use wasm_bindgen::JsCast as _;

                if let Some(number) = wasm::as_safe_f64(&value) {
                    #[expect(
                        clippy::cast_possible_truncation,
                        clippy::cast_sign_loss,
                        reason = "lossy conversion is intentional — saturates for out-of-range values"
                    )]
                    return Ok(number as $ty);
                }

                let bigint = value
                    .dyn_into::<js_sys::BigInt>()
                    .map_err(|_| Error::UnexpectedType { expected: "bigint" })?;
                <$ty>::try_from(bigint).map_err(|_| Error::OutOfRange)
            }

            #[cfg(target_arch = "wasm32")]
            fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
                patch_js_atomic(self, old, set);
            }
        }
    )*};
}

impl_typewire_lossy!(u64, i64, u128, i128);

impl Typewire for usize {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::usize);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    // On wasm32, usize is u32 — fits in f64.
    wasm_bindgen::JsValue::from_f64(*self as f64)
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    let n = wasm::as_safe_f64(&value).ok_or(Error::UnexpectedType { expected: "number" })?;
    #[expect(
      clippy::cast_possible_truncation,
      clippy::cast_sign_loss,
      reason = "on wasm32, usize == u32, validated by round-trip"
    )]
    Ok(n as usize)
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

impl Typewire for isize {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::isize);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_f64(*self as f64)
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    let n = wasm::as_safe_f64(&value).ok_or(Error::UnexpectedType { expected: "number" })?;
    #[expect(
      clippy::cast_possible_truncation,
      reason = "on wasm32, isize == i32, validated by round-trip"
    )]
    Ok(n as isize)
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

impl Typewire for char {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::char);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_str(&self.to_string())
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    let s = value.as_string().ok_or(Error::UnexpectedType { expected: "string" })?;
    let mut chars = s.chars();
    let Some(c) = chars.next() else {
      return Err(Error::InvalidValue { message: "empty string".into() });
    };
    if chars.next().is_some() {
      return Err(Error::InvalidValue { message: "expected single character".into() });
    }
    Ok(c)
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

impl Typewire for String {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::str);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_str(self)
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    value.as_string().ok_or(Error::UnexpectedType { expected: "string" })
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

impl Typewire for std::borrow::Cow<'_, str> {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::str);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_str(self)
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    value
      .as_string()
      .map(std::borrow::Cow::Owned)
      .ok_or(Error::UnexpectedType { expected: "string" })
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

impl Typewire for () {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::Unit);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::NULL
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(_value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    Ok(())
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, _old: &wasm_bindgen::JsValue, _set: impl FnOnce(wasm_bindgen::JsValue)) {
    // Unit type is a singleton — nothing to diff.
  }
}

// ---------------------------------------------------------------------------
// Compound types
// ---------------------------------------------------------------------------

impl<T: Typewire> Typewire for Option<T> {
  type Ident = schema::coded::OptionIdent<T::Ident>;
  const IDENT: Self::Ident = schema::coded::OptionIdent::new(T::IDENT);

  fn or_default() -> Option<Self> {
    Some(None)
  }

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    match self {
      Some(v) => v.to_js(),
      None => wasm_bindgen::JsValue::NULL,
    }
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    if wasm::is_nullish(&value) { Ok(None) } else { T::from_js(value).map(Some) }
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js_lenient(value: wasm_bindgen::JsValue, field: &str) -> Result<Self, Error> {
    if wasm::is_nullish(&value) {
      Ok(None)
    } else {
      match T::from_js(value) {
        Ok(v) => Ok(Some(v)),
        Err(e) => {
          log::warn!("{field}: defaulting to None: {e}");
          Ok(None)
        }
      }
    }
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    match self {
      None => {
        if !wasm::is_nullish(old) {
          set(wasm_bindgen::JsValue::NULL);
        }
      }
      Some(v) => {
        if wasm::is_nullish(old) {
          set(v.to_js());
        } else {
          v.patch_js(old, set);
        }
      }
    }
  }
}

#[cfg(target_arch = "wasm32")]
pub fn array_ref<'a, T: Typewire + 'a>(
  iter: impl IntoIterator<Item = &'a T>,
) -> wasm_bindgen::JsValue {
  array(iter.into_iter().map(|item| item.to_js()))
}

#[cfg(target_arch = "wasm32")]
pub fn array<T: Typewire>(iter: impl IntoIterator<Item = T>) -> wasm_bindgen::JsValue {
  let iter = iter.into_iter();
  let (low, high) = iter.size_hint();
  let arr;
  if Some(low) == high {
    arr = js_sys::Array::new_with_length(low as u32);
    for (i, item) in iter.enumerate() {
      arr.set(i as u32, item.to_js());
    }
  } else {
    arr = js_sys::Array::new();
    for item in iter {
      arr.push(&item.to_js());
    }
  }
  arr.into()
}

impl<T: Typewire> Typewire for Vec<T> {
  type Ident = schema::coded::SeqIdent<T::Ident>;
  const IDENT: Self::Ident = schema::coded::SeqIdent::new(T::IDENT);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    array_ref(self.iter())
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    let arr: js_sys::Array =
      value.try_into().map_err(|_| Error::UnexpectedType { expected: "array" })?;
    let mut out = Vec::with_capacity(arr.length() as usize);
    for i in 0..arr.length() {
      out.push(T::from_js(arr.get(i))?);
    }
    Ok(out)
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js_lenient(value: wasm_bindgen::JsValue, field: &str) -> Result<Self, Error> {
    use wasm_bindgen::JsCast as _;

    let Some(arr) = value.dyn_ref::<js_sys::Array>() else {
      log::warn!("{field}: expected array, skipping (got {:?})", value.js_typeof());
      return Ok(Vec::new());
    };
    let mut out = Vec::with_capacity(arr.length() as usize);
    for i in 0..arr.length() {
      match T::from_js(arr.get(i)) {
        Ok(v) => out.push(v),
        Err(e) => log::warn!("{field}[{i}]: skipping invalid element: {e}"),
      }
    }
    Ok(out)
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_slice(self.iter(), old, set);
  }
}

#[cfg(target_arch = "wasm32")]
pub fn patch_js_slice<'a, T: Typewire + 'a>(
  new: impl ExactSizeIterator<Item = &'a T> + Clone,
  old: &wasm_bindgen::JsValue,
  set: impl FnOnce(wasm_bindgen::JsValue),
) {
  #[cfg(debug_assertions)]
  let old_old = old.clone();
  if !patch_js_slice_inner(new.clone(), old, set) {
    return;
  }
  #[cfg(debug_assertions)]
  {
    let new_native = <Vec<T>>::from_js(old.clone()).unwrap();
    let new = new.map(Typewire::to_js).collect::<Vec<_>>();
    assert_eq!(new_native.len(), new.len());
    if &old_old != old {
      log::warn!("~~ patch_js_slice: target = {new:#?}, old = {old_old:#?} new = {old:#?}");
    }
  }
}

/// LCS-based slice patching with `T::patch_js` delegation.
///
/// Builds a patched `JsValue` array by calling `T::patch_js` on each element
/// positionally. Unchanged elements keep the same JS reference as the old array.
/// Then uses `similar`'s LCS algorithm on the JsValue references (`===`) to
/// compute minimal splice operations.
///
/// Does NOT require `T: PartialEq` — comparison uses JS reference identity.
#[cfg(target_arch = "wasm32")]
pub fn patch_js_slice_inner<'a, T: Typewire + 'a>(
  new: impl ExactSizeIterator<Item = &'a T> + Clone,
  old: &wasm_bindgen::JsValue,
  set: impl FnOnce(wasm_bindgen::JsValue),
) -> bool {
  use similar::algorithms::{Capture, Compact, Replace as SimilarReplace};
  use wasm_bindgen::JsCast as _;

  let Some(arr) = old.dyn_ref::<js_sys::Array>() else {
    set(array_ref(new));
    return false;
  };

  let old_len = arr.length() as usize;
  let new_len = new.len();

  // Fast path: same length
  if old_len == new_len {
    for (i, elem) in new.enumerate() {
      elem.patch_js(&arr.get(i as u32), |val| arr.set(i as u32, val));
    }
    return false;
  }

  // Collect old JS references
  let old_refs: Vec<wasm_bindgen::JsValue> = (0..old_len).map(|i| arr.get(i as u32)).collect();

  // Build patched refs: for each new element, try patch_js against the
  // positionally corresponding old element. If unchanged, the old ref is kept.
  // If changed or new, we get a fresh JsValue.
  let mut patched_refs: Vec<wasm_bindgen::JsValue> = Vec::with_capacity(new_len);
  for (i, elem) in new.clone().enumerate() {
    if i < old_len {
      let mut result = old_refs[i].clone();
      elem.patch_js(&old_refs[i], |v| result = v);
      patched_refs.push(result);
    } else {
      patched_refs.push(elem.to_js());
    }
  }

  // LCS diff on JsValue references — === compares by reference identity,
  // so unchanged elements (same ref) are matched as Equal.
  let mut d = Compact::new(SimilarReplace::new(Capture::new()), &old_refs, &patched_refs);
  similar::algorithms::lcs::diff(&mut d, &old_refs, 0..old_len, &patched_refs, 0..new_len)
    .unwrap_or(());

  let ops = d.into_inner().into_inner().into_ops();

  let mut offset: isize = 0;

  #[expect(
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    reason = "matches difficient's implementation"
  )]
  for op in ops {
    match op {
      similar::DiffOp::Equal { old_index, new_index, len } => {
        // Elements matched by reference — patch in place (already done
        // during patched_refs construction, but we need to update the
        // actual array if the position shifted due to prior splices)
        for ix in 0..len {
          let target_idx = (new_index + ix) as u32;
          let actual_idx = ((old_index + ix) as isize + offset) as u32;
          if actual_idx != target_idx {
            // Position shifted — move element
            arr.set(target_idx, patched_refs[new_index + ix].clone());
          } else {
            // Same position — the value is already patched_refs[i]
            // which may have been updated by patch_js
            arr.set(actual_idx, patched_refs[new_index + ix].clone());
          }
        }
      }
      similar::DiffOp::Delete { old_len, old_index, .. } => {
        let at = (old_index as isize + offset) as u32;
        arr.splice_many(at, old_len as u32, &[]);
        offset -= old_len as isize;
      }
      similar::DiffOp::Insert { new_index, new_len, .. } => {
        arr.splice_many(new_index as u32, 0, &patched_refs[new_index..new_index + new_len]);
        offset += new_len as isize;
      }
      similar::DiffOp::Replace { old_index, old_len, new_index, new_len, .. } => {
        let at = (old_index as isize + offset) as u32;
        arr.splice_many(at, old_len as u32, &patched_refs[new_index..new_index + new_len]);
        offset -= old_len as isize;
        offset += new_len as isize;
      }
    }
  }

  true
}

impl<T: Typewire> Typewire for Box<T> {
  type Ident = T::Ident;
  const IDENT: Self::Ident = T::IDENT;

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    (**self).to_js()
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    T::from_js(value).map(Box::new)
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    (**self).patch_js(old, set);
  }
}

impl<T: Typewire, const N: usize> Typewire for [T; N] {
  type Ident = schema::coded::SeqIdent<T::Ident>;
  const IDENT: Self::Ident = schema::coded::SeqIdent::new(T::IDENT);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    let arr = js_sys::Array::new_with_length(N as u32);
    for (i, item) in self.iter().enumerate() {
      arr.set(i as u32, item.to_js());
    }
    arr.into()
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    let arr: js_sys::Array =
      value.try_into().map_err(|_| Error::UnexpectedType { expected: "array" })?;
    if arr.length() as usize != N {
      return Err(Error::InvalidValue {
        message: format!("expected array of length {N}, got {}", arr.length()),
      });
    }
    // Use MaybeUninit to build the array element-by-element.
    let mut out: [std::mem::MaybeUninit<T>; N] =
      unsafe { std::mem::MaybeUninit::uninit().assume_init() };
    for (i, slot) in out.iter_mut().enumerate() {
      match T::from_js(arr.get(i as u32)) {
        Ok(v) => {
          slot.write(v);
        }
        Err(e) => {
          // Drop already-initialized elements before returning.
          for already in &mut out[..i] {
            unsafe { already.assume_init_drop() };
          }
          return Err(e);
        }
      }
    }
    // SAFETY: all elements have been initialized.
    Ok(unsafe { std::mem::transmute_copy::<_, [T; N]>(&out) })
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    use wasm_bindgen::JsCast as _;
    let Some(arr) = old.dyn_ref::<js_sys::Array>() else {
      set(self.to_js());
      return;
    };
    for i in 0..N {
      let idx = i as u32;
      self[i].patch_js(&arr.get(idx), |v| arr.set(idx, v));
    }
  }
}

// --- Maps → JS objects ---

/// Patches a JS object in place by iterating new key-value entries, recursing
/// into each value's `patch_js`, and deleting keys that are no longer present.
///
/// Used by `HashMap`, `BTreeMap`, and `serde_json::Value::Object`.
#[cfg(target_arch = "wasm32")]
pub fn patch_js_map<'a, V: Typewire + 'a>(
  entries: impl IntoIterator<Item = (wasm_bindgen::JsValue, &'a V)>,
  contains_key: impl Fn(&wasm_bindgen::JsValue) -> bool,
  to_js: impl FnOnce() -> wasm_bindgen::JsValue,
  old: &wasm_bindgen::JsValue,
  set: impl FnOnce(wasm_bindgen::JsValue),
) {
  use wasm_bindgen::JsCast as _;

  let Some(old_obj) = old.dyn_ref::<js_sys::Object>() else {
    set(to_js());
    return;
  };
  // Arrays are objects in JS — don't patch an array as a keyed object
  if js_sys::Array::is_array(old) {
    set(to_js());
    return;
  }

  // Snapshot old keys before patching — avoids iterating newly-added keys
  // during the delete pass below.
  let old_keys = js_sys::Object::keys(old_obj);

  // Patch existing and new keys
  for (k_js, v) in entries {
    let old_v = js_sys::Reflect::get(old, &k_js).unwrap_or(wasm_bindgen::JsValue::UNDEFINED);
    v.patch_js(&old_v, |new_v| {
      let _ = js_sys::Reflect::set(old, &k_js, &new_v);
    });
  }

  // Delete keys present in old but not in new
  for i in 0..old_keys.length() {
    let key = old_keys.get(i);
    if !contains_key(&key) {
      let _ = js_sys::Reflect::delete_property(old_obj, &key);
    }
  }
}

impl<K: Typewire + Eq + core::hash::Hash, V: Typewire, S: ::std::hash::BuildHasher + Default>
  Typewire for std::collections::HashMap<K, V, S>
{
  type Ident = schema::coded::MapIdent<K::Ident, V::Ident>;
  const IDENT: Self::Ident = schema::coded::MapIdent::new(K::IDENT, V::IDENT);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    let obj = js_sys::Object::new();
    for (k, v) in self {
      let _ = js_sys::Reflect::set(&obj, &k.to_js(), &v.to_js());
    }
    obj.into()
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    use wasm_bindgen::JsCast as _;
    let entries = js_sys::Object::entries(
      value.dyn_ref::<js_sys::Object>().ok_or(Error::UnexpectedType { expected: "object" })?,
    );
    let mut map = std::collections::HashMap::default();
    map.reserve(entries.length() as usize);
    for i in 0..entries.length() {
      let pair: js_sys::Array = entries.get(i).into();
      let key = K::from_js(pair.get(0))?;
      let val = V::from_js(pair.get(1))?;
      map.insert(key, val);
    }
    Ok(map)
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js_lenient(value: wasm_bindgen::JsValue, field: &str) -> Result<Self, Error> {
    use wasm_bindgen::JsCast as _;
    let Some(obj) = value.dyn_ref::<js_sys::Object>() else {
      log::warn!("{field}: expected object, skipping");
      return Ok(Self::default());
    };
    let entries = js_sys::Object::entries(obj);
    let mut map = std::collections::HashMap::default();
    map.reserve(entries.length() as usize);
    for i in 0..entries.length() {
      let pair: js_sys::Array = entries.get(i).into();
      match (K::from_js(pair.get(0)), V::from_js(pair.get(1))) {
        (Ok(k), Ok(v)) => {
          map.insert(k, v);
        }
        (Err(e), _) | (_, Err(e)) => {
          log::warn!("{field}: skipping entry {i}: {e}");
        }
      }
    }
    Ok(map)
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_map(
      self.iter().map(|(k, v)| (k.to_js(), v)),
      |js_key| K::from_js(js_key.clone()).ok().is_some_and(|k| self.contains_key(&k)),
      || self.to_js(),
      old,
      set,
    );
  }
}

impl<K: Typewire + Ord, V: Typewire> Typewire for std::collections::BTreeMap<K, V> {
  type Ident = schema::coded::MapIdent<K::Ident, V::Ident>;
  const IDENT: Self::Ident = schema::coded::MapIdent::new(K::IDENT, V::IDENT);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    let obj = js_sys::Object::new();
    for (k, v) in self {
      let _ = js_sys::Reflect::set(&obj, &k.to_js(), &v.to_js());
    }
    obj.into()
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    use wasm_bindgen::JsCast as _;
    let entries = js_sys::Object::entries(
      value.dyn_ref::<js_sys::Object>().ok_or(Error::UnexpectedType { expected: "object" })?,
    );
    let mut map = std::collections::BTreeMap::new();
    for i in 0..entries.length() {
      let pair: js_sys::Array =
        entries.get(i).dyn_into().map_err(|_| Error::UnexpectedType { expected: "array" })?;
      let key = K::from_js(pair.get(0))?;
      let val = V::from_js(pair.get(1))?;
      map.insert(key, val);
    }
    Ok(map)
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js_lenient(value: wasm_bindgen::JsValue, field: &str) -> Result<Self, Error> {
    use wasm_bindgen::JsCast as _;
    let Some(obj) = value.dyn_ref::<js_sys::Object>() else {
      log::warn!("{field}: expected object, skipping");
      return Ok(Self::default());
    };
    let entries = js_sys::Object::entries(obj);
    let mut map = std::collections::BTreeMap::new();
    for i in 0..entries.length() {
      let pair: js_sys::Array = match entries.get(i).dyn_into() {
        Ok(a) => a,
        Err(_) => {
          log::warn!("{field}: skipping entry {i}: not an array");
          continue;
        }
      };
      match (K::from_js(pair.get(0)), V::from_js(pair.get(1))) {
        (Ok(k), Ok(v)) => {
          map.insert(k, v);
        }
        (Err(e), _) | (_, Err(e)) => {
          log::warn!("{field}: skipping entry {i}: {e}");
        }
      }
    }
    Ok(map)
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_map(
      self.iter().map(|(k, v)| (k.to_js(), v)),
      |js_key| K::from_js(js_key.clone()).ok().is_some_and(|k| self.contains_key(&k)),
      || self.to_js(),
      old,
      set,
    );
  }
}

// --- Tuples ---

macro_rules! impl_typewire_tuple {
    ($n:literal, $types:ident; $($idx:tt : $T:ident),+) => {
        impl<$($T: Typewire),+> Typewire for ($($T,)+) {
            type Ident = schema::coded::TupleIdent<
                schema::coded::$types<$($T::Ident),+>>;
            const IDENT: Self::Ident = schema::coded::TupleIdent::new(
                $n, schema::coded::$types($($T::IDENT),+));

            #[cfg(target_arch = "wasm32")]
            fn to_js(&self) -> wasm_bindgen::JsValue {
                let arr = js_sys::Array::new();
                $(arr.push(&self.$idx.to_js());)+
                arr.into()
            }

            #[cfg(target_arch = "wasm32")]
            fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
                let arr: js_sys::Array = value
                    .try_into()
                    .map_err(|_| Error::UnexpectedType { expected: "array" })?;
                Ok(($($T::from_js(arr.get($idx))?,)+))
            }

            #[cfg(target_arch = "wasm32")]
            fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
                use wasm_bindgen::JsCast as _;
                let Some(arr) = old.dyn_ref::<js_sys::Array>() else {
                    set(self.to_js());
                    return;
                };
                $(self.$idx.patch_js(&arr.get($idx), |v| arr.set($idx, v));)+
            }
        }
    };
}

impl_typewire_tuple!(1, Types1; 0: A);
impl_typewire_tuple!(2, Types2; 0: A, 1: B);
impl_typewire_tuple!(3, Types3; 0: A, 1: B, 2: C);
impl_typewire_tuple!(4, Types4; 0: A, 1: B, 2: C, 3: D);
impl_typewire_tuple!(5, Types5; 0: A, 1: B, 2: C, 3: D, 4: E);
impl_typewire_tuple!(6, Types6; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F);
impl_typewire_tuple!(7, Types7; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G);
impl_typewire_tuple!(8, Types8; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H);
impl_typewire_tuple!(9, Types9; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I);
impl_typewire_tuple!(10, Types10; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J);
impl_typewire_tuple!(11, Types11; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K);
impl_typewire_tuple!(12, Types12; 0: A, 1: B, 2: C, 3: D, 4: E, 5: F, 6: G, 7: H, 8: I, 9: J, 10: K, 11: L);

// ---------------------------------------------------------------------------
// Feature-gated implementations
// ---------------------------------------------------------------------------

#[cfg(feature = "uuid")]
impl Typewire for uuid::Uuid {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::Uuid);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_str(&self.to_string())
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    let s = value.as_string().ok_or(Error::UnexpectedType { expected: "string" })?;
    uuid::Uuid::try_parse(&s).map_err(|e| Error::InvalidValue { message: e.to_string() })
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

#[cfg(feature = "fractional_index")]
impl Typewire for fractional_index::FractionalIndex {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::FractionalIndex);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_str(&self.to_string())
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    let s = value.as_string().ok_or(Error::UnexpectedType { expected: "string" })?;
    fractional_index::FractionalIndex::from_string(&s)
      .map_err(|e| Error::InvalidValue { message: e.to_string() })
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

#[cfg(feature = "chrono")]
impl<Tz: chrono::TimeZone> Typewire for chrono::DateTime<Tz>
where
  Tz::Offset: core::fmt::Display,
  Self: From<chrono::DateTime<chrono::FixedOffset>>,
{
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::DateTime);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_str(&self.to_rfc3339())
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    let s = value.as_string().ok_or(Error::UnexpectedType { expected: "string" })?;
    chrono::DateTime::parse_from_rfc3339(&s)
      .map(Into::into)
      .map_err(|e| Error::InvalidValue { message: e.to_string() })
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

#[cfg(feature = "url")]
impl Typewire for url::Url {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::Url);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    wasm_bindgen::JsValue::from_str(self.as_str())
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    let s = value.as_string().ok_or(Error::UnexpectedType { expected: "string" })?;
    url::Url::parse(&s).map_err(|e| Error::InvalidValue { message: e.to_string() })
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

#[cfg(feature = "serde_json")]
impl Typewire for serde_json::Value {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::SerdeJsonValue);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    match self {
      serde_json::Value::Null => wasm_bindgen::JsValue::NULL,
      serde_json::Value::Bool(b) => wasm_bindgen::JsValue::from_bool(*b),
      serde_json::Value::Number(n) => {
        wasm_bindgen::JsValue::from_f64(n.as_f64().unwrap_or(f64::NAN))
      }
      serde_json::Value::String(s) => wasm_bindgen::JsValue::from_str(s),
      serde_json::Value::Array(arr) => array_ref(arr.iter()),
      serde_json::Value::Object(map) => {
        let obj = js_sys::Object::new();
        for (k, v) in map {
          let _ = js_sys::Reflect::set(&obj, &wasm_bindgen::JsValue::from_str(k), &v.to_js());
        }
        obj.into()
      }
    }
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    use wasm_bindgen::JsCast as _;

    if value.is_null() || value.is_undefined() {
      Ok(serde_json::Value::Null)
    } else if let Some(b) = value.as_bool() {
      Ok(serde_json::Value::Bool(b))
    } else if let Some(n) = value.as_f64() {
      // JS only has f64 — recover integer representation for whole numbers
      // so that round-tripping preserves serde_json's i64/u64 distinction.
      if n.fract() == 0.0 {
        #[expect(clippy::cast_possible_truncation)]
        let i = n as i64;
        if i as f64 == n {
          return Ok(serde_json::Value::Number(i.into()));
        }
        #[expect(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let u = n as u64;
        if u as f64 == n {
          return Ok(serde_json::Value::Number(u.into()));
        }
      }
      serde_json::Number::from_f64(n)
        .map(serde_json::Value::Number)
        .ok_or(Error::InvalidValue { message: "invalid JSON number (NaN or Infinity)".into() })
    } else if let Some(s) = value.as_string() {
      Ok(serde_json::Value::String(s))
    } else if js_sys::Array::is_array(&value) {
      let arr = js_sys::Array::from(&value);
      let mut vec = Vec::with_capacity(arr.length() as usize);
      for i in 0..arr.length() {
        vec.push(Self::from_js(arr.get(i))?);
      }
      Ok(serde_json::Value::Array(vec))
    } else if let Some(obj) = value.dyn_ref::<js_sys::Object>() {
      let entries = js_sys::Object::entries(obj);
      let mut map = serde_json::Map::with_capacity(entries.length() as usize);
      for i in 0..entries.length() {
        let pair = js_sys::Array::from(&entries.get(i));
        let key = pair.get(0).as_string().ok_or(Error::UnexpectedType { expected: "string" })?;
        let val = Self::from_js(pair.get(1))?;
        map.insert(key, val);
      }
      Ok(serde_json::Value::Object(map))
    } else {
      Err(Error::UnexpectedType { expected: "JSON value" })
    }
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    // Type-erase `set` to break monomorphization recursion. serde_json::Value
    // is recursive (Array/Object contain Value), and each `impl FnOnce` closure
    // is a unique type. Without erasure the compiler generates an infinite chain
    // of distinct instantiations.
    let mut set = Some(set);
    patch_js_json_value(self, old, &mut |v| (set.take().unwrap())(v));
  }
}

/// Inner helper for `serde_json::Value::patch_js`. Takes a `&mut dyn FnMut`
/// callback so all recursive calls through `patch_js_slice`/`patch_js_map`
/// share the same concrete monomorphization without a heap allocation.
#[cfg(all(feature = "serde_json", target_arch = "wasm32"))]
fn patch_js_json_value(
  value: &serde_json::Value,
  old: &wasm_bindgen::JsValue,
  set: &mut dyn FnMut(wasm_bindgen::JsValue),
) {
  match value {
    serde_json::Value::Null => {
      if !old.is_null() && !old.is_undefined() {
        set(wasm_bindgen::JsValue::NULL);
      }
    }
    serde_json::Value::Bool(b) => {
      if old.as_bool() != Some(*b) {
        set(wasm_bindgen::JsValue::from_bool(*b));
      }
    }
    serde_json::Value::Number(n) => {
      let f = n.as_f64().unwrap_or(f64::NAN);
      if old.as_f64() != Some(f) {
        set(wasm_bindgen::JsValue::from_f64(f));
      }
    }
    serde_json::Value::String(s) => {
      if old.as_string().as_deref() != Some(s.as_str()) {
        set(wasm_bindgen::JsValue::from_str(s));
      }
    }
    serde_json::Value::Array(arr) => {
      patch_js_slice(arr.iter(), old, set);
    }
    serde_json::Value::Object(map) => {
      patch_js_map(
        map.iter().map(|(k, v)| (wasm_bindgen::JsValue::from_str(k), v)),
        |js_key| js_key.as_string().is_some_and(|k| map.contains_key(&k)),
        || value.to_js(),
        old,
        set,
      );
    }
  }
}

#[cfg(feature = "bytes")]
impl Typewire for bytes::Bytes {
  type Ident = schema::coded::PrimitiveIdent;
  const IDENT: Self::Ident = schema::coded::PrimitiveIdent::new(schema::Scalar::Bytes);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    js_sys::Uint8ClampedArray::new_from_slice(self).into()
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    use wasm_bindgen::JsCast as _;
    if let Some(arr) = value.dyn_ref::<js_sys::Uint8ClampedArray>() {
      Ok(Self::from(arr.to_vec()))
    } else if let Some(arr) = value.dyn_ref::<js_sys::Uint8Array>() {
      Ok(Self::from(arr.to_vec()))
    } else {
      Err(Error::UnexpectedType { expected: "Uint8ClampedArray or Uint8Array" })
    }
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_atomic(self, old, set);
  }
}

#[cfg(feature = "indexmap")]
impl<T: Typewire + Eq + core::hash::Hash> Typewire for indexmap::IndexSet<T> {
  type Ident = schema::coded::SeqIdent<T::Ident>;
  const IDENT: Self::Ident = schema::coded::SeqIdent::new(T::IDENT);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    array_ref(self.iter())
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    let arr: js_sys::Array =
      value.try_into().map_err(|_| Error::UnexpectedType { expected: "array" })?;
    let mut set = indexmap::IndexSet::with_capacity(arr.length() as usize);
    for i in 0..arr.length() {
      set.insert(T::from_js(arr.get(i))?);
    }
    Ok(set)
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js_lenient(value: wasm_bindgen::JsValue, field: &str) -> Result<Self, Error> {
    use wasm_bindgen::JsCast as _;
    let Some(arr) = value.dyn_ref::<js_sys::Array>() else {
      log::warn!("{field}: expected array, skipping");
      return Ok(Self::default());
    };
    let mut set = indexmap::IndexSet::with_capacity(arr.length() as usize);
    for i in 0..arr.length() {
      match T::from_js(arr.get(i)) {
        Ok(v) => {
          set.insert(v);
        }
        Err(e) => log::warn!("{field}[{i}]: skipping invalid element: {e}"),
      }
    }
    Ok(set)
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_slice(self.as_slice().iter(), old, set);
  }
}

#[cfg(feature = "indexmap")]
impl<K: Typewire + Eq + core::hash::Hash, V: Typewire> Typewire for indexmap::IndexMap<K, V> {
  type Ident = schema::coded::MapIdent<K::Ident, V::Ident>;
  const IDENT: Self::Ident = schema::coded::MapIdent::new(K::IDENT, V::IDENT);

  #[cfg(target_arch = "wasm32")]
  fn to_js(&self) -> wasm_bindgen::JsValue {
    let obj = js_sys::Object::new();
    for (k, v) in self {
      let _ = js_sys::Reflect::set(&obj, &k.to_js(), &v.to_js());
    }
    obj.into()
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js(value: wasm_bindgen::JsValue) -> Result<Self, Error> {
    use wasm_bindgen::JsCast as _;
    let entries = js_sys::Object::entries(
      value.dyn_ref::<js_sys::Object>().ok_or(Error::UnexpectedType { expected: "object" })?,
    );
    let mut map = indexmap::IndexMap::with_capacity(entries.length() as usize);
    for i in 0..entries.length() {
      let pair: js_sys::Array = entries.get(i).into();
      let key = K::from_js(pair.get(0))?;
      let val = V::from_js(pair.get(1))?;
      map.insert(key, val);
    }
    Ok(map)
  }

  #[cfg(target_arch = "wasm32")]
  fn from_js_lenient(value: wasm_bindgen::JsValue, field: &str) -> Result<Self, Error> {
    use wasm_bindgen::JsCast as _;
    let Some(obj) = value.dyn_ref::<js_sys::Object>() else {
      log::warn!("{field}: expected object, skipping");
      return Ok(Self::default());
    };
    let entries = js_sys::Object::entries(obj);
    let mut map = indexmap::IndexMap::with_capacity(entries.length() as usize);
    for i in 0..entries.length() {
      let pair: js_sys::Array = entries.get(i).into();
      match (K::from_js(pair.get(0)), V::from_js(pair.get(1))) {
        (Ok(k), Ok(v)) => {
          map.insert(k, v);
        }
        (Err(e), _) | (_, Err(e)) => {
          log::warn!("{field}: skipping entry {i}: {e}");
        }
      }
    }
    Ok(map)
  }

  #[cfg(target_arch = "wasm32")]
  fn patch_js(&self, old: &wasm_bindgen::JsValue, set: impl FnOnce(wasm_bindgen::JsValue)) {
    patch_js_map(
      self.iter().map(|(k, v)| (k.to_js(), v)),
      |js_key| K::from_js(js_key.clone()).ok().is_some_and(|k| self.contains_key(&k)),
      || self.to_js(),
      old,
      set,
    );
  }
}
