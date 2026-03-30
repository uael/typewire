use std::{path::PathBuf, sync::LazyLock};

use anyhow::Result;
use clap::{Parser, Subcommand};
use xshell::{Shell, cmd};

#[derive(Parser)]
#[command(name = "xtask", about = "Typewire project automation")]
#[command(styles = clap_cargo::style::CLAP_STYLING)]
#[clap(disable_version_flag = true, bin_name = "cargo xtask")]
struct Cli {
  #[command(subcommand)]
  command: Command,
}

#[derive(Subcommand)]
enum Command {
  /// Format code
  Fmt {
    /// Check formatting without making changes
    #[arg(long)]
    check: bool,
  },
  /// Lint code and check formatting
  Lint {
    /// Fix lint issues automatically
    #[arg(long)]
    fix: bool,
  },
}

/// Project root directory.
static ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
  std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(1).unwrap().to_path_buf()
});

fn main() -> Result<()> {
  let cli = Cli::parse();

  let mut sh = Shell::new()?;
  sh.set_current_dir(ROOT.as_path());

  match cli.command {
    Command::Fmt { check } => fmt(&sh, check),
    Command::Lint { fix } => lint(&sh, fix),
  }
}

fn fmt(sh: &Shell, check: bool) -> Result<()> {
  let args =
    std::iter::once("--all").chain(check.then_some(["--", "--check"]).into_iter().flatten());
  cmd!(sh, "cargo +nightly fmt {args...}").run_echo()?;
  Ok(())
}

fn lint(sh: &Shell, fix: bool) -> Result<()> {
  let args =
    if fix { ["--fix", "--allow-dirty", "--allow-staged"] } else { ["--", "-D", "warnings"] };

  // typewire-schema's `syn` and `codegen` features are mutually exclusive,
  // so we lint each meaningful feature combination separately.

  // Workspace with default features (typewire[derive], typewire-schema[syn]).
  cmd!(sh, "cargo clippy --tests {args...}").run_echo()?;

  // typewire-schema: no features (coded only).
  cmd!(sh, "cargo clippy -p typewire-schema --tests --no-default-features {args...}").run_echo()?;

  // typewire-schema: syn path (encode).
  cmd!(sh, "cargo clippy -p typewire-schema --tests --features syn {args...}").run_echo()?;

  // typewire-schema: typescript path (decode + codegen, no syn).
  cmd!(sh, "cargo clippy -p typewire-schema --tests --features typescript {args...}").run_echo()?;

  // typewire: no features (no derive, no optional deps).
  cmd!(sh, "cargo clippy -p typewire --no-default-features {args...}").run_echo()?;

  // typewire: all optional type features.
  let type_features = "uuid,fractional_index,chrono,url,indexmap,bytes,base64,serde_json";
  cmd!(sh, "cargo clippy -p typewire --tests --features {type_features} {args...}").run_echo()?;

  // typewire: cli feature without derive (codegen/typescript path).
  cmd!(sh, "cargo clippy -p typewire --no-default-features --features cli {args...}").run_echo()?;

  // typewire-derive.
  cmd!(sh, "cargo clippy -p typewire-derive --tests {args...}").run_echo()?;

  fmt(sh, !fix)
}
