#![cfg(target_arch = "wasm32")]

use std::{cell::RefCell, collections::HashMap};

use typewire::Typewire;
use wasm_bindgen::prelude::*;

// ===========================================================================
// Transparent newtypes
// ===========================================================================

/// Opaque user identifier — serializes as a plain string.
#[derive(Clone, PartialEq, Eq, Hash, Typewire)]
#[typewire(transparent)]
pub struct UserId(String);

/// Opaque message identifier.
#[derive(Clone, PartialEq, Eq, Hash, Typewire)]
#[typewire(transparent)]
pub struct MessageId(String);

/// Unix timestamp in milliseconds — serializes as a number.
#[derive(Clone, PartialEq, Typewire)]
#[typewire(transparent)]
pub struct Timestamp(f64);

// ===========================================================================
// Core domain types
// ===========================================================================

/// Priority levels — all-unit enum (external tagging -> string union).
#[derive(Clone, PartialEq, Eq, Typewire)]
#[typewire(rename_all = "lowercase")]
pub enum Priority {
  Low,
  Medium,
  High,
}

/// A todo item with rich metadata.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct Todo {
  pub id: u32,
  pub title: String,
  pub completed: bool,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub priority: Priority,
  pub tags: Vec<String>,
  pub created_at: Timestamp,
  /// Arbitrary key-value metadata (e.g. category, color).
  pub metadata: HashMap<String, String>,
  /// Arbitrary extra data — exercises the `serde_json` feature.
  pub extra: HashMap<String, serde_json::Value>,
}

/// A named list of todos.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct TodoList {
  pub name: String,
  pub todos: Vec<Todo>,
}

// ===========================================================================
// Message content — internally tagged enum
// ===========================================================================

/// Content variants inside a message.
/// Demonstrates internally tagged enum (`tag = "type"`).
#[derive(Clone, Typewire)]
#[typewire(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MessageContent {
  /// Plain text message.
  Text { body: String },
  /// Image with binary data and dimensions.
  Image {
    #[typewire(base64)]
    data: Vec<u8>,
    width: u32,
    height: u32,
    alt_text: Option<String>,
  },
  /// A reply referencing another message.
  Reply { parent_id: MessageId, body: String },
  /// System-generated message (join, leave, rename, etc.).
  System { text: String },
}

// ===========================================================================
// Commands — adjacently tagged enum
// ===========================================================================

/// Commands that can be applied to the todo list.
/// Demonstrates adjacently tagged enum (`tag` + `content`).
#[derive(Clone, Typewire)]
#[typewire(tag = "type", content = "data", rename_all_fields = "camelCase")]
pub enum Command {
  Add(Todo),
  Toggle {
    id: u32,
  },
  Remove {
    id: u32,
  },
  SetPriority {
    id: u32,
    priority: Priority,
  },
  /// Send a message associated with a todo.
  SendMessage {
    todo_id: u32,
    content: MessageContent,
    options: SendOptions,
  },
}

/// Options for sending a message.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase", default)]
pub struct SendOptions {
  pub notify: bool,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub thread_id: Option<MessageId>,
  pub mentions: Vec<UserId>,
}

impl Default for SendOptions {
  fn default() -> Self {
    Self { notify: true, thread_id: None, mentions: Vec::new() }
  }
}

// ===========================================================================
// Reactions — adjacently tagged enum
// ===========================================================================

/// A reaction event on a message.
/// Demonstrates adjacently tagged enum (`tag` + `content`).
#[derive(Clone, Typewire)]
#[typewire(tag = "action", content = "payload", rename_all_fields = "camelCase")]
pub enum ReactionEvent {
  Add { emoji: String, user_id: UserId },
  Remove { emoji: String, user_id: UserId },
  Clear,
}

// ===========================================================================
// Read receipts — untagged enum
// ===========================================================================

/// Read receipt — parsed from either a timestamp number or an object.
/// Demonstrates untagged enum.
#[derive(Clone, Typewire)]
#[typewire(untagged)]
pub enum ReadReceipt {
  /// Simple: just a timestamp.
  Simple(Timestamp),
  /// Detailed: timestamp + device info.
  Detailed { timestamp: Timestamp, device: String },
}

// ===========================================================================
// Server events — tuples + nested enums
// ===========================================================================

/// Coordinates for positioning (demonstrates tuple struct).
#[derive(Clone, PartialEq, Typewire)]
pub struct Position(pub f64, pub f64);

/// Typing indicator with position tuple.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct TypingIndicator {
  pub user_id: UserId,
  /// Cursor position (line, column) — demonstrates tuple usage.
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub cursor_position: Option<Position>,
}

/// Server-sent events.
#[derive(Clone, Typewire)]
#[typewire(
  tag = "event",
  content = "data",
  rename_all = "camelCase",
  rename_all_fields = "camelCase"
)]
pub enum ServerEvent {
  /// New message received.
  MessageReceived { todo_id: u32, content: MessageContent, sent_at: Timestamp },
  /// User started typing.
  UserTyping(TypingIndicator),
  /// Reaction added or removed.
  ReactionUpdated { message_id: MessageId, event: ReactionEvent },
  /// Connection health — no payload.
  Ping,
}

