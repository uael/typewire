#![cfg(target_arch = "wasm32")]
#![expect(clippy::must_use_candidate, reason = "wasm_bindgen exports are called from JS")]
#![expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#![expect(clippy::derive_partial_eq_without_eq, reason = "f64 fields prevent Eq derivation")]

// ===========================================================================
// Perf benchmarks: compare all three serialization libraries
// ===========================================================================

#[cfg(feature = "perf")]
mod perf;

// ===========================================================================
// Size benchmarks: exactly one feature per build to isolate bundle overhead
// ===========================================================================

// --- baseline: pass-through JsValue (no serialization framework) ---

#[cfg(feature = "size-baseline")]
mod size_baseline {
  use wasm_bindgen::prelude::*;

  #[wasm_bindgen]
  pub fn create_user(value: JsValue) -> Result<JsValue, JsValue> {
    Ok(value)
  }

  #[wasm_bindgen]
  pub fn get_profile(value: JsValue) -> Result<JsValue, JsValue> {
    Ok(value)
  }

  #[wasm_bindgen]
  pub fn apply_command(value: JsValue) -> Result<JsValue, JsValue> {
    Ok(value)
  }

  #[wasm_bindgen]
  pub fn apply_event(value: JsValue) -> Result<JsValue, JsValue> {
    Ok(value)
  }
}

// --- Shared serde type definitions (used by size-serde-wasm-bindgen, size-serde-json) ---

#[cfg(any(feature = "size-serde-wasm-bindgen", feature = "size-serde-json"))]
mod size_serde_types {
  use serde::{Deserialize, Serialize};

  #[derive(Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub score: f64,
  }

  #[derive(Clone, Serialize, Deserialize)]
  pub struct Address {
    pub street: String,
    pub city: String,
    pub zip: String,
  }

  #[derive(Clone, Serialize, Deserialize)]
  #[serde(rename_all = "camelCase")]
  pub struct Profile {
    pub user: User,
    pub address: Address,
    pub tags: Vec<String>,
    pub nickname: Option<String>,
  }

  #[derive(Clone, Serialize, Deserialize)]
  pub enum Command {
    Create(User),
    Update { id: u32, name: String },
    Delete { id: u32 },
  }

  #[derive(Clone, Serialize, Deserialize)]
  #[serde(tag = "type", content = "data")]
  pub enum Event {
    Created(User),
    Updated { id: u32, name: String },
    Deleted { id: u32 },
  }
}

// --- serde-wasm-bindgen: round-trip via serde-wasm-bindgen ---

#[cfg(all(feature = "size-serde-wasm-bindgen", not(feature = "size-serde-json"),))]
mod size_serde_wasm_bindgen {
  use serde::Serialize;
  use wasm_bindgen::prelude::*;

  use super::size_serde_types::*;

  #[wasm_bindgen]
  pub fn create_user(value: JsValue) -> Result<JsValue, JsValue> {
    let user: User =
      serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ser = serde_wasm_bindgen::Serializer::json_compatible();
    user.serialize(&ser).map_err(|e| JsValue::from_str(&e.to_string()))
  }

  #[wasm_bindgen]
  pub fn get_profile(value: JsValue) -> Result<JsValue, JsValue> {
    let profile: Profile =
      serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ser = serde_wasm_bindgen::Serializer::json_compatible();
    profile.serialize(&ser).map_err(|e| JsValue::from_str(&e.to_string()))
  }

  #[wasm_bindgen]
  pub fn apply_command(value: JsValue) -> Result<JsValue, JsValue> {
    let cmd: Command =
      serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ser = serde_wasm_bindgen::Serializer::json_compatible();
    cmd.serialize(&ser).map_err(|e| JsValue::from_str(&e.to_string()))
  }

  #[wasm_bindgen]
  pub fn apply_event(value: JsValue) -> Result<JsValue, JsValue> {
    let event: Event =
      serde_wasm_bindgen::from_value(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let ser = serde_wasm_bindgen::Serializer::json_compatible();
    event.serialize(&ser).map_err(|e| JsValue::from_str(&e.to_string()))
  }
}

// --- serde-json: round-trip via serde_json (JSON string) ---

#[cfg(feature = "size-serde-json")]
mod size_serde_json {
  use wasm_bindgen::prelude::*;

  use super::size_serde_types::*;

  #[wasm_bindgen]
  pub fn create_user(value: JsValue) -> Result<JsValue, JsValue> {
    let json_str = value.as_string().ok_or_else(|| JsValue::from_str("expected JSON string"))?;
    let user: User =
      serde_json::from_str(&json_str).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let out = serde_json::to_string(&user).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(JsValue::from_str(&out))
  }

  #[wasm_bindgen]
  pub fn get_profile(value: JsValue) -> Result<JsValue, JsValue> {
    let json_str = value.as_string().ok_or_else(|| JsValue::from_str("expected JSON string"))?;
    let profile: Profile =
      serde_json::from_str(&json_str).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let out = serde_json::to_string(&profile).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(JsValue::from_str(&out))
  }

  #[wasm_bindgen]
  pub fn apply_command(value: JsValue) -> Result<JsValue, JsValue> {
    let json_str = value.as_string().ok_or_else(|| JsValue::from_str("expected JSON string"))?;
    let cmd: Command =
      serde_json::from_str(&json_str).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let out = serde_json::to_string(&cmd).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(JsValue::from_str(&out))
  }

  #[wasm_bindgen]
  pub fn apply_event(value: JsValue) -> Result<JsValue, JsValue> {
    let json_str = value.as_string().ok_or_else(|| JsValue::from_str("expected JSON string"))?;
    let event: Event =
      serde_json::from_str(&json_str).map_err(|e| JsValue::from_str(&e.to_string()))?;
    let out = serde_json::to_string(&event).map_err(|e| JsValue::from_str(&e.to_string()))?;
    Ok(JsValue::from_str(&out))
  }
}

// --- typewire: round-trip via typewire derive ---

#[cfg(feature = "size-typewire")]
#[expect(clippy::needless_pass_by_value, reason = "wasm_bindgen requires owned parameters")]
mod size_typewire {
  use typewire::Typewire;
  use wasm_bindgen::prelude::*;

  #[derive(Clone, Typewire)]
  #[typewire(rename_all = "camelCase")]
  pub struct User {
    pub id: u32,
    pub name: String,
    pub email: String,
    pub active: bool,
    pub score: f64,
  }

  #[derive(Clone, Typewire)]
  pub struct Address {
    pub street: String,
    pub city: String,
    pub zip: String,
  }

  #[derive(Clone, Typewire)]
  #[typewire(rename_all = "camelCase")]
  pub struct Profile {
    pub user: User,
    pub address: Address,
    pub tags: Vec<String>,
    pub nickname: Option<String>,
  }

  #[derive(Clone, Typewire)]
  pub enum Command {
    Create(User),
    Update { id: u32, name: String },
    Delete { id: u32 },
  }

  #[derive(Clone, Typewire)]
  #[typewire(tag = "type", content = "data")]
  pub enum Event {
    Created(User),
    Updated { id: u32, name: String },
    Deleted { id: u32 },
  }

  #[wasm_bindgen]
  pub fn create_user(value: User) -> JsValue {
    value.to_js()
  }

  #[wasm_bindgen]
  pub fn get_profile(value: Profile) -> JsValue {
    value.to_js()
  }

  #[wasm_bindgen]
  pub fn apply_command(cmd: Command) -> JsValue {
    cmd.to_js()
  }

  #[wasm_bindgen]
  pub fn apply_event(event: Event) -> JsValue {
    event.to_js()
  }
}
