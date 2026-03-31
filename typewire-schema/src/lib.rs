//! Schema types for the typewire ecosystem.
//!
//! This crate defines the type metadata used throughout the
//! [typewire pipeline](https://docs.rs/typewire#pipeline):
//! the derive macro encodes [`Schema`] values into link-section records,
//! the [`decode`] module reconstructs them with owned data, and language
//! emitters (e.g. [`typescript`]) consume them to generate declarations.

mod scalar;

pub use zerocopy;

#[doc(hidden)]
pub mod coded;

#[doc(hidden)]
#[cfg(feature = "encode")]
pub mod encode;

/// Decoder: link-section bytes → [`Schema`] with owned data.
#[cfg(feature = "decode")]
pub mod decode;

/// TypeScript declaration emitter.
#[cfg(feature = "typescript")]
pub mod typescript;

use bitflags::bitflags;
/// Scalar (leaf) type identifiers. See [`Scalar`] for variants.
pub use scalar::Scalar;

// ---------------------------------------------------------------------------
// Syn repr — for derive codegen (feature = "encode")
// ---------------------------------------------------------------------------

/// Type aliases used by [`Schema`] and related types.
///
/// At derive-time (`feature = "encode"`), these are `syn` AST types.
/// At decode-time (`feature = "decode"`), these are owned types.
/// The two features are **mutually exclusive** within a single compilation.
#[cfg(feature = "encode")]
pub mod repr {
  /// Type identifier — a `syn::Ident` at derive-time.
  pub type Ident = ::syn::Ident;
  /// String data (field wire names, aliases).
  pub type Str = String;
  /// Type reference — a `syn::Type` at derive-time.
  pub type Ty = ::syn::Type;
  /// Generic parameters.
  pub type Generics = ::syn::Generics;
  /// Type path (e.g. for proxy types).
  pub type Path = ::syn::Path;
  /// Primitive type reference.
  pub type Primitive = ::syn::Type;
  /// Ordered collection.
  pub type Seq<T> = Vec<T>;
}

// ---------------------------------------------------------------------------
// Owned repr — for decoding and codegen (feature = "decode", not "encode")
// ---------------------------------------------------------------------------

/// Type aliases used by [`Schema`] and related types (decode-time).
///
/// See the [`encode` variant](self::repr) for the derive-time counterpart.
#[cfg(all(feature = "decode", not(feature = "encode")))]
pub mod repr {
  /// Type identifier — an owned `String` at decode-time.
  pub type Ident = String;
  /// String data (field wire names, aliases).
  pub type Str = String;
  /// Type reference — a boxed [`Schema`](super::Schema) at decode-time.
  pub type Ty = Box<super::Schema>;
  /// Generic parameters — a list of type parameter names.
  pub type Generics = Vec<String>;
  /// Type path (e.g. for proxy types) — an owned string.
  pub type Path = String;
  /// Primitive type reference — a [`Scalar`](super::Scalar) discriminant.
  pub type Primitive = super::Scalar;
  /// Ordered collection.
  pub type Seq<T> = Vec<T>;
}

// ---------------------------------------------------------------------------
// Flags — always available (used by both coded and Schema)
// ---------------------------------------------------------------------------

