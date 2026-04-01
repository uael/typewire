// Generate SVG charts from bench JSON results using d3.
//
// Usage:
//   cargo xtask bench --json | node benches/wasm/chart.js > benches/wasm.svg
//   node benches/wasm/chart.js bench.json > benches/wasm.svg

const fs = require("fs");
const d3 = require("d3");
const { JSDOM } = require("jsdom");

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

const PERF_LIBS = ["typewire", "serde_wasm_bindgen", "serde_json"];
const PERF_COLORS = { typewire: "#2563eb", serde_wasm_bindgen: "#dc2626", serde_json: "#16a34a" };

const SIZE_ORDER = ["baseline", "typewire", "serde_wasm_bindgen", "serde_json"];
const SIZE_COLORS = { baseline: "#94a3b8", typewire: "#2563eb", serde_wasm_bindgen: "#dc2626", serde_json: "#16a34a" };
const LABELS = { typewire: "typewire", serde_wasm_bindgen: "serde-wasm-bindgen", serde_json: "serde_json", baseline: "baseline" };

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function readInput() {
  const arg = process.argv[2];
  if (arg && arg !== "-") return JSON.parse(fs.readFileSync(arg, "utf-8"));
  return JSON.parse(fs.readFileSync("/dev/stdin", "utf-8"));
}

function formatUs(us) {
  if (us < 1) return `${(us * 1000).toFixed(0)} ns`;
  if (us < 1000) return `${us.toFixed(1)} µs`;
  return `${(us / 1000).toFixed(1)} ms`;
}

function formatBytes(b) {
  if (b >= 1048576) return `${(b / 1048576).toFixed(1)} MB`;
  if (b >= 1024) return `${(b / 1024).toFixed(1)} KB`;
  return `${b} B`;
}

function createSvg(width, height) {
  const dom = new JSDOM("<!DOCTYPE html><html><body></body></html>");
  const body = d3.select(dom.window.document.body);
  return body
    .append("svg")
    .attr("xmlns", "http://www.w3.org/2000/svg")
    .attr("width", width)
    .attr("height", height)
    .attr("font-family", "system-ui, -apple-system, sans-serif");
}

// ---------------------------------------------------------------------------
// Performance chart — grouped horizontal bars
// ---------------------------------------------------------------------------

