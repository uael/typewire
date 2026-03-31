// Test: Can JS host functions return multiple wasm values (externref)?
import { readFileSync } from "fs";

const wasmBytes = readFileSync(new URL("./test_multivalue_import.wasm", import.meta.url));

const importObject = {
  env: {
    // Returns an iterable of 2 values — per the spec, the engine should
    // destructure this into 2 externrefs
    get_pair() {
      console.log("  [JS] get_pair called, returning [42, 'hello']");
      return [42, "hello"];
    },

    // Takes an object, returns 3 values
    destruct3(obj) {
      console.log(`  [JS] destruct3 called with:`, obj);
      return [obj.a, obj.b, obj.c];
    },
  },
};

try {
  const { instance } = await WebAssembly.instantiate(wasmBytes, importObject);
  const { test_get_pair, test_destruct3, get_result } = instance.exports;

  console.log("=== Test 1: get_pair (multi-value return: 2 externrefs) ===");
  test_get_pair();
  const r0 = get_result(0);
  const r1 = get_result(1);
  console.log(`  Result[0] = ${r0} (type: ${typeof r0})`);
  console.log(`  Result[1] = ${r1} (type: ${typeof r1})`);

  console.log("\n=== Test 2: destruct3 (multi-value return: 3 externrefs) ===");
  test_destruct3({ a: 10, b: "foo", c: true });
  const d0 = get_result(0);
  const d1 = get_result(1);
  const d2 = get_result(2);
  console.log(`  Result[0] = ${d0} (type: ${typeof d0})`);
  console.log(`  Result[1] = ${d1} (type: ${typeof d1})`);
  console.log(`  Result[2] = ${d2} (type: ${typeof d2})`);

  console.log("\n✅ Multi-value import from JS works!");
} catch (e) {
  console.error("❌ Failed:", e.message);
  console.error(e.stack);
}