bitflags! {
    /// Per-field attribute flags.
    ///
    /// ```
    /// use typewire_schema::FieldFlags;
    ///
    /// let flags = FieldFlags::SKIP_SER | FieldFlags::BASE64;
    /// assert!(flags.contains(FieldFlags::SKIP_SER));
    /// assert!(!flags.contains(FieldFlags::FLATTEN));
    /// ```
    #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
    pub struct FieldFlags: u8 {
        /// `#[serde(skip_serializing)]` — omit from serialized output.
        const SKIP_SER    = 1 << 0;
        /// `#[serde(skip_deserializing)]` — ignore during deserialization.
        const SKIP_DE     = 1 << 1;
        /// `#[serde(flatten)]` — inline nested struct fields into the parent.
        const FLATTEN     = 1 << 2;
        /// `#[typewire(base64)]` — base64-encode `Vec<u8>` fields.
        const BASE64      = 1 << 3;
        /// `#[typewire(display)]` — use `Display`/`FromStr` for conversion.
        const DISPLAY     = 1 << 4;
        /// Byte-array encoding via the `serde_bytes` crate.
        const SERDE_BYTES = 1 << 5;
        /// `#[typewire(lenient)]` — skip invalid elements instead of failing.
        const LENIENT     = 1 << 6;
    }

    /// Per-variant attribute flags.
    ///
    /// ```
    /// use typewire_schema::VariantFlags;
    ///
    /// let flags = VariantFlags::OTHER;
    /// assert!(flags.contains(VariantFlags::OTHER));
    /// ```
    #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
    pub struct VariantFlags: u8 {
        /// `#[serde(skip_serializing)]`
        const SKIP_SER = 1 << 0;
        /// `#[serde(skip_deserializing)]`
        const SKIP_DE  = 1 << 1;
        /// `#[serde(other)]` — catch-all for unknown variant names.
        const OTHER    = 1 << 2;
        /// `#[serde(untagged)]` — untagged within a tagged enum.
        const UNTAGGED = 1 << 3;
    }

    /// Container-level flags for structs.
    ///
    /// ```
    /// use typewire_schema::StructFlags;
    ///
    /// let flags = StructFlags::ATOMIC | StructFlags::CONTAINER_DEFAULT;
    /// assert!(flags.contains(StructFlags::ATOMIC));
    /// ```
    #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
    pub struct StructFlags: u8 {
        /// `#[diffable(atomic)]` — patch as a whole, not per-field.
        const ATOMIC              = 1 << 0;
        /// `#[serde(default)]` — use per-field defaults for missing fields.
        const CONTAINER_DEFAULT   = 1 << 1;
        /// `#[serde(deny_unknown_fields)]` — reject unrecognized keys.
        const DENY_UNKNOWN_FIELDS = 1 << 2;
    }

    /// Container-level flags for enums.
    ///
    /// ```
    /// use typewire_schema::EnumFlags;
    ///
    /// let flags = EnumFlags::ALL_UNIT;
    /// assert!(flags.contains(EnumFlags::ALL_UNIT));
    /// ```
    #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
    pub struct EnumFlags: u8 {
        /// `#[diffable(atomic)]` — patch as a whole.
        const ATOMIC   = 1 << 0;
        /// All variants are unit variants (enables string union in TypeScript).
        const ALL_UNIT = 1 << 1;
    }
}

// SAFETY: bitflags! generates #[repr(transparent)] wrappers around u8,
// so every bit pattern is a valid byte sequence with no padding.
macro_rules! impl_zerocopy_for_flags {
  ($($ty:ident),*) => { $(
    // SAFETY: #[repr(transparent)] around u8.
    unsafe impl zerocopy::IntoBytes for $ty {
      fn only_derive_is_allowed_to_implement_this_trait() {}
    }
    // SAFETY: #[repr(transparent)] around u8.
    unsafe impl zerocopy::Immutable for $ty {
      fn only_derive_is_allowed_to_implement_this_trait() {}
    }
  )* };
}

impl_zerocopy_for_flags!(FieldFlags, VariantFlags, StructFlags, EnumFlags);

// ---------------------------------------------------------------------------
// Schema — the type metadata used by both encode and decode stages
// ---------------------------------------------------------------------------