function renderPerfChart(perf) {
  // Group entries preserving insertion order.
  const groups = [];
  const groupMap = {};
  for (const [key, us] of Object.entries(perf)) {
    const dot = key.lastIndexOf(".");
    const bench = key.substring(0, dot);
    const lib = key.substring(dot + 1);
    if (!groupMap[bench]) {
      groupMap[bench] = {};
      groups.push({ name: bench, libs: groupMap[bench] });
    }
    groupMap[bench][lib] = us;
  }

  const margin = { top: 40, right: 120, bottom: 50, left: 160 };
  const barH = 18;
  const groupPad = 16;
  const innerH = groups.length * (PERF_LIBS.length * barH + groupPad) - groupPad;
  const width = 720;
  const height = innerH + margin.top + margin.bottom;

  const svg = createSvg(width, height);
  svg.append("rect").attr("width", "100%").attr("height", "100%").attr("fill", "#fff");

  // Title.
  svg
    .append("text")
    .attr("x", width / 2)
    .attr("y", 24)
    .attr("text-anchor", "middle")
    .attr("font-size", 14)
    .attr("font-weight", "bold")
    .text("Performance — round-trip (lower is better)");

  const g = svg.append("g").attr("transform", `translate(${margin.left},${margin.top})`);
  const chartW = width - margin.left - margin.right;

  let yOffset = 0;
  for (const { name, libs } of groups) {
    // Per-group scale so bars are always readable.
    let groupMax = 0;
    for (const lib of PERF_LIBS) {
      if (libs[lib] != null && libs[lib] > groupMax) groupMax = libs[lib];
    }
    const x = d3.scaleLinear().domain([0, groupMax]).range([0, chartW]).nice();

    const groupG = g.append("g").attr("transform", `translate(0,${yOffset})`);

    // Group label.
    groupG
      .append("text")
      .attr("x", -12)
      .attr("y", (PERF_LIBS.length * barH) / 2)
      .attr("text-anchor", "end")
      .attr("dominant-baseline", "middle")
      .attr("font-size", 11)
      .attr("fill", "#374151")
      .text(name);

    // Bars.
    PERF_LIBS.forEach((lib, i) => {
      const us = libs[lib];
      if (us == null) return;
      const barW = Math.max(4, x(us));
      const by = i * barH;

      groupG
        .append("rect")
        .attr("x", 0)
        .attr("y", by + 1)
        .attr("width", barW)
        .attr("height", barH - 2)
        .attr("fill", PERF_COLORS[lib])
        .attr("rx", 3);

      groupG
        .append("text")
        .attr("x", barW + 6)
        .attr("y", by + barH / 2 + 1)
        .attr("dominant-baseline", "middle")
        .attr("font-size", 10)
        .attr("fill", "#6b7280")
        .text(formatUs(us));
    });

    yOffset += PERF_LIBS.length * barH + groupPad;
  }

  // Legend.
  const legend = svg
    .append("g")
    .attr("transform", `translate(${margin.left}, ${height - 20})`);
  let lx = 0;
  for (const lib of PERF_LIBS) {
    legend.append("rect").attr("x", lx).attr("y", 0).attr("width", 12).attr("height", 12).attr("fill", PERF_COLORS[lib]).attr("rx", 2);
    const label = LABELS[lib] || lib;
    legend.append("text").attr("x", lx + 16).attr("y", 10).attr("font-size", 11).text(label);
    lx += label.length * 6.5 + 30;
  }

  return svg.node().outerHTML;
}

// ---------------------------------------------------------------------------
// Size chart — grouped horizontal bars (raw + gz per crate)
// ---------------------------------------------------------------------------

