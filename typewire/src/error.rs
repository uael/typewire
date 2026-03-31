#[derive(Debug, thiserror::Error)]
pub enum Error {
  #[error("expected {expected}")]
  UnexpectedType { expected: &'static str },

  #[error("missing field `{field}`")]
  MissingField { field: &'static str },

  #[error("unknown variant `{variant}`")]
  UnknownVariant { variant: String },

  #[error("invalid value: {message}")]
  InvalidValue { message: String },

  #[error("out of range")]
  OutOfRange,

  #[error("no matching variant")]
  NoMatchingVariant,

  #[error("{0}")]
  Custom(String),

  #[error("in `{context}`: {source}")]
  Context { context: String, source: Box<Self> },
}

impl Error {
  /// Wraps this error with the name of the type or field where it occurred.
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