/// The root schema node representing a Rust type's metadata.
///
/// This enum is the central data structure of the schema pipeline. It has
/// two modes depending on the active feature:
///
/// - **`feature = "encode"`** (derive-time) — 5 definition variants using
///   `syn` AST types. The derive macro builds these, then serializes
///   them into link-section records.
///
/// - **`feature = "decode"`** (decode-time) — 5 definition variants + 8
///   type-reference variants, all using owned data. The [`decode`] module
///   reconstructs these from binary, and language emitters (e.g.
///   [`typescript`]) consume them to generate declarations.
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub enum Schema {
  /// Named type reference (not a definition).
  #[cfg(all(feature = "decode", not(feature = "encode")))]
  Native(repr::Str),
  /// Leaf type: bool, u32, String, Uuid, …
  #[cfg(all(feature = "decode", not(feature = "encode")))]
  Primitive(repr::Primitive),
  /// `Option<T>`
  #[cfg(all(feature = "decode", not(feature = "encode")))]
  Option(repr::Ty),
  /// `Vec<T>`, `IndexSet<T>`
  #[cfg(all(feature = "decode", not(feature = "encode")))]
  Seq(repr::Ty),
  /// `HashMap<K, V>`, `BTreeMap<K, V>`, `IndexMap<K, V>`
  #[cfg(all(feature = "decode", not(feature = "encode")))]
  Map { key: repr::Ty, value: repr::Ty },
  /// `Box<T>`
  #[cfg(all(feature = "decode", not(feature = "encode")))]
  Box(repr::Ty),
  /// `(A, B, ...)` — tuple type reference
  #[cfg(all(feature = "decode", not(feature = "encode")))]
  Tuple(repr::Seq<repr::Ty>),
  /// Skipped field — type unavailable
  #[cfg(all(feature = "decode", not(feature = "encode")))]
  Skipped,

  /// Named / tuple / unit struct
  Struct(Struct),
  /// `#[serde(transparent)]`
  Transparent(Transparent),
  /// Enum (all tagging strategies)
  Enum(Enum),
  /// `#[serde(into = "X")]`
  IntoProxy(IntoProxy),
  /// `#[serde(from = "X")]` / `#[serde(try_from = "X")]`
  FromProxy(FromProxy),
}

#[cfg(any(feature = "encode", feature = "decode"))]
impl Schema {
  /// Returns this type's identifier.
  ///
  /// At encode-time, returns a `&syn::Ident`. At decode-time, returns
  /// `Option<&str>` (type-reference variants like `Primitive` have no name).
  #[cfg(feature = "encode")]
  #[must_use]
  pub const fn ident(&self) -> &repr::Ident {
    match self {
      Self::Struct(s) => &s.ident,
      Self::Transparent(t) => &t.ident,
      Self::Enum(e) => &e.ident,
      Self::IntoProxy(p) => &p.ident,
      Self::FromProxy(p) => &p.ident,
    }
  }

  #[cfg(all(feature = "decode", not(feature = "encode")))]
  #[must_use]
  pub fn ident(&self) -> Option<&str> {
    match self {
      Self::Native(s) => Some(s),
      Self::Struct(s) => Some(&s.ident),
      Self::Transparent(t) => Some(&t.ident),
      Self::Enum(e) => Some(&e.ident),
      Self::IntoProxy(p) => Some(&p.ident),
      Self::FromProxy(p) => Some(&p.ident),
      _ => None,
    }
  }

  /// Returns this type's generic parameters.
  #[cfg(feature = "encode")]
  #[must_use]
  pub const fn generics(&self) -> &repr::Generics {
    match self {
      Self::Struct(s) => &s.generics,
      Self::Transparent(t) => &t.generics,
      Self::Enum(e) => &e.generics,
      Self::IntoProxy(p) => &p.generics,
      Self::FromProxy(p) => &p.generics,
    }
  }

  #[cfg(all(feature = "decode", not(feature = "encode")))]
  #[must_use]
  pub const fn generics(&self) -> Option<&repr::Generics> {
    match self {
      Self::Struct(s) => Some(&s.generics),
      Self::Transparent(t) => Some(&t.generics),
      Self::Enum(e) => Some(&e.generics),
      Self::IntoProxy(p) => Some(&p.generics),
      Self::FromProxy(p) => Some(&p.generics),
      _ => None,
    }
  }

