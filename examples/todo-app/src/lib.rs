#![cfg(target_arch = "wasm32")]

use typewire::Typewire;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Domain types — derive generates to_js/from_js + embeds schema in the binary
// ---------------------------------------------------------------------------

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
}

#[derive(Clone, PartialEq, Eq, Typewire)]
#[typewire(rename_all = "lowercase")]
pub enum Priority {
  Low,
  Medium,
  High,
}

#[derive(Clone, Typewire)]
#[typewire(tag = "type", content = "data")]
pub enum Command {
  Add(Todo),
  Toggle { id: u32 },
  Remove { id: u32 },
  SetPriority { id: u32, priority: Priority },
}

#[derive(Clone, Typewire)]
#[typewire(transparent)]
pub struct TodoId(u32);

#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct TodoList {
  pub name: String,
  pub todos: Vec<Todo>,
}

// ---------------------------------------------------------------------------
// Re-export typewire-generated types into the wasm-bindgen .d.ts
// ---------------------------------------------------------------------------

#[wasm_bindgen(typescript_custom_section)]
const TS_IMPORTS: &str = r#"import type { Todo, TodoList, Command } from '../types.d.ts';"#;

// ---------------------------------------------------------------------------
// Exported wasm functions using the derived Typewire conversions
// ---------------------------------------------------------------------------

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
  }

  Ok(list)
}
