use std::{path::PathBuf, sync::LazyLock};

use anyhow::{Result, bail};
use bitflags::bitflags;
use clap::{Parser, Subcommand, ValueEnum};
use xshell::{Shell, cmd};

bitflags! {
  /// Which test suites to instrument for code coverage.
  #[derive(Clone, Copy, Debug)]
  struct CoverageMode: u8 {
    /// Native unit + integration tests.
    const UNIT = 0b01;
    /// wasm32 tests via `wasm-bindgen-test` (nightly).
    const WASM = 0b10;
  }
}

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
    /// Collect code coverage via cargo-llvm-cov (unit + wasm)
    #[arg(long)]
    coverage: bool,
    /// Write per-crate coverage JSON to this path
    #[arg(long, value_name = "PATH")]
    coverage_output: Option<PathBuf>,
  },
  /// Check coverage delta against a parent commit's git note
  CoverageDelta {
    /// Path to the current coverage JSON file
    #[arg(value_name = "COVERAGE_JSON")]
    coverage_json: PathBuf,
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
          test_with_coverage(&sh, CoverageMode::UNIT, coverage_output.as_deref())
        } else {
          test_unit(&sh)
        }
      }
      Some(TestSuite::Wasm) => {
        if coverage {
          test_with_coverage(&sh, CoverageMode::WASM, coverage_output.as_deref())
        } else {
          test_wasm(&sh)
        }
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
          // Combined coverage runs unit + wasm under instrumentation.
          // E2e is skipped because it doesn't support coverage and is
          // already validated by the main CI workflow.
          test_with_coverage(&sh, CoverageMode::all(), coverage_output.as_deref())
        } else {
          test_unit(&sh)?;
          test_wasm(&sh)?;
          test_e2e(&mut sh)
        }
      }
    },
    Command::CoverageDelta { coverage_json } => coverage_delta(&sh, &coverage_json),
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

/// Wasm-specific RUSTFLAGS for coverage instrumentation via `wasm-bindgen-test`.
///
/// Requires nightly >= 1.87.0 and wasm-bindgen-test >= 0.3.57.
const WASM_COV_RUSTFLAGS: &str = "-Cinstrument-coverage -Zno-profiler-runtime \
  -Clink-args=--no-gc-sections --cfg=wasm_bindgen_unstable_test_coverage";

