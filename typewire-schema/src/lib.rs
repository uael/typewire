//! Schema types for the typewire ecosystem.
//!
//! This crate defines the type metadata used throughout the typewire
//! pipeline. The data flows through three stages:
//!
//! ```text
//! ┌──────────────┐      ┌───────────────┐     ┌───────────────┐
//! │ syn::Schema  │      │  coded::*     │     │  Schema       │
//! │ (derive-time)│─encode─▶ (link section)─decode─▶ (codegen) │
//! └──────────────┘      └───────────────┘     └───────────────┘
//!   feature="encode"      always available      feature="decode"
//! ```
//!
//! 1. **`encode`** — The derive macro analyzes Rust types into [`Schema`] values
//!    with `syn` AST types, then the `encode` module serializes them as
//!    flat [`coded::Record<T>`](coded::Record) statics embedded in link sections.
//!
//! 2. **`coded`** — The binary format. Always available. `#[repr(C, packed)]`,
//!    `Copy`, const-constructible types that linkers concatenate into a single
//!    `typewire_schemas` section.
//!
//! 3. **`decode`** — The `decode` module reads link-section bytes back into
//!    [`Schema`] values with owned data. Language-specific emitters (e.g.
//!    `typescript`) consume these to generate bindings.
//!
//! # Modules
//!
//! | Module | Feature | Purpose |
//! |--------|---------|---------|
//! | [`coded`] | *(always)* | Binary format types for link-section embedding |
//! | `encode` | `encode` | `Schema` → `TokenStream` (link-section record construction) |
//! | `decode` | `decode` | Link-section bytes → `Schema` |
//! | `typescript` | `typescript` | `Schema` → `.d.ts` declarations |

mod scalar;

pub use zerocopy;

/// Binary format types — always available.
pub mod coded;

/// Encoder: [`Schema`] → [`coded::Record<T>`](coded::Record) TokenStream.
#[cfg(feature = "encode")]
pub mod encode;

/// Decoder: link-section bytes → [`Schema`] with owned data.
#[cfg(feature = "decode")]
pub mod decode;

/// TypeScript declaration emitter.
#[cfg(feature = "typescript")]
pub mod typescript;

use bitflags::bitflags;
pub use scalar::Scalar;

// ---------------------------------------------------------------------------
// Syn repr — for derive codegen (feature = "encode")
// ---------------------------------------------------------------------------

#[cfg(feature = "encode")]
pub mod repr {
  pub type Ident = ::syn::Ident;
  pub type Str = String;
  pub type Ty = ::syn::Type;
  pub type Generics = ::syn::Generics;
  pub type Path = ::syn::Path;
  pub type Primitive = ::syn::Type;
  pub type Seq<T> = Vec<T>;
}

// ---------------------------------------------------------------------------
// Owned repr — for decoding and codegen (feature = "decode", not "encode")
// ---------------------------------------------------------------------------

#[cfg(all(feature = "decode", not(feature = "encode")))]
pub mod repr {
  pub type Ident = String;
  pub type Str = String;
  pub type Ty = Box<super::Schema>;
  pub type Generics = Vec<String>;
  pub type Path = String;
  pub type Primitive = super::Scalar;
  pub type Seq<T> = Vec<T>;
}

// ---------------------------------------------------------------------------
// Flags — always available (used by both coded and Schema)
// ---------------------------------------------------------------------------

bitflags! {
    #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
    pub struct FieldFlags: u8 {
        const SKIP_SER    = 1 << 0;
        const SKIP_DE     = 1 << 1;
        const FLATTEN     = 1 << 2;
        const BASE64      = 1 << 3;
        const DISPLAY     = 1 << 4;
        const SERDE_BYTES = 1 << 5;
        const LENIENT     = 1 << 6;
    }

    #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
    pub struct VariantFlags: u8 {
        const SKIP_SER = 1 << 0;
        const SKIP_DE  = 1 << 1;
        const OTHER    = 1 << 2;
        const UNTAGGED = 1 << 3;
    }

    #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
    pub struct StructFlags: u8 {
        const ATOMIC              = 1 << 0;
        const CONTAINER_DEFAULT   = 1 << 1;
        const DENY_UNKNOWN_FIELDS = 1 << 2;
    }

    #[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
    pub struct EnumFlags: u8 {
        const ATOMIC   = 1 << 0;
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

/// The root schema node for a type.
///
/// With `feature = "encode"` (derive-time): 5 definition variants using syn AST types.
/// With `feature = "decode"` (decode-time): 5 definition variants + 8 type-reference
/// variants, all using owned data.
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

#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct Field {
  pub ident: repr::Ident,
  pub ty: repr::Ty,
  /// The wire name after applying `rename` / `rename_all`.
  pub wire_name: repr::Str,
  pub flags: FieldFlags,
  pub aliases: repr::Seq<repr::Str>,
  pub default: FieldDefault,
  pub skip_serializing_if: Option<repr::Path>,
}

// ---------------------------------------------------------------------------
// Struct
// ---------------------------------------------------------------------------

#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub enum StructShape {
  Named(repr::Seq<Field>),
  Tuple(repr::Seq<repr::Ty>),
  Unit,
}

