use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use typewire::Typewire;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// JS identity functions (force values across the wasm–JS boundary)
// ---------------------------------------------------------------------------

#[wasm_bindgen(inline_js = "
export function js_identity(v) { return v; }
export function js_json_roundtrip(s) { return JSON.stringify(JSON.parse(s)); }
")]
extern "C" {
  fn js_identity(v: JsValue) -> JsValue;
  fn js_json_roundtrip(s: &str) -> String;
}

// ---------------------------------------------------------------------------
// Benchmark types -- identical definitions for typewire and serde
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Typewire, Serialize, Deserialize)]
pub struct Simple {
  pub id: u32,
  pub name: String,
  pub score: f64,
  pub active: bool,
}

#[derive(Clone, Debug, PartialEq, Typewire, Serialize, Deserialize)]
pub struct Address {
  pub street: String,
  pub city: String,
  pub zip: String,
}

#[derive(Clone, Debug, PartialEq, Typewire, Serialize, Deserialize)]
pub struct Complex {
  pub id: u32,
  pub name: String,
  pub address: Address,
  pub tags: Vec<String>,
  pub nickname: Option<String>,
  pub actions: HashMap<String, Action>,
}

// Externally tagged so that all serde formats can roundtrip.
#[derive(Clone, Debug, PartialEq, Typewire, Serialize, Deserialize)]
pub enum Action {
  Create(Simple),
  Update { id: u32, name: String },
  Delete { id: u32 },
}

// ---------------------------------------------------------------------------
// Fixture constructors
// ---------------------------------------------------------------------------

fn make_simple(i: u32) -> Simple {
  Simple {
    id: i,
    name: format!("item-{i}"),
    score: f64::from(i) * 1.5,
    active: i.is_multiple_of(2),
  }
}

fn make_action(i: u32) -> Action {
  match i % 3 {
    0 => Action::Create(make_simple(i)),
    1 => Action::Update { id: i, name: format!("updated-{i}") },
    _ => Action::Delete { id: i },
  }
}

fn make_complex(i: u32) -> Complex {
  let mut actions = HashMap::new();
  // Use a fixed set of actions for consistent fixture data across all indices.
  for j in 0..32 {
    actions.insert(j.to_string(), make_action(j));
  }

  Complex {
    id: i,
    name: format!("user-{i}"),
    address: Address {
      street: format!("{i} Main St"),
      city: "Anytown".into(),
      zip: format!("{:05}", i % 100_000),
    },
    tags: vec![format!("tag-{i}"), format!("group-{}", i % 10)],
    nickname: if i.is_multiple_of(3) { Some(format!("nick-{i}")) } else { None },
    actions,
  }
}

fn make_complex_vec(n: u32) -> Vec<Complex> {
  (0..n).map(make_complex).collect()
}

// ---------------------------------------------------------------------------
// Benchmark harness
// ---------------------------------------------------------------------------

const ITERATIONS: u32 = 50_000;
const COLLECTION_ITERATIONS: u32 = 500;
const COLLECTION_SIZE: u32 = 256;

#[wasm_bindgen]
extern "C" {
  #[wasm_bindgen(js_namespace = performance, js_name = now)]
  fn performance_now() -> f64;
}

fn bench(n: u32, f: impl Fn()) -> f64 {
  for _ in 0..100 {
    f();
  }
  let start = performance_now();
  for _ in 0..n {
    f();
  }
  performance_now() - start
}

// ---------------------------------------------------------------------------
// Round-trip benchmarks (serialize → cross JS boundary → deserialize)
// ---------------------------------------------------------------------------

// --- 1. simple struct ---

#[wasm_bindgen]
pub fn bench_simple_roundtrip_typewire() -> f64 {
  let val = make_simple(42);
  bench(ITERATIONS, || {
    let js = js_identity(val.to_js());
    let _ = Simple::from_js(js).unwrap_throw();
  })
}

#[wasm_bindgen]
pub fn bench_simple_roundtrip_serde_wasm_bindgen() -> f64 {
  let val = make_simple(42);
  let ser = serde_wasm_bindgen::Serializer::json_compatible();
  bench(ITERATIONS, || {
    let js = js_identity(val.serialize(&ser).unwrap_throw());
    let _: Simple = serde_wasm_bindgen::from_value(js).unwrap_throw();
  })
}

