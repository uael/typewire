use std::{path::PathBuf, sync::LazyLock};

use anyhow::{Result, bail};
use clap::{Parser, Subcommand, ValueEnum};
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
  /// Build documentation
  Doc,
  /// Run tests
  Test {
    /// Which test suite to run (default: all)
    #[arg(value_enum)]
    suite: Option<TestSuite>,
    /// Collect code coverage via cargo-llvm-cov (unit tests only)
    #[arg(long)]
    coverage: bool,
    /// Write per-crate coverage JSON to this path
    #[arg(long, value_name = "PATH")]
    coverage_output: Option<PathBuf>,
  },
}

#[derive(Clone, ValueEnum)]
enum TestSuite {
  /// Native unit + integration tests
  Unit,
  /// wasm32 tests (requires wasm-bindgen-cli)
  Wasm,
  /// End-to-end: build wasm -> typegen -> tsc -> node
  E2e,
}

/// Project root directory.
static ROOT: LazyLock<PathBuf> = LazyLock::new(|| {
  std::path::Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(1).unwrap().to_path_buf()
});

const WASM_TARGET: &str = "wasm32-unknown-unknown";

/// Optional type features for the `typewire` crate. Keep in sync with
/// `typewire/Cargo.toml` `[package.metadata.docs.rs]`.
const TYPE_FEATURES: &str = "uuid,fractional_index,chrono,url,indexmap,bytes,base64,serde_json";

/// Crates to report coverage for.
const COVERAGE_CRATES: &[&str] = &["typewire", "typewire-derive", "typewire-schema"];

fn main() -> Result<()> {
  let cli = Cli::parse();

  let mut sh = Shell::new()?;
  sh.set_current_dir(ROOT.as_path());

  match cli.command {
    Command::Fmt { check } => fmt(&sh, check),
    Command::Lint { fix } => lint(&sh, fix),
    Command::Doc => doc(&sh),
    Command::Test { suite, coverage, coverage_output } => match suite {
      Some(TestSuite::Unit) => {
        if coverage {
          test_unit_with_coverage(&sh, coverage_output.as_deref())
        } else {
          test_unit(&sh)
        }
      }
      Some(TestSuite::Wasm) => {
        if coverage {
          bail!(
            "--coverage is not supported for wasm tests (LLVM instrument-coverage \
                 does not target wasm32)"
          );
        }
        test_wasm(&sh)
      }
      Some(TestSuite::E2e) => {
        if coverage {
          bail!(
            "--coverage is not supported for e2e tests (LLVM instrument-coverage \
                 does not target wasm32)"
          );
        }
        test_e2e(&mut sh)
      }
      None => {
        if coverage {
          test_unit_with_coverage(&sh, coverage_output.as_deref())?;
        } else {
          test_unit(&sh)?;
        }
        test_wasm(&sh)?;
        test_e2e(&mut sh)
      }
    },
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

  // typewire-schema's `encode` and `decode` features are mutually exclusive,
  // so we lint each meaningful feature combination separately.

  // Workspace with default features (typewire[derive], typewire-schema[encode]).
  cmd!(sh, "cargo clippy --tests {args...}").run_echo()?;

  // typewire-schema: no features (coded only).
  cmd!(sh, "cargo clippy -p typewire-schema --tests --no-default-features {args...}").run_echo()?;

  // typewire-schema: encode path.
  cmd!(sh, "cargo clippy -p typewire-schema --tests --features encode {args...}").run_echo()?;

  // typewire-schema: typescript path (decode, no encode).
  cmd!(sh, "cargo clippy -p typewire-schema --tests --features typescript {args...}").run_echo()?;

  // typewire: no features (no derive, no optional deps).
  cmd!(sh, "cargo clippy -p typewire --no-default-features {args...}").run_echo()?;

  // typewire: all optional type features.
  let type_features = TYPE_FEATURES;
  cmd!(sh, "cargo clippy -p typewire --tests --features {type_features} {args...}").run_echo()?;

  // typewire: cli feature without derive (codegen/typescript path).
  cmd!(sh, "cargo clippy -p typewire --no-default-features --features cli {args...}").run_echo()?;

  // typewire-derive.
  cmd!(sh, "cargo clippy -p typewire-derive --tests {args...}").run_echo()?;

  // wasm32: typewire + examples (default features).
  cmd!(sh, "cargo clippy -p typewire -p todo-app --target {WASM_TARGET} {args...}").run_echo()?;

  // wasm32: typewire with all optional type features.
  cmd!(sh, "cargo clippy -p typewire --target {WASM_TARGET} --features {type_features} {args...}")
    .run_echo()?;

  fmt(sh, !fix)
}

// ---------------------------------------------------------------------------
// Documentation
// ---------------------------------------------------------------------------

fn doc(sh: &Shell) -> Result<()> {
  let type_features = TYPE_FEATURES;
  let typewire_features = format!("derive,schemas,{type_features}");
  cmd!(sh, "cargo doc --no-deps -p typewire --features {typewire_features}").run_echo()?;
  cmd!(sh, "cargo doc --no-deps -p typewire-derive --all-features").run_echo()?;
  cmd!(sh, "cargo doc --no-deps -p typewire-schema --features typescript").run_echo()?;
  Ok(())
}

// ---------------------------------------------------------------------------
// Test suites
// ---------------------------------------------------------------------------

fn test_unit(sh: &Shell) -> Result<()> {
  cmd!(sh, "cargo test --all").run_echo()?;
  // Schema roundtrip tests need the typescript feature (separate build to avoid
  // encode+decode feature conflict).
  cmd!(sh, "cargo test -p typewire-schema --features typescript").run_echo()?;
  Ok(())
}

fn test_wasm(sh: &Shell) -> Result<()> {
  cmd!(sh, "cargo test -p typewire --target {WASM_TARGET}").run_echo()?;
  Ok(())
}

fn test_e2e(sh: &mut Shell) -> Result<()> {
  // Build wasm.
  cmd!(sh, "cargo build -p todo-app --target {WASM_TARGET} --release").run_echo()?;

  // Generate TypeScript (strips the section by default) and diff against
  // the checked-in snapshot.
  cmd!(
    sh,
    "cargo run -p typewire --features cli -- target/{WASM_TARGET}/release/todo_app.wasm -o examples/todo-app/types.gen.d.ts"
  )
  .run_echo()?;
  cmd!(sh, "diff examples/todo-app/types.d.ts examples/todo-app/types.gen.d.ts").run_echo()?;

  // Assert the typewire_schemas section was stripped from the binary.
  let stripped_output = cmd!(
    sh,
    "cargo run -p typewire --features cli -- target/{WASM_TARGET}/release/todo_app.wasm -o /dev/null"
  )
  .ignore_status()
  .output()?;
  assert!(
    !stripped_output.status.success(),
    "typewire_schemas section should have been stripped, but CLI succeeded"
  );
  let stderr = String::from_utf8_lossy(&stripped_output.stderr);
  assert!(
    stderr.contains("section") || stderr.contains("not found") || stderr.contains("no schema"),
    "expected section-not-found error, got: {stderr}"
  );

  // Generate JS bindings.
  cmd!(
    sh,
    "wasm-bindgen target/{WASM_TARGET}/release/todo_app.wasm --out-dir examples/todo-app/pkg --target nodejs"
  )
  .run_echo()?;

  // Type-check and run.
  sh.set_current_dir(ROOT.join("examples/todo-app"));
  cmd!(sh, "npm install --prefer-offline").run_echo()?;
  cmd!(sh, "npx tsc --noEmit").run_echo()?;
  cmd!(sh, "npx tsx test.ts").run_echo()?;

  Ok(())
}

// ---------------------------------------------------------------------------
// Coverage
// ---------------------------------------------------------------------------

/// Per-crate coverage result.
#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct CrateCoverage {
  name: String,
  covered: u64,
  total: u64,
  percent: f64,
}