function renderSizeChart(size) {
  const crates = {};
  for (const [key, bytes] of Object.entries(size)) {
    const dot = key.lastIndexOf(".");
    const name = key.substring(0, dot);
    const metric = key.substring(dot + 1);
    if (!crates[name]) crates[name] = {};
    crates[name][metric] = bytes;
  }
  const crateNames = SIZE_ORDER.filter((n) => crates[n]);
  const metrics = ["raw", "gz"];

  const margin = { top: 40, right: 120, bottom: 50, left: 120 };
  const barH = 22;
  const groupPad = 14;
  const innerH = crateNames.length * (metrics.length * barH + groupPad) - groupPad;
  const width = 720;
  const height = innerH + margin.top + margin.bottom;

  const svg = createSvg(width, height);
  svg.append("rect").attr("width", "100%").attr("height", "100%").attr("fill", "#fff");

  svg
    .append("text")
    .attr("x", width / 2)
    .attr("y", 24)
    .attr("text-anchor", "middle")
    .attr("font-size", 14)
    .attr("font-weight", "bold")
    .text("Bundle Size (after wasm-bindgen + wasm-opt -Oz)");

  const g = svg.append("g").attr("transform", `translate(${margin.left},${margin.top})`);
  const chartW = width - margin.left - margin.right;

  let maxBytes = 0;
  for (const m of Object.values(crates)) {
    for (const v of Object.values(m)) {
      if (v > maxBytes) maxBytes = v;
    }
  }

  const x = d3.scaleLinear().domain([0, maxBytes]).range([0, chartW]).nice();
  const xAxis = d3.axisBottom(x).ticks(6).tickFormat((d) => formatBytes(d));
  g.append("g")
    .attr("transform", `translate(0,${innerH})`)
    .call(xAxis)
    .selectAll("text")
    .attr("font-size", 10);

  g.append("g")
    .selectAll("line")
    .data(x.ticks(6))
    .join("line")
    .attr("x1", (d) => x(d))
    .attr("x2", (d) => x(d))
    .attr("y1", 0)
    .attr("y2", innerH)
    .attr("stroke", "#e5e7eb")
    .attr("stroke-dasharray", "3,3");

  const metricLabels = { raw: "raw", gz: "gzip" };

  let yOffset = 0;
  for (const name of crateNames) {
    const groupG = g.append("g").attr("transform", `translate(0,${yOffset})`);
    const color = SIZE_COLORS[name] || "#999";

    groupG
      .append("text")
      .attr("x", -12)
      .attr("y", (metrics.length * barH) / 2)
      .attr("text-anchor", "end")
      .attr("dominant-baseline", "middle")
      .attr("font-size", 11)
      .attr("fill", "#374151")
      .text(LABELS[name] || name);

    metrics.forEach((metric, i) => {
      const bytes = crates[name][metric];
      if (bytes == null) return;
      const barW = x(bytes);
      const by = i * barH;
      const opacity = metric === "gz" ? 0.55 : 1;

      groupG
        .append("rect")
        .attr("x", 0)
        .attr("y", by + 1)
        .attr("width", barW)
        .attr("height", barH - 2)
        .attr("fill", color)
        .attr("opacity", opacity)
        .attr("rx", 3);

      groupG
        .append("text")
        .attr("x", barW + 6)
        .attr("y", by + barH / 2 + 1)
        .attr("dominant-baseline", "middle")
        .attr("font-size", 10)
        .attr("fill", "#6b7280")
        .text(`${formatBytes(bytes)} (${metricLabels[metric]})`);
    });

    yOffset += metrics.length * barH + groupPad;
  }

  // Legend.
  const legend = svg
    .append("g")
    .attr("transform", `translate(${margin.left}, ${height - 20})`);
  let lx = 0;
  for (const name of crateNames) {
    legend.append("rect").attr("x", lx).attr("y", 0).attr("width", 12).attr("height", 12).attr("fill", SIZE_COLORS[name]).attr("rx", 2);
    const sizeLabel = LABELS[name] || name;
    legend.append("text").attr("x", lx + 16).attr("y", 10).attr("font-size", 11).text(sizeLabel);
    lx += sizeLabel.length * 6.5 + 30;
  }

  return svg.node().outerHTML;
}

// ---------------------------------------------------------------------------
// Main — stack perf + size charts vertically
// ---------------------------------------------------------------------------

function main() {
  const data = readInput();
  const parts = [];

  if (data.perf && Object.keys(data.perf).length > 0) {
    parts.push(renderPerfChart(data.perf));
  }
  if (data.size && Object.keys(data.size).length > 0) {
    parts.push(renderSizeChart(data.size));
  }

  if (parts.length === 0) {
    process.stderr.write("No data to chart.\n");
    process.exit(1);
  }

  // Parse dimensions and stack into one SVG.
  let totalH = 0;
  let maxW = 0;
  const dims = parts.map((p) => {
    const wm = p.match(/width="(\d+)"/);
    const hm = p.match(/height="(\d+)"/);
    const w = wm ? parseInt(wm[1]) : 720;
    const h = hm ? parseInt(hm[1]) : 400;
    if (w > maxW) maxW = w;
    totalH += h;
    return { w, h };
  });
  totalH += (parts.length - 1) * 24;

  let out = `<svg xmlns="http://www.w3.org/2000/svg" width="${maxW}" height="${totalH}" font-family="system-ui,-apple-system,sans-serif">\n`;
  out += `<rect width="100%" height="100%" fill="#fff"/>\n`;

  let yOffset = 0;
  for (let i = 0; i < parts.length; i++) {
    const inner = parts[i]
      .replace(/<svg[^>]*>/, "")
      .replace(/<\/svg>\s*$/, "")
      .replace(/<rect width="100%" height="100%"[^/]*\/>/, "");
    out += `<g transform="translate(0,${yOffset})">\n${inner}</g>\n`;
    yOffset += dims[i].h + 24;
  }

  out += `</svg>\n`;
  process.stdout.write(out);
}

main();
