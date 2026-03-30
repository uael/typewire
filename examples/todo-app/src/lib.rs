use typewire::Typewire;
use wasm_bindgen::prelude::*;

// ---------------------------------------------------------------------------
// Domain types — derive generates to_js/from_js + embeds schema in the binary
// ---------------------------------------------------------------------------

#[derive(Clone, Typewire)]
#[serde(rename_all = "camelCase")]
pub struct Todo {
  pub id: u32,
  pub title: String,
  pub completed: bool,
  #[serde(skip_serializing_if = "Option::is_none")]
  pub description: Option<String>,
  pub priority: Priority,
  pub tags: Vec<String>,
}

#[derive(Clone, PartialEq, Typewire)]
#[serde(rename_all = "lowercase")]
pub enum Priority {
  Low,
  Medium,
  High,
}

#[derive(Clone, Typewire)]
#[serde(tag = "type", content = "data")]
pub enum Command {
  Add(Todo),
  Toggle { id: u32 },
  Remove { id: u32 },
  SetPriority { id: u32, priority: Priority },
}

#[derive(Clone, Typewire)]
#[serde(transparent)]
pub struct TodoId(u32);

#[derive(Clone, Typewire)]
#[serde(rename_all = "camelCase")]
pub struct TodoList {
  pub name: String,
  pub todos: Vec<Todo>,
}

// ---------------------------------------------------------------------------
// Exported wasm functions using the derived Typewire conversions
// ---------------------------------------------------------------------------

#[wasm_bindgen]
pub fn create_todo(value: JsValue) -> Result<JsValue, JsValue> {
  let todo = Todo::from_js(value).map_err(|e| JsValue::from_str(&e.to_string()))?;
  Ok(todo.to_js())
}

#[wasm_bindgen]
pub fn apply_command(list: JsValue, cmd: JsValue) -> Result<JsValue, JsValue> {
  let mut todo_list = TodoList::from_js(list).map_err(|e| JsValue::from_str(&e.to_string()))?;
  let command = Command::from_js(cmd).map_err(|e| JsValue::from_str(&e.to_string()))?;

  match command {
    Command::Add(todo) => todo_list.todos.push(todo),
    Command::Toggle { id } => {
      if let Some(t) = todo_list.todos.iter_mut().find(|t| t.id == id) {
        t.completed = !t.completed;
      }
    }
    Command::Remove { id } => todo_list.todos.retain(|t| t.id != id),
    Command::SetPriority { id, priority } => {
      if let Some(t) = todo_list.todos.iter_mut().find(|t| t.id == id) {
        t.priority = priority;
      }
    }
  }

  Ok(todo_list.to_js())
}