#[wasm_bindgen]
pub fn bench_simple_roundtrip_serde_json() -> f64 {
  let val = make_simple(42);
  bench(ITERATIONS, || {
    let s = serde_json::to_string(&val).unwrap_throw();
    let s = js_json_roundtrip(&s);
    let _: Simple = serde_json::from_str(&s).unwrap_throw();
  })
}

// --- 2. simple enum ---

#[wasm_bindgen]
pub fn bench_enum_roundtrip_typewire() -> f64 {
  let val = make_action(42);
  bench(ITERATIONS, || {
    let js = js_identity(val.to_js());
    let _ = Action::from_js(js).unwrap_throw();
  })
}

#[wasm_bindgen]
pub fn bench_enum_roundtrip_serde_wasm_bindgen() -> f64 {
  let val = make_action(42);
  let ser = serde_wasm_bindgen::Serializer::json_compatible();
  bench(ITERATIONS, || {
    let js = js_identity(val.serialize(&ser).unwrap_throw());
    let _: Action = serde_wasm_bindgen::from_value(js).unwrap_throw();
  })
}

#[wasm_bindgen]
pub fn bench_enum_roundtrip_serde_json() -> f64 {
  let val = make_action(42);
  bench(ITERATIONS, || {
    let s = serde_json::to_string(&val).unwrap_throw();
    let s = js_json_roundtrip(&s);
    let _: Action = serde_json::from_str(&s).unwrap_throw();
  })
}

// --- 3. complex struct ---

#[wasm_bindgen]
pub fn bench_complex_roundtrip_typewire() -> f64 {
  let val = make_complex(42);
  bench(ITERATIONS, || {
    let js = js_identity(val.to_js());
    let _ = Complex::from_js(js).unwrap_throw();
  })
}

#[wasm_bindgen]
pub fn bench_complex_roundtrip_serde_wasm_bindgen() -> f64 {
  let val = make_complex(42);
  let ser = serde_wasm_bindgen::Serializer::json_compatible();
  bench(ITERATIONS, || {
    let js = js_identity(val.serialize(&ser).unwrap_throw());
    let _: Complex = serde_wasm_bindgen::from_value(js).unwrap_throw();
  })
}

#[wasm_bindgen]
pub fn bench_complex_roundtrip_serde_json() -> f64 {
  let val = make_complex(42);
  bench(ITERATIONS, || {
    let s = serde_json::to_string(&val).unwrap_throw();
    let s = js_json_roundtrip(&s);
    let _: Complex = serde_json::from_str(&s).unwrap_throw();
  })
}

// --- 4. complex collection ---

#[wasm_bindgen]
pub fn bench_collection_roundtrip_typewire() -> f64 {
  let val = make_complex_vec(COLLECTION_SIZE);
  bench(COLLECTION_ITERATIONS, || {
    let js = js_identity(val.to_js());
    let _ = <Vec<Complex>>::from_js(js).unwrap_throw();
  })
}

#[wasm_bindgen]
pub fn bench_collection_roundtrip_serde_wasm_bindgen() -> f64 {
  let val = make_complex_vec(COLLECTION_SIZE);
  let ser = serde_wasm_bindgen::Serializer::json_compatible();
  bench(COLLECTION_ITERATIONS, || {
    let js = js_identity(val.serialize(&ser).unwrap_throw());
    let _: Vec<Complex> = serde_wasm_bindgen::from_value(js).unwrap_throw();
  })
}

#[wasm_bindgen]
pub fn bench_collection_roundtrip_serde_json() -> f64 {
  let val = make_complex_vec(COLLECTION_SIZE);
  bench(COLLECTION_ITERATIONS, || {
    let s = serde_json::to_string(&val).unwrap_throw();
    let s = js_json_roundtrip(&s);
    let _: Vec<Complex> = serde_json::from_str(&s).unwrap_throw();
  })
}

// ---------------------------------------------------------------------------
// Getters for the JS runner
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn get_iterations() -> u32 {
  ITERATIONS
}

#[wasm_bindgen]
pub fn get_collection_iterations() -> u32 {
  COLLECTION_ITERATIONS
}

#[wasm_bindgen]
pub fn get_collection_size() -> u32 {
  COLLECTION_SIZE
}
