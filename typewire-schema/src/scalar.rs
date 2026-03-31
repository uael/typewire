/// Scalar (leaf) type identifiers for the schema representation.
///
/// Each variant maps to a Rust primitive or a well-known domain type for which
/// `typewire` provides a built-in `Typewire` implementation.
///
/// Scalars are the terminal nodes of the schema type graph — they have no inner
/// type parameters. Compound types like `Option<T>`, `Vec<T>`, and `HashMap<K, V>`
/// are represented by [`Schema`](crate::Schema) variants instead.
///
/// # Mapping to foreign types
///
/// | Scalar | Rust type | TypeScript |
/// |--------|-----------|------------|
/// | [`bool`](Scalar::bool) | [`bool`] | `boolean` |
/// | [`u8`](Scalar::u8) .. [`u32`](Scalar::u32), [`i8`](Scalar::i8) .. [`i32`](Scalar::i32) | integer primitives | `number` |
/// | [`u64`](Scalar::u64) .. [`u128`](Scalar::u128), [`i64`](Scalar::i64) .. [`i128`](Scalar::i128) | large integers | `number` (JS: lossy >2⁵³) |
/// | [`usize`](Scalar::usize), [`isize`](Scalar::isize) | pointer-width integers | `number` |
/// | [`f32`](Scalar::f32), [`f64`](Scalar::f64) | floating-point | `number` |
/// | [`char`](Scalar::char) | single Unicode scalar | `string` |
/// | [`str`](Scalar::str) | `String` / `Cow<str>` | `string` |
/// | [`Unit`](Scalar::Unit) | `()` | `null` |
/// | [`Url`](Scalar::Url) | [`url::Url`](https://docs.rs/url/latest/url/struct.Url.html) | `string` |
/// | [`Uuid`](Scalar::Uuid) | [`uuid::Uuid`](https://docs.rs/uuid/latest/uuid/struct.Uuid.html) | `string` |
/// | [`Bytes`](Scalar::Bytes) | [`bytes::Bytes`](https://docs.rs/bytes/latest/bytes/struct.Bytes.html) | `Uint8ClampedArray` |
/// | [`DateTime`](Scalar::DateTime) | [`chrono::DateTime<Tz>`](https://docs.rs/chrono/latest/chrono/struct.DateTime.html) | `string` (RFC 3339) |
/// | [`SerdeJsonValue`](Scalar::SerdeJsonValue) | [`serde_json::Value`](https://docs.rs/serde_json/latest/serde_json/enum.Value.html) | `any` |
/// | [`FractionalIndex`](Scalar::FractionalIndex) | [`fractional_index::FractionalIndex`](https://docs.rs/fractional_index/latest/fractional_index/struct.FractionalIndex.html) | `string` |
///
/// ```
/// use typewire_schema::Scalar;
///
/// // Convert from discriminant
/// assert_eq!(Scalar::from_u8(0), Some(Scalar::bool));
/// assert_eq!(Scalar::from_u8(16), Some(Scalar::str));
/// assert_eq!(Scalar::from_u8(255), None);
/// ```
#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq, zerocopy::IntoBytes, zerocopy::Immutable)]
#[expect(non_camel_case_types, reason = "Named after the Rust types they represent.")]
#[repr(u8)]
pub enum Scalar {
  /// [`bool`]
  bool,
  /// [`u8`]
  u8,
  /// [`u16`]
  u16,
  /// [`u32`]
  u32,
  /// [`u64`]
  u64,
  /// [`u128`]
  u128,
  /// [`i8`]
  i8,
  /// [`i16`]
  i16,
  /// [`i32`]
  i32,
  /// [`i64`]
  i64,
  /// [`i128`]
  i128,
  /// [`usize`]
  usize,
  /// [`isize`]
  isize,
  /// [`f32`]
  f32,
  /// [`f64`]
  f64,
  /// [`char`]
  char,
  /// [`String`] / [`Cow<str>`](std::borrow::Cow)
  str,
  /// `()`
  Unit,
  /// [`url::Url`](https://docs.rs/url/latest/url/struct.Url.html)
  Url,
  /// [`uuid::Uuid`](https://docs.rs/uuid/latest/uuid/struct.Uuid.html)
  Uuid,
  /// [`bytes::Bytes`](https://docs.rs/bytes/latest/bytes/struct.Bytes.html)
  Bytes,
  /// [`chrono::DateTime<Tz>`](https://docs.rs/chrono/latest/chrono/struct.DateTime.html)
  DateTime,
  /// [`serde_json::Value`](https://docs.rs/serde_json/latest/serde_json/enum.Value.html)
  SerdeJsonValue,
  /// [`fractional_index::FractionalIndex`](https://docs.rs/fractional_index/latest/fractional_index/struct.FractionalIndex.html)
  FractionalIndex,
}

// Free `const _` is always evaluated — catches non-contiguous discriminants
// at compile time.
const _: () = assert!(
  Scalar::FractionalIndex as u8 as usize + 1 == 24,
  "Scalar: variant count does not match max discriminant + 1"
);

impl Scalar {
  /// Converts a `u8` discriminant back to a [`Scalar`] value.
  ///
  /// Returns `None` if the value exceeds the maximum discriminant.
  ///
  /// ```
  /// use typewire_schema::Scalar;
  ///
  /// assert_eq!(Scalar::from_u8(0), Some(Scalar::bool));
  /// assert_eq!(Scalar::from_u8(23), Some(Scalar::FractionalIndex));
  /// assert_eq!(Scalar::from_u8(24), None);
  /// ```
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
