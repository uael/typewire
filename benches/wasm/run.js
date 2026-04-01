// Benchmark runner: loads the wasm module and prints comparison results.
//
// Usage:
//   cargo xtask bench perf           # human-readable table
//   cargo xtask bench perf --json    # machine-readable JSON
//
// Or manually:
//   cargo build -p bench-wasm --target wasm32-unknown-unknown --release
//   wasm-bindgen target/wasm32-unknown-unknown/release/bench_wasm.wasm \
//     --out-dir benches/wasm/pkg --target nodejs
//   node benches/wasm/run.js [--json]

const wasm = require("./pkg/bench_wasm.js");

const jsonMode = process.argv.includes("--json");

function formatUs(us) {
  if (us < 1) {
    return `${(us * 1000).toFixed(0)} ns/op`;
  }
  return `${us.toFixed(1)} us/op`;
}

function runBenchmark(name, fns, iterations) {
  const result = { name };
  const timings = {};

  for (const [label, fn] of Object.entries(fns)) {
    const ms = fn();
    const usPerOp = (ms / iterations) * 1000;
    result[label] = formatUs(usPerOp);
    timings[label] = usPerOp;
  }

  // Find the fastest
  const sorted = Object.entries(timings).sort((a, b) => a[1] - b[1]);
  const fastest = sorted[0];
  const slowest = sorted[sorted.length - 1];
  const ratio = slowest[1] / fastest[1];
  result.fastest = `${fastest[0]} ${ratio.toFixed(2)}x`;
  result.timings = timings;

  return result;
}

function main() {
  const iters = wasm.get_iterations();
  const collIters = wasm.get_collection_iterations();
  const collSize = wasm.get_collection_size();

  const results = [];

  // 1. Simple struct
  results.push(runBenchmark("simple_struct", {
    typewire: wasm.bench_simple_roundtrip_typewire,
    serde_wasm_bindgen: wasm.bench_simple_roundtrip_serde_wasm_bindgen,
    serde_json: wasm.bench_simple_roundtrip_serde_json,
  }, iters));

  // 2. Simple enum
  results.push(runBenchmark("simple_enum", {
    typewire: wasm.bench_enum_roundtrip_typewire,
    serde_wasm_bindgen: wasm.bench_enum_roundtrip_serde_wasm_bindgen,
    serde_json: wasm.bench_enum_roundtrip_serde_json,
  }, iters));

  // 3. Complex struct
  results.push(runBenchmark("complex", {
    typewire: wasm.bench_complex_roundtrip_typewire,
    serde_wasm_bindgen: wasm.bench_complex_roundtrip_serde_wasm_bindgen,
    serde_json: wasm.bench_complex_roundtrip_serde_json,
  }, iters));

  // 4. Complex collection
  results.push(runBenchmark(`vec_${collSize}_complex`, {
    typewire: wasm.bench_collection_roundtrip_typewire,
    serde_wasm_bindgen: wasm.bench_collection_roundtrip_serde_wasm_bindgen,
    serde_json: wasm.bench_collection_roundtrip_serde_json,
  }, collIters));

  if (jsonMode) {
    // Output machine-readable JSON: { "benchmark.lib": us }
    const obj = {};
    for (const r of results) {
      for (const [lib, us] of Object.entries(r.timings)) {
        obj[`${r.name}.${lib}`] = us;
      }
    }
    console.log(JSON.stringify(obj));
    return;
  }

  // Human-readable output
  const libs = ["typewire", "serde_wasm_bindgen", "serde_json"];
  const colW = 14;

  console.log("=".repeat(76));
  console.log("  Performance Comparison: typewire vs serde-wasm-bindgen vs serde_json");
  console.log("  Round-trip: serialize → cross JS boundary → deserialize");
  console.log("=".repeat(76));
  console.log();

  console.log("-".repeat(76));
  console.log(
    "Benchmark".padEnd(22),
    ...libs.map(l => l.padEnd(colW)),
    "Fastest",
  );
  console.log("-".repeat(76));
  for (const r of results) {
    console.log(
      r.name.padEnd(22),
      ...libs.map(l => (r[l] || "n/a").padEnd(colW)),
      r.fastest,
    );
  }
  console.log("-".repeat(76));
  console.log();
  console.log(`Iterations: ${iters} (collections: ${collIters})`);
  console.log("Times are wall-clock ms measured with performance.now() inside wasm.");
  console.log("Values cross the wasm-JS boundary via identity functions.");
  console.log();
}

main();
