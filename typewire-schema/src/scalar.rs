/// Scalar (leaf) type identifiers for the static schema representation.
///
/// Each variant maps to a primitive Rust type or a well-known domain type
/// for which `typewire` provides a built-in `Typewire` implementation.
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[expect(non_camel_case_types, reason = "Named after the Rust types they represent.")]
#[repr(u8)]
pub enum Scalar {
  bool,
  u8,
  u16,
  u32,
  u64,
  u128,
  i8,
  i16,
  i32,
  i64,
  i128,
  usize,
  isize,
  f32,
  f64,
  char,
  str,
  Unit,
  Url,
  Uuid,
  Bytes,
  DateTime,
  SerdeJsonValue,
  FractionalIndex,
}

// Free `const _` is always evaluated — catches non-contiguous discriminants
// at compile time.
const _: () = assert!(
  Scalar::FractionalIndex as u8 as usize + 1 == 24,
  "Scalar: variant count does not match max discriminant + 1"
);

impl Scalar {
  /// Convert a u8 discriminant back to a `Scalar` value.
  ///
  /// Returns `None` if the value is out of range.
  #[must_use]
  pub fn from_u8(v: u8) -> Option<Self> {
    if v <= Self::FractionalIndex as u8 {
      // SAFETY: Scalar is #[repr(u8)] with contiguous variants
      // 0 through FractionalIndex, verified by the const assert above.
      Some(unsafe { core::mem::transmute::<u8, Self>(v) })
    } else {
      None
    }
  }
}
