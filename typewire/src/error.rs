/// Errors produced when converting foreign-language values to Rust types.
///
/// Generated conversion implementations (e.g. `from_js` on wasm32)
/// return this error when the input value doesn't match the expected shape.
/// Errors can be nested with [`in_context`](Error::in_context) to build a
/// path to the problematic field.
///
/// ```
/// use typewire::Error;
///
/// let err = Error::UnexpectedType { expected: "number" };
/// let wrapped = err.in_context("age").in_context("User");
/// assert_eq!(wrapped.to_string(), "in `User`: in `age`: expected number");
/// ```
///
/// On `wasm32`, `Error` converts to a JS `Error` via
/// `impl From<Error> for JsValue`.
#[derive(Debug, thiserror::Error)]
pub enum Error {
  /// The value had the wrong JavaScript type (e.g. got a string, expected a number).
  #[error("expected {expected}")]
  UnexpectedType { expected: &'static str },

  /// A required field was missing from a JS object.
  #[error("missing field `{field}`")]
  MissingField { field: &'static str },

  /// An enum variant name didn't match any known variant.
  #[error("unknown variant `{variant}`")]
  UnknownVariant { variant: String },

  /// The value had the right type but an invalid content (e.g. bad UUID string).
  #[error("invalid value: {message}")]
  InvalidValue { message: String },

  /// A numeric value was outside the representable range for the target type.
  #[error("out of range")]
  OutOfRange,

  /// No variant of an untagged enum matched the input.
  #[error("no matching variant")]
  NoMatchingVariant,

  /// A free-form error message.
  #[error("{0}")]
  Custom(String),

  /// Wraps an inner error with the name of the enclosing type or field.
  #[error("in `{context}`: {source}")]
  Context {
    /// The type or field name providing context.
    context: String,
    /// The inner error.
    source: Box<Self>,
  },
}

impl Error {
  /// Wraps this error with the name of the type or field where it occurred,
  /// producing a [`Context`](Error::Context) variant.
  ///
  /// Chaining multiple `in_context` calls builds a readable path:
  ///
  /// ```
  /// use typewire::Error;
  ///
  /// let err = Error::MissingField { field: "name" }
  ///   .in_context("inner")
  ///   .in_context("Outer");
  /// assert_eq!(
  ///   err.to_string(),
  ///   "in `Outer`: in `inner`: missing field `name`",
  /// );
  /// ```
  #[must_use]
  pub fn in_context(self, context: impl Into<String>) -> Self {
    Self::Context { context: context.into(), source: Box::new(self) }
  }
}

#[cfg(target_arch = "wasm32")]
impl From<Error> for wasm_bindgen::JsValue {
  fn from(err: Error) -> Self {
    js_sys::Error::new(&err.to_string()).into()
  }
}