#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct Struct {
  pub ident: repr::Ident,
  pub generics: repr::Generics,
  pub flags: StructFlags,
  pub shape: StructShape,
}

// ---------------------------------------------------------------------------
// Transparent
// ---------------------------------------------------------------------------

#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct Transparent {
  pub ident: repr::Ident,
  pub generics: repr::Generics,
  pub atomic: bool,
  /// `Some` for named fields, `None` for tuple fields.
  pub field_ident: Option<repr::Ident>,
  pub field_ty: repr::Ty,
}

// ---------------------------------------------------------------------------
// Enum
// ---------------------------------------------------------------------------

#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone, Debug, Hash, PartialEq, Eq)]
pub enum Tagging {
  External,
  Internal { tag: repr::Str },
  Adjacent { tag: repr::Str, content: repr::Str },
  Untagged,
}

#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub enum VariantKind {
  Unit,
  Named(repr::Seq<Field>),
  Unnamed(repr::Seq<repr::Ty>),
}

#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct Variant {
  pub ident: repr::Ident,
  /// The primary wire name after applying `rename` / `rename_all`.
  pub wire_name: repr::Str,
  /// Primary name + all `#[serde(alias)]` names.
  pub all_wire_names: repr::Seq<repr::Str>,
  pub flags: VariantFlags,
  pub kind: VariantKind,
}

#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct Enum {
  pub ident: repr::Ident,
  pub generics: repr::Generics,
  pub flags: EnumFlags,
  pub tagging: Tagging,
  pub variants: repr::Seq<Variant>,
}

// ---------------------------------------------------------------------------
// Proxy types
// ---------------------------------------------------------------------------

/// The struct/enum shape for proxy types that need their own codegen.
/// Only available at derive-time (`feature = "encode"`).
#[cfg(feature = "encode")]
#[derive(Clone)]
pub enum TypeShape {
  Struct(Struct),
  Enum(Enum),
}

/// How `from_js` is resolved for an `#[serde(into)]` type.
/// Only available at derive-time (`feature = "encode"`).
#[cfg(feature = "encode")]
#[derive(Clone)]
pub enum FromBody {
  /// `#[serde(from = "X")]` — delegate to proxy's `from_js`.
  Proxy(repr::Path),
  /// `#[serde(try_from = "X")]` — delegate to proxy's `from_js` + `try_from`.
  TryProxy(repr::Path),
  /// No `from/try_from` — build from the type's own fields.
  Own(TypeShape),
}

#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct IntoProxy {
  pub ident: repr::Ident,
  pub generics: repr::Generics,
  pub into_ty: repr::Path,
  /// How `from_js` is resolved. Only available at derive-time — the
  /// binary format does not encode this.
  #[cfg(feature = "encode")]
  pub from_body: FromBody,
}

#[cfg(any(feature = "encode", feature = "decode"))]
#[derive(Clone)]
#[cfg_attr(not(feature = "encode"), derive(Debug, Hash, PartialEq, Eq))]
pub struct FromProxy {
  pub ident: repr::Ident,
  pub generics: repr::Generics,
  pub proxy: repr::Path,
  pub is_try: bool,
  /// The type's own struct/enum shape, used for `to_js` and `patch_js`.
  /// Only available at derive-time — the binary format does not encode this.
  #[cfg(feature = "encode")]
  pub own_shape: TypeShape,
}
