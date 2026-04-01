use std::{collections::BTreeMap, io::Write, path::Path};

use anyhow::Result;
use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use xshell::{Shell, cmd};

const WASM_TARGET: &str = "wasm32-unknown-unknown";

bitflags! {
  /// Selects which benchmark suites to run.
  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub struct BenchKind: u8 {
    /// Bundle size comparison (raw + gzip).
    const SIZE = 1;
    /// Performance benchmarks (wasm, requires Node.js).
    const PERF = 1 << 1;
  }
}

/// Machine-readable benchmark results.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct BenchResults {
  /// Bundle sizes in bytes, keyed by "crate.metric" (e.g. "typewire.gz").
  #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
  size: BTreeMap<String, u64>,
  /// Perf metrics in microseconds per operation, keyed by benchmark name.
  /// Uses `serde_json::Value` (with `preserve_order`) to keep insertion order.
  #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
  perf: serde_json::Value,
}

struct SizeEntry {
  label: &'static str,
  crate_key: &'static str,
  raw: u64,
  gz: u64,
}

#[expect(clippy::cast_precision_loss, reason = "approximate display of byte counts")]
fn fmt_bytes(bytes: u64) -> String {
  if bytes >= 1_048_576 {
    format!("{:.1}M", bytes as f64 / 1_048_576.0)
  } else if bytes >= 1024 {
    format!("{:.1}K", bytes as f64 / 1024.0)
  } else {
    format!("{bytes}B")
  }
}

fn gzip_size(path: &Path) -> Result<u64> {
  let data = std::fs::read(path)?;
  let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
  encoder.write_all(&data)?;
  let compressed = encoder.finish()?;
  Ok(compressed.len() as u64)
}

/// Size benchmark features for the unified `bench-wasm` crate.
const SIZE_FEATURES: [(&str, &str, &str); 4] = [
  ("size-baseline", "baseline (wasm-bindgen only)", "baseline"),
  ("size-typewire", "typewire", "typewire"),
  ("size-serde-wasm-bindgen", "serde-wasm-bindgen", "serde_wasm_bindgen"),
  ("size-serde-json", "serde_json", "serde_json"),
];

fn collect_sizes(sh: &Shell, root: &Path) -> Result<Vec<SizeEntry>> {
  let mut entries = Vec::new();
  let pkg_dir = root.join("benches/wasm/pkg");

  for (feature, label, crate_key) in &SIZE_FEATURES {
    // Build with exactly one size feature to isolate bundle overhead.
    cmd!(
      sh,
      "cargo build -p bench-wasm --target {WASM_TARGET} --release --no-default-features --features {feature}"
    )
    .run_echo()?;

    let wasm_path = root.join(format!("target/{WASM_TARGET}/release/bench_wasm.wasm"));
    anyhow::ensure!(wasm_path.exists(), "{} not found", wasm_path.display());

    // Strip wasm-bindgen metadata (describe functions + custom section).
    let bg_wasm =
      crate::wasm::bindgen(sh, &wasm_path, &pkg_dir, crate::wasm::BindgenFlags::OPTIMIZE)?;

    let raw = std::fs::metadata(&bg_wasm)?.len();
    let gz = gzip_size(&bg_wasm)?;
    entries.push(SizeEntry { label, crate_key, raw, gz });
  }

  Ok(entries)
}

// `entries` is always non-empty (populated from the `SIZE_FEATURES` const).
fn print_size_tables(entries: &[SizeEntry]) {
  println!("{:<30} {:>10} {:>10}", "Crate", "Raw", "Gzip");
  println!("{:<30} {:>10} {:>10}", "-----", "---", "----");
  for entry in entries {
    println!("{:<30} {:>10} {:>10}", entry.label, fmt_bytes(entry.raw), fmt_bytes(entry.gz));
  }

  println!();
  println!("--- Delta from baseline ---");
  println!();
  println!("{:<30} {:>10} {:>10}", "Crate", "Raw delta", "Gzip delta");
  println!("{:<30} {:>10} {:>10}", "-----", "---------", "----------");

  let baseline_raw = entries[0].raw;
  let baseline_gz = entries[0].gz;
  for (i, entry) in entries.iter().enumerate() {
    if i == 0 {
      println!("{:<30} {:>10} {:>10}", entry.label, "(baseline)", "(baseline)");
    } else {
      let raw_delta = entry.raw.saturating_sub(baseline_raw);
      let gz_delta = entry.gz.saturating_sub(baseline_gz);
      println!(
        "{:<30} {:>10} {:>10}",
        entry.label,
        format!("+{}", fmt_bytes(raw_delta)),
        format!("+{}", fmt_bytes(gz_delta)),
      );
    }
  }
}

