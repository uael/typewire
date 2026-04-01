use std::collections::{BTreeMap, HashMap};

use anyhow::{Result, bail};
use xshell::{Shell, cmd};

use crate::{CoverageMode, ROOT, WASM_TARGET};

/// Crates to report coverage for.
const COVERAGE_CRATES: &[&str] = &["typewire", "typewire-derive", "typewire-schema"];

/// Wasm-specific RUSTFLAGS for coverage instrumentation via `wasm-bindgen-test`.
///
/// Requires nightly >= 1.87.0 and wasm-bindgen-test >= 0.3.57.
const WASM_COV_RUSTFLAGS: &str = "-Cinstrument-coverage -Zno-profiler-runtime \
  -Clink-args=--no-gc-sections --cfg=wasm_bindgen_unstable_test_coverage";

/// Maximum allowed coverage regression (in percentage points).
const MAX_REGRESSION: f64 = 1.0;

// ---------------------------------------------------------------------------
// Per-file coverage data
// ---------------------------------------------------------------------------

/// Line coverage for a single source file.
#[derive(Clone, Debug)]
struct FileCoverage {
  covered: u64,
  total: u64,
}

/// Per-crate coverage result.
///
/// The `files` map holds per-file line counts used for cross-target merging
/// but is excluded from the serialized JSON (only the aggregate numbers are
/// written to the coverage output).
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct CrateCoverage {
  name: String,
  covered: u64,
  total: u64,
  percent: f64,
  #[serde(skip)]
  files: BTreeMap<String, FileCoverage>,
}

// ---------------------------------------------------------------------------
// Test with coverage
// ---------------------------------------------------------------------------

/// Run tests under `cargo-llvm-cov` and produce per-crate coverage reports.
///
/// `mode` selects which suites to instrument: `CoverageMode::UNIT` adds native
/// workspace tests (including the `typewire-schema` typescript-feature pass),
/// and `CoverageMode::WASM` adds wasm32 tests under nightly with
/// `wasm-bindgen-test`'s experimental coverage.
pub fn test_with_coverage(
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
  //
  // The `typewire` crate compiles different code for native vs wasm32
  // (most impls are behind `#[cfg(target_arch = "wasm32")]`), so we
  // must collect reports for both targets and merge at the file level.
  let mut results = Vec::new();
  for &krate in COVERAGE_CRATES {
    let native = cmd!(sh, "cargo llvm-cov report --json --package {krate}").read()?;
    let mut summary = parse_llvm_cov_json(&native, krate)?;

    if mode.contains(CoverageMode::WASM) && krate == "typewire" {
      let wasm =
        cmd!(sh, "cargo llvm-cov report --json --package {krate} --target {WASM_TARGET}").read()?;
      let wasm_summary = parse_llvm_cov_json(&wasm, krate)?;
      summary = merge_coverage(summary, &wasm_summary);
    }

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

// ---------------------------------------------------------------------------
// Coverage delta
// ---------------------------------------------------------------------------

/// Compare current coverage against the parent commit's git note.
///
/// Exits with an error if any crate's line coverage drops by more than
/// `MAX_REGRESSION` percentage points.
pub fn coverage_delta(sh: &Shell, coverage_json: &std::path::Path) -> Result<()> {
  let contents = std::fs::read_to_string(coverage_json)?;
  let current: Vec<CrateCoverage> = serde_json::from_str(&contents)?;
  let current_map: BTreeMap<&str, f64> =
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

// ---------------------------------------------------------------------------
// JSON parsing
// ---------------------------------------------------------------------------

/// Parse the JSON output from `cargo llvm-cov report --json` and extract
/// per-file line coverage for a given crate.
fn parse_llvm_cov_json(json_str: &str, crate_name: &str) -> Result<CrateCoverage> {
  let v: serde_json::Value = serde_json::from_str(json_str)?;

  let mut files = BTreeMap::new();
  if let Some(file_array) = v["data"][0]["files"].as_array() {
    for file in file_array {
      if let Some(filename) = file["filename"].as_str() {
        let lines = &file["summary"]["lines"];
        let total = lines["count"].as_u64().unwrap_or(0);
        let covered = lines["covered"].as_u64().unwrap_or(0);
        files.insert(filename.to_string(), FileCoverage { covered, total });
      }
    }
  }

  let (covered, total) = aggregate(&files);
  let percent = compute_percent(covered, total);

  Ok(CrateCoverage { name: crate_name.to_string(), covered, total, percent, files })
}

// ---------------------------------------------------------------------------
// Merging
// ---------------------------------------------------------------------------

/// Merge coverage from two targets (native + wasm) for the same crate.
///
/// Files that appear in both reports have their line counts summed (they
/// represent disjoint `#[cfg]`-gated regions within the same source file).
/// Files that appear in only one report are taken as-is.
fn merge_coverage(a: CrateCoverage, b: &CrateCoverage) -> CrateCoverage {
  let mut merged = a.files;

  for (filename, b_file) in &b.files {
    merged
      .entry(filename.clone())
      .and_modify(|a_file| {
        a_file.covered += b_file.covered;
        a_file.total += b_file.total;
      })
      .or_insert_with(|| b_file.clone());
  }

  let (covered, total) = aggregate(&merged);
  let percent = compute_percent(covered, total);

  CrateCoverage { name: a.name, covered, total, percent, files: merged }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Sum covered/total across all files.
fn aggregate(files: &BTreeMap<String, FileCoverage>) -> (u64, u64) {
  files.values().fold((0, 0), |(c, t), f| (c + f.covered, t + f.total))
}

/// Compute coverage percentage from covered/total line counts.
#[expect(
  clippy::cast_precision_loss,
  reason = "line counts are small enough that f64 precision is fine"
)]
fn compute_percent(covered: u64, total: u64) -> f64 {
  if total > 0 { covered as f64 / total as f64 * 100.0 } else { 0.0 }
}

// ---------------------------------------------------------------------------
// Git notes
// ---------------------------------------------------------------------------

/// Read coverage percentages from the parent commit's git note.
///
/// Determines the comparison base using `git merge-base` (for PRs) or
/// `HEAD~1` (for pushes to main). Returns `None` when no note exists
/// (first run).
fn get_parent_coverage(sh: &Shell) -> Option<HashMap<String, f64>> {
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
fn parse_coverage_note(note: &str) -> Option<HashMap<String, f64>> {
  let mut result = HashMap::new();
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