  /// Returns `true` if this is a type-reference variant (not a definition).
  #[cfg(all(feature = "decode", not(feature = "encode")))]
  #[must_use]
  pub const fn is_type_ref(&self) -> bool {
    !matches!(
      self,
      Self::Struct(_)
        | Self::Transparent(_)
        | Self::Enum(_)
        | Self::IntoProxy(_)
        | Self::FromProxy(_)
    )
  }
}

// ---------------------------------------------------------------------------
// Field
// ---------------------------------------------------------------------------

/// How a field's default value is provided when the field is absent.
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone, Default)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub enum FieldDefault {
  /// No default — field is required (unless `or_default()` provides one).
  #[default]
  None,
  /// Use `Default::default()`.
  Default,
  /// Call a specific function path.
  Path(repr::Path),
}

/// A struct or variant field with its metadata.
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct Field {
  /// The Rust field name.
  pub ident: repr::Ident,
  /// The field's type (as a schema reference).
  pub ty: repr::Ty,
  /// The wire name after applying `rename` / `rename_all`.
  pub wire_name: repr::Str,
  /// Attribute flags (skip, flatten, base64, etc.).
  pub flags: FieldFlags,
  /// Additional accepted names from `#[serde(alias = "...")]`.
  pub aliases: repr::Seq<repr::Str>,
  /// How the default value is provided when the field is absent.
  pub default: FieldDefault,
  /// Path for `#[serde(skip_serializing_if = "...")]`.
  pub skip_serializing_if: Option<repr::Path>,
}

// ---------------------------------------------------------------------------
// Struct
// ---------------------------------------------------------------------------

/// The body shape of a struct: named fields, tuple, or unit.
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub enum StructShape {
  /// `struct Foo { bar: T, ... }` — named fields.
  Named(repr::Seq<Field>),
  /// `struct Foo(T, U, ...)` — positional fields.
  Tuple(repr::Seq<repr::Ty>),
  /// `struct Foo;` — no fields.
  Unit,
}

/// A struct type's schema metadata.
///
/// Covers named structs, tuple structs, and unit structs.
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct Struct {
  /// The Rust type name.
  pub ident: repr::Ident,
  /// Generic type parameters.
  pub generics: repr::Generics,
  /// Container-level attribute flags.
  pub flags: StructFlags,
  /// The struct body shape (named, tuple, or unit).
  pub shape: StructShape,
}

// ---------------------------------------------------------------------------
// Transparent
// ---------------------------------------------------------------------------

/// A `#[serde(transparent)]` newtype wrapper.
///
/// The type's wire representation is identical to its inner field's.
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct Transparent {
  /// The Rust type name.
  pub ident: repr::Ident,
  /// Generic type parameters.
  pub generics: repr::Generics,
  /// Whether patching is atomic (`#[diffable(atomic)]`).
  pub atomic: bool,
  /// `Some` for named fields (`struct Foo { inner: T }`), `None` for tuple
  /// fields (`struct Foo(T)`).
  pub field_ident: Option<repr::Ident>,
  /// The inner field's type.
  pub field_ty: repr::Ty,
}

// ---------------------------------------------------------------------------
// Enum
// ---------------------------------------------------------------------------

/// Serde enum tagging strategy.
///
/// Determines how variant names appear in the wire format:
///
/// - [`External`](Tagging::External) — `{ "VariantName": { ... } }` (serde default)
/// - [`Internal`](Tagging::Internal) — `{ "tag": "VariantName", ... }`
/// - [`Adjacent`](Tagging::Adjacent) — `{ "tag": "VariantName", "content": { ... } }`
/// - [`Untagged`](Tagging::Untagged) — variant is inferred from content shape
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Tagging {
  /// `{ "VariantName": payload }` — the serde default.
  External,
  /// `{ tag: "VariantName", ...fields }` — `#[serde(tag = "...")]`.
  Internal { tag: repr::Str },
  /// `{ tag: "VariantName", content: payload }` — `#[serde(tag, content)]`.
  Adjacent { tag: repr::Str, content: repr::Str },
  /// No tag — variant is matched by content. `#[serde(untagged)]`.
  Untagged,
}