fn run_size(sh: &Shell, root: &Path, results: &mut BenchResults, json: bool) -> Result<()> {
  let entries = collect_sizes(sh, root)?;

  for entry in &entries {
    results.size.insert(format!("{}.raw", entry.crate_key), entry.raw);
    results.size.insert(format!("{}.gz", entry.crate_key), entry.gz);
  }

  if !json {
    print_size_tables(&entries);
  }

  Ok(())
}

const PERF_LIBS: [&str; 3] = ["typewire", "serde_wasm_bindgen", "serde_json"];

fn fmt_us(us: f64) -> String {
  if us < 1.0 {
    format!("{:.0} ns/op", us * 1000.0)
  } else if us < 1000.0 {
    format!("{us:.1} us/op")
  } else {
    format!("{:.1} ms/op", us / 1000.0)
  }
}

fn print_perf_table(perf: &serde_json::Value) {
  let Some(map) = perf.as_object() else {
    return;
  };

  // Group entries by benchmark name, preserving insertion order.
  let mut groups: Vec<(String, Vec<(&str, f64)>)> = Vec::new();
  let mut group_idx: BTreeMap<String, usize> = BTreeMap::new();
  for (key, val) in map {
    let Some(us) = val.as_f64() else {
      continue;
    };
    let dot = key.rfind('.').unwrap_or(0);
    let bench = &key[..dot];
    let lib = &key[dot + 1..];
    if let Some(&idx) = group_idx.get(bench) {
      groups[idx].1.push((lib, us));
    } else {
      group_idx.insert(bench.to_owned(), groups.len());
      groups.push((bench.to_owned(), vec![(lib, us)]));
    }
  }

  let col_w = 20;
  println!("{}", "=".repeat(76));
  println!("  Performance Comparison: typewire vs serde-wasm-bindgen vs serde_json");
  println!("  Round-trip: serialize \u{2192} cross JS boundary \u{2192} deserialize");
  println!("{}", "=".repeat(76));
  println!();
  println!("{}", "-".repeat(76));
  print!("{:<22}", "Benchmark");
  for lib in &PERF_LIBS {
    print!("{lib:<col_w$}");
  }
  println!("Fastest");
  println!("{}", "-".repeat(76));

  for (name, entries) in &groups {
    print!("{name:<22}");
    let mut fastest: Option<(&str, f64)> = None;
    let mut slowest_us: f64 = 0.0;
    for lib in &PERF_LIBS {
      let us = entries.iter().find(|(l, _)| l == lib).map(|(_, v)| *v);
      match us {
        Some(v) => {
          print!("{:<col_w$}", fmt_us(v));
          if fastest.is_none() || v < fastest.unwrap().1 {
            fastest = Some((lib, v));
          }
          if v > slowest_us {
            slowest_us = v;
          }
        }
        None => print!("{:<col_w$}", "n/a"),
      }
    }
    if let Some((lib, fast_us)) = fastest {
      let ratio = slowest_us / fast_us;
      println!("{lib} {ratio:.2}x");
    } else {
      println!();
    }
  }
  println!("{}", "-".repeat(76));
  println!();
}

