use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result};
use bitflags::bitflags;
use xshell::{Shell, cmd};

bitflags! {
  /// Options for `bindgen`.
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub struct BindgenFlags: u8 {
    /// Emit Node.js-compatible bindings (default: web target).
    const NODEJS     = 1;
    /// Run `wasm-opt -Oz` on the output.
    const OPTIMIZE   = 1 << 1;
    /// Generate `.d.ts` type declarations.
    const TYPESCRIPT = 1 << 2;
  }
}

/// Run wasm-bindgen on a compiled `.wasm` file.
///
/// Generates JS bindings in `out_dir` according to `flags`. Returns the path
/// to the `_bg.wasm`.
pub fn bindgen(sh: &Shell, input: &Path, out_dir: &Path, flags: BindgenFlags) -> Result<PathBuf> {
  let mut b = wasm_bindgen_cli_support::Bindgen::new();
  b.input_path(input).typescript(flags.contains(BindgenFlags::TYPESCRIPT));
  if flags.contains(BindgenFlags::NODEJS) {
    b.nodejs(true)?;
  } else {
    b.web(true)?;
  }
  b.generate(out_dir)?;

  let stem = input
    .file_stem()
    .and_then(|s| s.to_str())
    .context("input path has no valid UTF-8 file stem")?;
  let bg_wasm = out_dir.join(format!("{stem}_bg.wasm"));

  if flags.contains(BindgenFlags::OPTIMIZE) {
    cmd!(
      sh,
      "wasm-opt -Oz --enable-reference-types --enable-bulk-memory --enable-multivalue
        --enable-nontrapping-float-to-int --enable-sign-ext
        -o {bg_wasm} {bg_wasm}"
    )
    .run_echo()?;
  }

  Ok(bg_wasm)
}
