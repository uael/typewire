import { readFileSync } from "fs";

const wasmBytes = readFileSync(new URL("./test_mv_bench2.wasm", import.meta.url));

const importObject = {
  env: {
    destruct_arr(obj) { return [obj.a, obj.b, obj.c]; },
    arr_get(arr, i) { return arr[i]; },
    destruct_mv3(obj) { return [obj.a, obj.b, obj.c]; },
    *destruct_mv3_gen(obj) { yield obj.a; yield obj.b; yield obj.c; },
    destruct_mv3_destr({ a, b, c }) { return [a, b, c]; },
    get_a(obj) { return obj.a; },
    get_b(obj) { return obj.b; },
    get_c(obj) { return obj.c; },
    reflect_get_a(obj) { return Reflect.get(obj, "a"); },
    reflect_get_b(obj) { return Reflect.get(obj, "b"); },
    reflect_get_c(obj) { return Reflect.get(obj, "c"); },
  },
};

const { instance } = await WebAssembly.instantiate(wasmBytes, importObject);
const { bench_array, bench_multivalue, bench_multivalue_gen, bench_multivalue_destr, bench_getters, bench_reflect, get_result } = instance.exports;

const obj = { a: 42, b: "hello", c: true };

// Verify
bench_array(obj);
console.log("Array:", get_result(0), get_result(1), get_result(2));
bench_multivalue(obj);
console.log("Multi-value:", get_result(0), get_result(1), get_result(2));
bench_getters(obj);
console.log("Getters:", get_result(0), get_result(1), get_result(2));
bench_multivalue_gen(obj);
console.log("Multi-value (gen):", get_result(0), get_result(1), get_result(2));
bench_multivalue_destr(obj);
console.log("Multi-value (destr):", get_result(0), get_result(1), get_result(2));
bench_reflect(obj);
console.log("Reflect:", get_result(0), get_result(1), get_result(2));

const N = 1_000_000;
console.log(`\nBenchmark (${N} iterations):`);

// Warmup
for (let i = 0; i < 50000; i++) {
  bench_array(obj);
  bench_multivalue(obj);
  bench_getters(obj);
  bench_multivalue_gen(obj);
  bench_multivalue_destr(obj);
  bench_reflect(obj);
}

for (let run = 1; run <= 3; run++) {
  console.log(`\nRun ${run}:`);

  let t0 = performance.now();
  for (let i = 0; i < N; i++) bench_reflect(obj);
  let t1 = performance.now();
  console.log(`  Reflect.get (3 calls):             ${(t1 - t0).toFixed(1)}ms`);

  t0 = performance.now();
  for (let i = 0; i < N; i++) bench_array(obj);
  t1 = performance.now();
  console.log(`  Array (1 destruct + 3 get):        ${(t1 - t0).toFixed(1)}ms`);

  t0 = performance.now();
  for (let i = 0; i < N; i++) bench_multivalue(obj);
  t1 = performance.now();
  console.log(`  Multi-value array (1 call, 3 ret): ${(t1 - t0).toFixed(1)}ms`);

  t0 = performance.now();
  for (let i = 0; i < N; i++) bench_multivalue_gen(obj);
  t1 = performance.now();
  console.log(`  Multi-value gen (1 call, 3 ret):   ${(t1 - t0).toFixed(1)}ms`);

  t0 = performance.now();
  for (let i = 0; i < N; i++) bench_multivalue_destr(obj);
  t1 = performance.now();
  console.log(`  Multi-value destr (1 call, 3 ret): ${(t1 - t0).toFixed(1)}ms`);

  t0 = performance.now();
  for (let i = 0; i < N; i++) bench_getters(obj);
  t1 = performance.now();
  console.log(`  Per-field getters (3 calls):       ${(t1 - t0).toFixed(1)}ms`);
}