fn run_perf(sh: &Shell, root: &Path, results: &mut BenchResults, json: bool) -> Result<()> {
  // Build with the perf feature to get all serialization libraries.
  cmd!(
    sh,
    "cargo build -p bench-wasm --target {WASM_TARGET} --release --no-default-features --features perf"
  )
  .run_echo()?;

  // Generate JS bindings.
  let wasm_path = root.join(format!("target/{WASM_TARGET}/release/bench_wasm.wasm"));
  let pkg_dir = root.join("benches/wasm/pkg");
  crate::wasm::bindgen(
    sh,
    &wasm_path,
    &pkg_dir,
    crate::wasm::BindgenFlags::NODEJS | crate::wasm::BindgenFlags::TYPESCRIPT,
  )?;

  // Run benchmarks once, always collecting JSON.
  let output = cmd!(sh, "node benches/wasm/run.js --json").output()?;
  let json_str = String::from_utf8(output.stdout)?;
  results.perf = serde_json::from_str(&json_str)?;

  if !json {
    print_perf_table(&results.perf);
  }

  Ok(())
}

fn generate_chart(sh: &Shell, root: &Path, results: &BenchResults) -> Result<()> {
  let bench_dir = root.join("benches/wasm");
  cmd!(sh, "npm install --prefer-offline --prefix {bench_dir}").run_echo()?;

  let json_path = root.join("target/bench.json");
  std::fs::write(&json_path, serde_json::to_string(results)?)?;

  let chart_path = root.join("benches/wasm.svg");
  let svg = cmd!(sh, "node benches/wasm/chart.js {json_path}").output()?;
  std::fs::write(&chart_path, &svg.stdout)?;

  println!();
  println!("Chart written to benches/wasm.svg");
  Ok(())
}

/// Run the selected benchmark suites.
///
/// When running all suites without `--json`, the chart SVG is regenerated
/// automatically from the results.
pub fn bench(sh: &Shell, root: &Path, kind: BenchKind, json: bool) -> Result<()> {
  let mut results = BenchResults::default();

  if kind.contains(BenchKind::SIZE) {
    run_size(sh, root, &mut results, json)?;
    if !json && kind.contains(BenchKind::PERF) {
      println!();
    }
  }

  if kind.contains(BenchKind::PERF) {
    run_perf(sh, root, &mut results, json)?;
  }

  if json {
    println!("{}", serde_json::to_string(&results)?);
  }

  // Regenerate chart when running all suites interactively.
  if kind == BenchKind::all() && !json {
    generate_chart(sh, root, &results)?;
  }

  Ok(())
}

/// Size regression threshold: fail if any size metric worsened by more than 1.5%.
const SIZE_THRESHOLD: f64 = 1.015;

/// Compare two bench JSON files and fail if size regressions exceed the threshold.
///
/// Perf metrics are reported for information only (too noisy on shared CI runners).
#[expect(clippy::cast_precision_loss, reason = "approximate display of regression ratios")]
pub fn check(current_path: &Path, parent_path: &Path) -> Result<()> {
  let current: BenchResults = serde_json::from_str(&std::fs::read_to_string(current_path)?)?;
  let parent: BenchResults = serde_json::from_str(&std::fs::read_to_string(parent_path)?)?;

  let mut failed = false;

  // Size metrics are deterministic -- fail on >1.5% regression.
  for (key, &cur) in &current.size {
    let Some(&prev) = parent.size.get(key) else { continue };
    if prev == 0 {
      continue;
    }
    let ratio = cur as f64 / prev as f64;
    let status = if ratio > SIZE_THRESHOLD {
      failed = true;
      "FAIL"
    } else {
      "ok"
    };
    println!("  size/{key}: {prev} -> {cur} ({ratio:.4}x) [{status}]");
  }

  // Perf metrics are noisy on shared CI runners -- report only, no failure.
  if let (Some(cur_map), Some(par_map)) = (current.perf.as_object(), parent.perf.as_object()) {
    for (key, cur_val) in cur_map {
      let Some(cur) = cur_val.as_f64() else { continue };
      let Some(prev) = par_map.get(key).and_then(serde_json::Value::as_f64) else {
        continue;
      };
      if prev <= 0.0 {
        continue;
      }
      let ratio = cur / prev;
      println!("  perf/{key}: {prev:.2} -> {cur:.2} us/op ({ratio:.4}x) [info]");
    }
  }

  if failed {
    println!();
    anyhow::bail!("REGRESSION DETECTED: one or more size metrics worsened by >1.5%");
  }
  println!();
  println!("No regressions detected.");
  Ok(())
}