/// The payload shape of an enum variant.
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub enum VariantKind {
  /// `Variant` — no payload.
  Unit,
  /// `Variant { field: T, ... }` — named fields.
  Named(repr::Seq<Field>),
  /// `Variant(T, U, ...)` — positional fields.
  Unnamed(repr::Seq<repr::Ty>),
}

/// An enum variant with its metadata.
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct Variant {
  /// The Rust variant name.
  pub ident: repr::Ident,
  /// The primary wire name after applying `rename` / `rename_all`.
  pub wire_name: repr::Str,
  /// Primary name + all `#[serde(alias = "...")]` names.
  pub all_wire_names: repr::Seq<repr::Str>,
  /// Attribute flags (skip, other, untagged).
  pub flags: VariantFlags,
  /// The variant's payload shape.
  pub kind: VariantKind,
}

/// An enum type's schema metadata.
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct Enum {
  /// The Rust type name.
  pub ident: repr::Ident,
  /// Generic type parameters.
  pub generics: repr::Generics,
  /// Container-level attribute flags.
  pub flags: EnumFlags,
  /// The tagging strategy.
  pub tagging: Tagging,
  /// The enum's variants.
  pub variants: repr::Seq<Variant>,
}

// ---------------------------------------------------------------------------
// Proxy types
// ---------------------------------------------------------------------------

/// The struct/enum shape of a type, used internally by proxy codegen.
///
/// Only available at derive-time (`feature = "encode"`).
#[cfg(feature = "encode")]
#[derive(Clone)]
pub enum TypeShape {
  /// The type is a struct.
  Struct(Struct),
  /// The type is an enum.
  Enum(Enum),
}

/// How deserialization is resolved for a type with `#[serde(into = "X")]`.
///
/// Only available at derive-time (`feature = "encode"`).
#[cfg(feature = "encode")]
#[derive(Clone)]
pub enum FromBody {
  /// `#[serde(from = "X")]` — delegate to proxy's `from_js`, then `From`.
  Proxy(repr::Path),
  /// `#[serde(try_from = "X")]` — delegate to proxy's `from_js`, then `TryFrom`.
  TryProxy(repr::Path),
  /// No `from`/`try_from` — build from the type's own fields.
  Own(TypeShape),
}

/// Schema for a type with `#[serde(into = "X")]`.
///
/// Serialization converts `Self` into the proxy type `X`, then serializes `X`.
/// The proxy type must also implement `Typewire`.
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct IntoProxy {
  /// The Rust type name.
  pub ident: repr::Ident,
  /// Generic type parameters.
  pub generics: repr::Generics,
  /// The proxy type path (the `X` in `into = "X"`).
  pub into_ty: repr::Path,
  /// How deserialization is resolved. Only available at derive-time —
  /// the binary format does not encode this.
  #[cfg(feature = "encode")]
  pub from_body: FromBody,
}

/// Schema for a type with `#[serde(from = "X")]` or `#[serde(try_from = "X")]`.
///
/// Deserialization reads the proxy type `X`, then converts via
/// `From<X>` or `TryFrom<X>`.
#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct FromProxy {
  /// The Rust type name.
  pub ident: repr::Ident,
  /// Generic type parameters.
  pub generics: repr::Generics,
  /// The proxy type path.
  pub proxy: repr::Path,
  /// `true` for `try_from`, `false` for `from`.
  pub is_try: bool,
  /// The type's own struct/enum shape, used for serialization and patching.
  /// Only available at derive-time — the binary format does not encode this.
  #[cfg(feature = "encode")]
  pub own_shape: TypeShape,
}