// ===========================================================================
// Proxy types — serde(try_from/into)
// ===========================================================================

/// A non-empty string validated at the boundary.
/// Demonstrates `#[typewire(try_from = "String", into = "String")]`.
#[derive(Clone, PartialEq, Eq, Typewire)]
#[typewire(try_from = "String", into = "String")]
pub struct NonEmptyString(String);

impl TryFrom<String> for NonEmptyString {
  type Error = &'static str;
  fn try_from(s: String) -> Result<Self, Self::Error> {
    if s.is_empty() { Err("string must not be empty") } else { Ok(Self(s)) }
  }
}

impl From<NonEmptyString> for String {
  fn from(v: NonEmptyString) -> Self {
    v.0
  }
}

/// API response metadata.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct ResponseMeta {
  pub request_id: String,
  #[typewire(rename = "ok")]
  pub success: bool,
  pub server_time: Timestamp,
}

// ===========================================================================
// Re-export typewire-generated types into the wasm-bindgen .d.ts
// ===========================================================================

#[wasm_bindgen(typescript_custom_section)]
const TS_IMPORTS: &str = r#"import type {
  UserId, MessageId, Timestamp, Priority, Todo, TodoList, MessageContent,
  Command, SendOptions, ReactionEvent, ReadReceipt, Position, TypingIndicator,
  ServerEvent, NonEmptyString, ResponseMeta
} from '../types.d.ts';"#;

// ===========================================================================
// Exported wasm functions — a realistic todo-app API
// ===========================================================================

/// Round-trip a JS object through the `Todo` type.
///
/// # Errors
///
/// Returns an error if the value is not a valid `Todo`.
#[expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#[wasm_bindgen(unchecked_return_type = "Todo")]
pub fn create_todo(
  #[wasm_bindgen(unchecked_param_type = "Todo")] value: Todo,
) -> Result<Todo, typewire::Error> {
  Ok(value)
}

/// Apply a `Command` to a `TodoList` and return the updated list.
///
/// # Errors
///
/// Returns an error if the inputs are not valid `TodoList`/`Command`.
#[wasm_bindgen(unchecked_return_type = "TodoList")]
pub fn apply_command(
  #[wasm_bindgen(unchecked_param_type = "TodoList")] mut list: TodoList,
  #[wasm_bindgen(unchecked_param_type = "Command")] cmd: Command,
) -> Result<TodoList, typewire::Error> {
  match cmd {
    Command::Add(todo) => list.todos.push(todo),
    Command::Toggle { id } => {
      if let Some(t) = list.todos.iter_mut().find(|t| t.id == id) {
        t.completed = !t.completed;
      }
    }
    Command::Remove { id } => list.todos.retain(|t| t.id != id),
    Command::SetPriority { id, priority } => {
      if let Some(t) = list.todos.iter_mut().find(|t| t.id == id) {
        t.priority = priority;
      }
    }
    Command::SendMessage { .. } => { /* no-op for demo */ }
  }

  Ok(list)
}

/// Describe a command for display.
#[expect(clippy::must_use_candidate, reason = "wasm_bindgen exports are called from JS")]
#[wasm_bindgen]
pub fn describe_command(#[wasm_bindgen(unchecked_param_type = "Command")] cmd: Command) -> String {
  match cmd {
    Command::Add(todo) => format!("add: {}", todo.title),
    Command::Toggle { id } => format!("toggle: {id}"),
    Command::Remove { id } => format!("remove: {id}"),
    Command::SetPriority { id, priority: _ } => format!("set priority: {id}"),
    Command::SendMessage { content, .. } => match content {
      MessageContent::Text { body } => format!("send text: {body}"),
      MessageContent::Image { width, height, .. } => {
        format!("send image: {width}x{height}")
      }
      MessageContent::Reply { body, .. } => format!("reply: {body}"),
      MessageContent::System { text } => format!("system: {text}"),
    },
  }
}

/// Round-trip a `ServerEvent`.
///
/// # Errors
///
/// Returns an error if the value is not a valid `ServerEvent`.
#[expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#[wasm_bindgen(unchecked_return_type = "ServerEvent")]
pub fn create_event(
  #[wasm_bindgen(unchecked_param_type = "ServerEvent")] value: ServerEvent,
) -> Result<ServerEvent, typewire::Error> {
  Ok(value)
}

/// Round-trip an untagged `ReadReceipt`.
///
/// # Errors
///
/// Returns an error if the value is not a valid `ReadReceipt`.
#[expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#[wasm_bindgen(unchecked_return_type = "ReadReceipt")]
pub fn create_read_receipt(
  #[wasm_bindgen(unchecked_param_type = "ReadReceipt")] value: ReadReceipt,
) -> Result<ReadReceipt, typewire::Error> {
  Ok(value)
}