/// Run unit tests under cargo-llvm-cov and produce per-crate coverage reports.
///
/// Coverage uses LLVM instrument-coverage which only supports native targets,
/// so wasm and e2e tests are excluded.
fn test_unit_with_coverage(sh: &Shell, output_path: Option<&std::path::Path>) -> Result<()> {
  // Clean previous coverage data to avoid stale profiles.
  cmd!(sh, "cargo llvm-cov clean --workspace").run_echo()?;

  // Run workspace tests under coverage (no report yet).
  cmd!(sh, "cargo llvm-cov --no-report --all").run_echo()?;

  // Run typewire-schema typescript feature tests under coverage too
  // (separate invocation for the mutually exclusive feature).
  cmd!(sh, "cargo llvm-cov --no-report -p typewire-schema --features typescript").run_echo()?;

  // Collect per-crate coverage by generating a JSON report for each crate.
  let mut results = Vec::new();
  for &krate in COVERAGE_CRATES {
    let json_str =
      cmd!(sh, "cargo llvm-cov report --json --package {krate} --summary-only").read()?;
    let summary = parse_llvm_cov_json(&json_str, krate)?;
    results.push(summary);
  }

  // Print human-readable summary.
  println!();
  println!("=== Coverage Summary ===");
  for r in &results {
    println!("  {}: {:.1}% ({}/{} lines)", r.name, r.percent, r.covered, r.total);
  }
  println!();

  // Write machine-readable JSON output.
  let json_output = serde_json::to_string_pretty(&results)?;
  if let Some(path) = output_path {
    std::fs::write(path, &json_output)?;
    println!("Coverage JSON written to {}", path.display());
  } else {
    let default_path = ROOT.join("target/coverage.json");
    std::fs::write(&default_path, &json_output)?;
    println!("Coverage JSON written to {}", default_path.display());
  }

  Ok(())
}

/// Parse the JSON output from `cargo llvm-cov report --json --summary-only`
/// and extract line coverage for a given crate.
fn parse_llvm_cov_json(json_str: &str, crate_name: &str) -> Result<CrateCoverage> {
  let v: serde_json::Value = serde_json::from_str(json_str)?;

  // The JSON format has `data[0].totals.lines.{count, covered, percent}`.
  let totals = &v["data"][0]["totals"]["lines"];
  let total = totals["count"].as_u64().unwrap_or(0);
  let covered = totals["covered"].as_u64().unwrap_or(0);
  let percent = totals["percent"].as_f64().unwrap_or(0.0);

  Ok(CrateCoverage { name: crate_name.to_string(), covered, total, percent })
}
