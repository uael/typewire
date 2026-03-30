/// Serde `rename_all` case conventions.
#[derive(Debug, Clone, Copy)]
pub enum RenameAll {
  Lower,
  Upper,
  Pascal,
  Camel,
  Snake,
  ScreamingSnake,
  Kebab,
  ScreamingKebab,
}

impl RenameAll {
  pub(crate) fn parse(s: &str) -> Option<Self> {
    match s {
      "lowercase" => Some(Self::Lower),
      "UPPERCASE" => Some(Self::Upper),
      "PascalCase" => Some(Self::Pascal),
      "camelCase" => Some(Self::Camel),
      "snake_case" => Some(Self::Snake),
      "SCREAMING_SNAKE_CASE" => Some(Self::ScreamingSnake),
      "kebab-case" => Some(Self::Kebab),
      "SCREAMING-KEBAB-CASE" => Some(Self::ScreamingKebab),
      _ => None,
    }
  }

  /// Convert an identifier to the target case. `ident` is in its original
  /// Rust form (`PascalCase` for variants, `snake_case` for fields).
  pub(crate) fn apply(self, ident: &str) -> String {
    let words = split_words(ident);
    match self {
      Self::Lower => words.iter().map(|w| w.to_lowercase()).collect(),
      Self::Upper => words.iter().map(|w| w.to_uppercase()).collect(),
      Self::Pascal => words
        .iter()
        .map(|w| {
          let mut c = w.chars();
          c.next().map_or_else(String::new, |first| {
            first.to_uppercase().to_string() + &c.as_str().to_lowercase()
          })
        })
        .collect(),
      Self::Camel => {
        let mut out = String::new();
        for (i, w) in words.iter().enumerate() {
          if i == 0 {
            out.push_str(&w.to_lowercase());
          } else {
            let mut c = w.chars();
            if let Some(first) = c.next() {
              out.extend(first.to_uppercase());
              out.push_str(&c.as_str().to_lowercase());
            }
          }
        }
        out
      }
      Self::Snake => words.iter().map(|w| w.to_lowercase()).collect::<Vec<_>>().join("_"),
      Self::ScreamingSnake => words.iter().map(|w| w.to_uppercase()).collect::<Vec<_>>().join("_"),
      Self::Kebab => words.iter().map(|w| w.to_lowercase()).collect::<Vec<_>>().join("-"),
      Self::ScreamingKebab => words.iter().map(|w| w.to_uppercase()).collect::<Vec<_>>().join("-"),
    }
  }
}

/// Split a Rust identifier into words, handling both `PascalCase` and
/// `snake_case` inputs.
fn split_words(ident: &str) -> Vec<String> {
  let mut words = Vec::new();
  let mut current = String::new();

  // If the identifier contains underscores, split on them (snake_case
  // or SCREAMING_SNAKE_CASE).
  if ident.contains('_') {
    for part in ident.split('_') {
      if !part.is_empty() {
        words.push(part.to_string());
      }
    }
    return words;
  }

  // Otherwise, split on case boundaries (PascalCase / camelCase).
  let chars: Vec<char> = ident.chars().collect();
  for (i, &ch) in chars.iter().enumerate() {
    if ch.is_uppercase() && i > 0 {
      // Start a new word on uppercase after lowercase, or when
      // transitioning from an acronym (e.g. "HTTPSPort" → ["HTTPS", "Port"]).
      let prev_lower = chars[i - 1].is_lowercase();
      let next_lower = chars.get(i + 1).is_some_and(|c| c.is_lowercase());
      if (prev_lower || next_lower) && !current.is_empty() {
        words.push(current.clone());
        current.clear();
      }
    }
    current.push(ch);
  }
  if !current.is_empty() {
    words.push(current);
  }
  words
}