/// Run tests under `cargo-llvm-cov` and produce per-crate coverage reports.
///
/// `mode` selects which suites to instrument: `CoverageMode::UNIT` adds native
/// workspace tests (including the `typewire-schema` typescript-feature pass),
/// and `CoverageMode::WASM` adds wasm32 tests under nightly with
/// `wasm-bindgen-test`'s experimental coverage.
fn test_with_coverage(
  sh: &Shell,
  mode: CoverageMode,
  output_path: Option<&std::path::Path>,
) -> Result<()> {
  cmd!(sh, "cargo llvm-cov clean --workspace").run_echo()?;

  if mode.contains(CoverageMode::UNIT) {
    cmd!(sh, "cargo llvm-cov --no-report --all").run_echo()?;
    // typewire-schema typescript feature tests (mutually exclusive with encode).
    cmd!(sh, "cargo llvm-cov --no-report -p typewire-schema --features typescript").run_echo()?;
  }

  if mode.contains(CoverageMode::WASM) {
    let wasm_rustflags = WASM_COV_RUSTFLAGS;
    cmd!(sh, "cargo +nightly llvm-cov --no-report -p typewire --target {WASM_TARGET}")
      .env("CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUNNER", "wasm-bindgen-test-runner")
      .env("CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS", wasm_rustflags)
      .run_echo()?;
  }

  // Collect per-crate coverage from all accumulated profdata.
  let mut results = Vec::new();
  for &krate in COVERAGE_CRATES {
    let json_str =
      cmd!(sh, "cargo llvm-cov report --json --package {krate} --summary-only").read()?;
    let summary = parse_llvm_cov_json(&json_str, krate)?;
    results.push(summary);
  }

  println!();
  println!("=== Coverage Summary ===");
  for r in &results {
    println!("  {}: {:.1}% ({}/{} lines)", r.name, r.percent, r.covered, r.total);
  }
  println!();

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

// ---------------------------------------------------------------------------
// Coverage delta
// ---------------------------------------------------------------------------

/// Maximum allowed coverage regression (in percentage points).
const MAX_REGRESSION: f64 = 1.0;

/// Compare current coverage against the parent commit's git note.
///
/// Exits with an error if any crate's line coverage drops by more than
/// `MAX_REGRESSION` percentage points.
fn coverage_delta(sh: &Shell, coverage_json: &std::path::Path) -> Result<()> {
  let contents = std::fs::read_to_string(coverage_json)?;
  let current: Vec<CrateCoverage> = serde_json::from_str(&contents)?;
  let current_map: std::collections::BTreeMap<&str, f64> =
    current.iter().map(|c| (c.name.as_str(), c.percent)).collect();

  let Some(parent) = get_parent_coverage(sh) else {
    println!("No parent coverage note found -- skipping delta check.");
    return Ok(());
  };

  println!();
  println!("{:<25} {:>8} {:>8} {:>8}  Status", "Crate", "Old", "New", "Delta");
  println!("{}", "-".repeat(62));

  let mut failed = false;
  for name in current_map.keys().copied() {
    let new_pct = current_map[name];
    let Some(&old_pct) = parent.get(name) else {
      println!("{name:<25} {:>8} {new_pct:>7.1}% {:>8}  new", "N/A", "");
      continue;
    };
    let delta = new_pct - old_pct;
    let status = if old_pct - new_pct > MAX_REGRESSION {
      failed = true;
      "FAIL (>1% regression)"
    } else if delta < 0.0 {
      "warn"
    } else {
      "ok"
    };
    println!("{name:<25} {old_pct:>7.1}% {new_pct:>7.1}% {delta:>+7.1}%  {status}");
  }

  println!();

  if failed {
    bail!("Coverage regression exceeds {MAX_REGRESSION:.1} percentage point threshold.");
  }
  println!("Coverage delta check passed.");
  Ok(())
}

/// Read coverage percentages from the parent commit's git note.
///
/// Determines the comparison base using `git merge-base` (for PRs) or
/// `HEAD~1` (for pushes to main). Returns `None` when no note exists
/// (first run).
fn get_parent_coverage(sh: &Shell) -> Option<std::collections::HashMap<String, f64>> {
  // Determine the comparison base.
  let base_sha = cmd!(sh, "git merge-base HEAD origin/main")
    .ignore_status()
    .read()
    .ok()
    .filter(|s| !s.is_empty())
    .or_else(|| {
      cmd!(sh, "git rev-parse HEAD~1").ignore_status().read().ok().filter(|s| !s.is_empty())
    })?;

  // Fetch notes ref (may not exist yet).
  let _ =
    cmd!(sh, "git fetch origin refs/notes/coverage:refs/notes/coverage").ignore_status().output();

  // Read the note attached to the base commit.
  let note = cmd!(sh, "git notes --ref=coverage show {base_sha}")
    .ignore_status()
    .read()
    .ok()
    .filter(|s| !s.is_empty())?;

  parse_coverage_note(&note)
}

/// Parse a coverage note body into a map of crate name to percentage.
///
/// Each line has the format `crate-name: 85.2%`. Blank lines are
/// skipped. Returns `None` when no valid entries are found.
fn parse_coverage_note(note: &str) -> Option<std::collections::HashMap<String, f64>> {
  let mut result = std::collections::HashMap::new();
  for line in note.lines() {
    let line = line.trim();
    if line.is_empty() {
      continue;
    }
    // Format: "crate-name: 85.2%"
    let (name, pct) = line.split_once(':')?;
    let pct = pct.trim().strip_suffix('%')?.trim();
    result.insert(name.trim().to_string(), pct.parse::<f64>().ok()?);
  }

  if result.is_empty() { None } else { Some(result) }
}

#[cfg(test)]
mod tests {
  use super::*;

  // -----------------------------------------------------------------------
  // parse_coverage_note
  // -----------------------------------------------------------------------

  #[test]
  fn parse_coverage_note_typical() {
    let note = "typewire: 85.2%\ntypewire-derive: 72.1%\ntypewire-schema: 90.3%\n";
    let map = parse_coverage_note(note).unwrap();
    assert_eq!(map.len(), 3);
    assert!((map["typewire"] - 85.2).abs() < f64::EPSILON);
    assert!((map["typewire-derive"] - 72.1).abs() < f64::EPSILON);
    assert!((map["typewire-schema"] - 90.3).abs() < f64::EPSILON);
  }

  #[test]
  fn parse_coverage_note_blank_lines() {
    let note = "\n  typewire: 50.0%  \n\n  typewire-derive: 60.0%  \n\n";
    let map = parse_coverage_note(note).unwrap();
    assert_eq!(map.len(), 2);
    assert!((map["typewire"] - 50.0).abs() < f64::EPSILON);
    assert!((map["typewire-derive"] - 60.0).abs() < f64::EPSILON);
  }

  #[test]
  fn parse_coverage_note_empty() {
    assert!(parse_coverage_note("").is_none());
    assert!(parse_coverage_note("  \n  \n").is_none());
  }

  #[test]
  fn parse_coverage_note_malformed_returns_none() {
    // No colon separator.
    assert!(parse_coverage_note("typewire 85.2%").is_none());
    // Missing percent sign.
    assert!(parse_coverage_note("typewire: 85.2").is_none());
  }

  // -----------------------------------------------------------------------
  // parse_llvm_cov_json
  // -----------------------------------------------------------------------

  #[test]
  fn parse_llvm_cov_json_extracts_lines() {
    let json = r#"{
      "data": [{
        "totals": {
          "lines": { "count": 200, "covered": 170, "percent": 85.0 }
        }
      }]
    }"#;
    let cov = parse_llvm_cov_json(json, "my-crate").unwrap();
    assert_eq!(cov.name, "my-crate");
    assert_eq!(cov.total, 200);
    assert_eq!(cov.covered, 170);
    assert!((cov.percent - 85.0).abs() < f64::EPSILON);
  }

  #[test]
  fn parse_llvm_cov_json_missing_fields_defaults_to_zero() {
    let json = r#"{"data": [{"totals": {"lines": {}}}]}"#;
    let cov = parse_llvm_cov_json(json, "empty").unwrap();
    assert_eq!(cov.total, 0);
    assert_eq!(cov.covered, 0);
    assert!((cov.percent - 0.0).abs() < f64::EPSILON);
  }

  // -----------------------------------------------------------------------
  // coverage_delta (using temp git repos)
  // -----------------------------------------------------------------------

  /// Create a temporary git repo with initial and child commits, attaching
  /// a coverage note to the initial commit. Returns `(Shell, temp_dir)`.
  fn setup_git_repo_with_note(note_body: &str) -> (Shell, tempfile::TempDir) {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let mut sh = Shell::new().expect("shell");
    sh.set_current_dir(tmp.path());

    // Initialise repo with a commit.
    xshell::cmd!(sh, "git init").run().expect("git init");
    xshell::cmd!(sh, "git config user.email test@test.com").run().unwrap();
    xshell::cmd!(sh, "git config user.name Test").run().unwrap();
    std::fs::write(tmp.path().join("file.txt"), "a").unwrap();
    xshell::cmd!(sh, "git add .").run().unwrap();
    xshell::cmd!(sh, "git commit -m initial").run().unwrap();

    // Attach a coverage note to the initial commit.
    xshell::cmd!(sh, "git notes --ref=coverage add -m {note_body} HEAD").run().unwrap();

    // Create a child commit (simulating a PR).
    std::fs::write(tmp.path().join("file.txt"), "b").unwrap();
    xshell::cmd!(sh, "git add .").run().unwrap();
    xshell::cmd!(sh, "git commit -m child").run().unwrap();

    (sh, tmp)
  }

  #[test]
  fn coverage_delta_passes_when_no_regression() {
    let note = "typewire: 80.0%\ntypewire-derive: 70.0%\ntypewire-schema: 90.0%";
    let (sh, tmp) = setup_git_repo_with_note(note);

    // Write a "current" coverage JSON that's equal or better.
    let current = serde_json::to_string(&vec![
      CrateCoverage { name: "typewire".into(), covered: 82, total: 100, percent: 82.0 },
      CrateCoverage { name: "typewire-derive".into(), covered: 70, total: 100, percent: 70.0 },
      CrateCoverage { name: "typewire-schema".into(), covered: 91, total: 100, percent: 91.0 },
    ])
    .unwrap();
    let json_path = tmp.path().join("coverage.json");
    std::fs::write(&json_path, current).unwrap();

    // Should succeed -- no regression.
    coverage_delta(&sh, &json_path).expect("should pass with no regression");
  }

  #[test]
  fn coverage_delta_fails_on_large_regression() {
    let note = "typewire: 80.0%\ntypewire-derive: 70.0%\ntypewire-schema: 90.0%";
    let (sh, tmp) = setup_git_repo_with_note(note);

    // Write a "current" coverage JSON with >1% regression in typewire.
    let current = serde_json::to_string(&vec![
      CrateCoverage { name: "typewire".into(), covered: 78, total: 100, percent: 78.0 },
      CrateCoverage { name: "typewire-derive".into(), covered: 70, total: 100, percent: 70.0 },
      CrateCoverage { name: "typewire-schema".into(), covered: 91, total: 100, percent: 91.0 },
    ])
    .unwrap();
    let json_path = tmp.path().join("coverage.json");
    std::fs::write(&json_path, current).unwrap();

    // Should fail -- typewire regressed by 2%.
    let result = coverage_delta(&sh, &json_path);
    assert!(result.is_err(), "should fail with >1% regression");
    let msg = result.unwrap_err().to_string();
    assert!(msg.contains("regression"), "error message should mention regression, got: {msg}");
  }

  #[test]
  fn coverage_delta_allows_small_regression() {
    let note = "typewire: 80.0%\ntypewire-derive: 70.0%\ntypewire-schema: 90.0%";
    let (sh, tmp) = setup_git_repo_with_note(note);

    // Write a "current" coverage JSON with exactly 1% regression (within threshold).
    let current = serde_json::to_string(&vec![
      CrateCoverage { name: "typewire".into(), covered: 79, total: 100, percent: 79.0 },
      CrateCoverage { name: "typewire-derive".into(), covered: 70, total: 100, percent: 70.0 },
      CrateCoverage { name: "typewire-schema".into(), covered: 90, total: 100, percent: 90.0 },
    ])
    .unwrap();
    let json_path = tmp.path().join("coverage.json");
    std::fs::write(&json_path, current).unwrap();

    // Should pass -- exactly 1.0% regression is the threshold (>1.0 fails).
    coverage_delta(&sh, &json_path).expect("should pass with exactly 1% regression");
  }

  #[test]
  fn coverage_delta_skips_when_no_parent_note() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let mut sh = Shell::new().expect("shell");
    sh.set_current_dir(tmp.path());

    // Initialise repo with two commits but no coverage note.
    xshell::cmd!(sh, "git init").run().expect("git init");
    xshell::cmd!(sh, "git config user.email test@test.com").run().unwrap();
    xshell::cmd!(sh, "git config user.name Test").run().unwrap();
    std::fs::write(tmp.path().join("file.txt"), "a").unwrap();
    xshell::cmd!(sh, "git add .").run().unwrap();
    xshell::cmd!(sh, "git commit -m initial").run().unwrap();
    std::fs::write(tmp.path().join("file.txt"), "b").unwrap();
    xshell::cmd!(sh, "git add .").run().unwrap();
    xshell::cmd!(sh, "git commit -m child").run().unwrap();

    let current = serde_json::to_string(&vec![CrateCoverage {
      name: "typewire".into(),
      covered: 80,
      total: 100,
      percent: 80.0,
    }])
    .unwrap();
    let json_path = tmp.path().join("coverage.json");
    std::fs::write(&json_path, current).unwrap();

    // Should pass gracefully -- no parent note exists.
    coverage_delta(&sh, &json_path).expect("should skip delta check when no parent note");
  }

  #[test]
  fn coverage_delta_handles_new_crate() {
    let note = "typewire: 80.0%";
    let (sh, tmp) = setup_git_repo_with_note(note);

    // Current report includes a crate that wasn't in the parent note.
    let current = serde_json::to_string(&vec![
      CrateCoverage { name: "typewire".into(), covered: 80, total: 100, percent: 80.0 },
      CrateCoverage { name: "typewire-new".into(), covered: 50, total: 100, percent: 50.0 },
    ])
    .unwrap();
    let json_path = tmp.path().join("coverage.json");
    std::fs::write(&json_path, current).unwrap();

    // Should pass -- new crates don't have a baseline to regress against.
    coverage_delta(&sh, &json_path).expect("should pass with a new crate");
  }

  // -----------------------------------------------------------------------
  // get_parent_coverage (git notes round-trip in a temp repo)
  // -----------------------------------------------------------------------

  #[test]
  fn get_parent_coverage_reads_note_from_parent() {
    let note = "typewire: 85.2%\ntypewire-derive: 72.1%";
    let (sh, _tmp) = setup_git_repo_with_note(note);

    let map = get_parent_coverage(&sh).expect("should read parent note");
    assert_eq!(map.len(), 2);
    assert!((map["typewire"] - 85.2).abs() < f64::EPSILON);
    assert!((map["typewire-derive"] - 72.1).abs() < f64::EPSILON);
  }

  #[test]
  fn get_parent_coverage_returns_none_without_note() {
    let tmp = tempfile::tempdir().expect("failed to create temp dir");
    let mut sh = Shell::new().expect("shell");
    sh.set_current_dir(tmp.path());

    xshell::cmd!(sh, "git init").run().expect("git init");
    xshell::cmd!(sh, "git config user.email test@test.com").run().unwrap();
    xshell::cmd!(sh, "git config user.name Test").run().unwrap();
    std::fs::write(tmp.path().join("file.txt"), "a").unwrap();
    xshell::cmd!(sh, "git add .").run().unwrap();
    xshell::cmd!(sh, "git commit -m initial").run().unwrap();
    std::fs::write(tmp.path().join("file.txt"), "b").unwrap();
    xshell::cmd!(sh, "git add .").run().unwrap();
    xshell::cmd!(sh, "git commit -m child").run().unwrap();

    assert!(get_parent_coverage(&sh).is_none());
  }

  // -----------------------------------------------------------------------
  // CoverageMode bitflags
  // -----------------------------------------------------------------------

  #[test]
  fn coverage_mode_flags() {
    let unit = CoverageMode::UNIT;
    let wasm = CoverageMode::WASM;
    let both = CoverageMode::UNIT | CoverageMode::WASM;

    assert!(unit.contains(CoverageMode::UNIT));
    assert!(!unit.contains(CoverageMode::WASM));
    assert!(both.contains(CoverageMode::UNIT));
    assert!(both.contains(CoverageMode::WASM));
    assert!(!wasm.contains(CoverageMode::UNIT));
    assert!(wasm.contains(CoverageMode::WASM));
  }
}