/// Round-trip a `ResponseMeta` (exercises rename on fields).
///
/// # Errors
///
/// Returns an error if the value is not valid.
#[expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#[wasm_bindgen(unchecked_return_type = "ResponseMeta")]
pub fn create_response_meta(
  #[wasm_bindgen(unchecked_param_type = "ResponseMeta")] value: ResponseMeta,
) -> Result<ResponseMeta, typewire::Error> {
  Ok(value)
}

/// Validate a non-empty string (exercises proxy type validation).
///
/// # Errors
///
/// Returns an error if the string is empty.
#[wasm_bindgen]
pub fn validate_non_empty(
  #[wasm_bindgen(unchecked_param_type = "NonEmptyString")] value: NonEmptyString,
) -> Result<String, typewire::Error> {
  Ok(String::from(value))
}

/// Look up a todo by id and return its title, or `"not found"`.
#[expect(
  clippy::needless_pass_by_value,
  clippy::must_use_candidate,
  reason = "wasm_bindgen exports require owned params and are called from JS"
)]
#[wasm_bindgen]
pub fn get_todo_title(
  #[wasm_bindgen(unchecked_param_type = "TodoList")] list: TodoList,
  id: u32,
) -> String {
  list
    .todos
    .iter()
    .find(|t| t.id == id)
    .map_or_else(|| "not found".to_string(), |t| t.title.clone())
}

/// Return how many todos in the list are completed.
#[expect(
  clippy::needless_pass_by_value,
  clippy::must_use_candidate,
  reason = "wasm_bindgen exports require owned params and are called from JS"
)]
#[wasm_bindgen]
pub fn count_completed(#[wasm_bindgen(unchecked_param_type = "TodoList")] list: TodoList) -> u32 {
  u32::try_from(list.todos.iter().filter(|t| t.completed).count()).unwrap_or(u32::MAX)
}

/// Filter todos by priority and return the matching list.
///
/// # Errors
///
/// Returns an error if the inputs are not valid.
#[expect(clippy::needless_pass_by_value, reason = "wasm_bindgen requires owned params")]
#[wasm_bindgen(unchecked_return_type = "TodoList")]
pub fn filter_by_priority(
  #[wasm_bindgen(unchecked_param_type = "TodoList")] list: TodoList,
  #[wasm_bindgen(unchecked_param_type = "Priority")] priority: Priority,
) -> Result<TodoList, typewire::Error> {
  Ok(TodoList {
    name: list.name,
    todos: list.todos.into_iter().filter(|t| t.priority == priority).collect(),
  })
}

// ===========================================================================
// Stateful API — local state + patch_js-based view updates
// ===========================================================================

thread_local! {
  static STATE: RefCell<TodoList> = const { RefCell::new(TodoList {
    name: String::new(),
    todos: Vec::new(),
  }) };
}

/// Initialize the local state with a name.
#[wasm_bindgen]
pub fn init(name: &str) {
  STATE.with(|s| {
    let mut state = s.borrow_mut();
    state.name = name.to_string();
    state.todos.clear();
  });
}

/// Dispatch a command to mutate the local state.
///
/// # Errors
///
/// Returns an error if the command is not valid.
#[wasm_bindgen]
pub fn dispatch(
  #[wasm_bindgen(unchecked_param_type = "Command")] cmd: Command,
) -> Result<(), typewire::Error> {
  STATE.with(|s| {
    let mut state = s.borrow_mut();
    match cmd {
      Command::Add(todo) => state.todos.push(todo),
      Command::Toggle { id } => {
        if let Some(t) = state.todos.iter_mut().find(|t| t.id == id) {
          t.completed = !t.completed;
        }
      }
      Command::Remove { id } => state.todos.retain(|t| t.id != id),
      Command::SetPriority { id, priority } => {
        if let Some(t) = state.todos.iter_mut().find(|t| t.id == id) {
          t.priority = priority;
        }
      }
      Command::SendMessage { .. } => {}
    }
  });
  Ok(())
}

/// Patch a JS view object in place to reflect the current state.
///
/// Uses `patch_js` for structural diffing — only the properties that
/// actually changed are touched.  In a reactive UI (e.g. `MobX`), this
/// triggers fine-grained re-renders.
#[wasm_bindgen]
pub fn view(view: &JsValue) {
  STATE.with(|s| {
    let state = s.borrow();
    state.patch_js(view, |fresh| {
      // Full replace fallback: copy all properties from fresh to view.
      // In practice this only fires on the very first call (when view
      // is an empty object).
      let src = js_sys::Object::from(fresh);
      let dst = js_sys::Object::from(view.clone());
      let entries = js_sys::Object::entries(&src);
      for i in 0..entries.length() {
        let pair = js_sys::Array::from(&entries.get(i));
        js_sys::Reflect::set(&dst, &pair.get(0), &pair.get(1)).ok();
      }
    });
  });
}
