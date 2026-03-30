// End-to-end test: load the wasm module and exercise the exported functions.
//
// Prerequisites:
//   cargo build -p todo-app --target wasm32-unknown-unknown --release
//   wasm-bindgen target/wasm32-unknown-unknown/release/todo_app.wasm \
//     --out-dir examples/todo-app/pkg --target nodejs --no-typescript
//
// Run:
//   node examples/todo-app/test.mjs

import { create_todo, apply_command } from "./pkg/todo_app.js";
import { strict as assert } from "node:assert";

// -- create_todo round-trip --------------------------------------------------

const todo = create_todo({
  id: 1,
  title: "Write tests",
  completed: false,
  priority: "high",
  tags: ["dev"],
});

assert.equal(todo.id, 1);
assert.equal(todo.title, "Write tests");
assert.equal(todo.completed, false);
assert.equal(todo.description, undefined);
assert.equal(todo.priority, "high");
assert.deepEqual(todo.tags, ["dev"]);

// -- apply_command: Add ------------------------------------------------------

let list = apply_command(
  { name: "Work", todos: [] },
  { type: "Add", data: todo },
);

assert.equal(list.name, "Work");
assert.equal(list.todos.length, 1);
assert.equal(list.todos[0].title, "Write tests");

// -- apply_command: Toggle ---------------------------------------------------

list = apply_command(list, { type: "Toggle", data: { id: 1 } });
assert.equal(list.todos[0].completed, true);

// -- apply_command: SetPriority ----------------------------------------------

list = apply_command(list, {
  type: "SetPriority",
  data: { id: 1, priority: "low" },
});
assert.equal(list.todos[0].priority, "low");

// -- apply_command: Remove ---------------------------------------------------

list = apply_command(list, { type: "Remove", data: { id: 1 } });
assert.equal(list.todos.length, 0);

console.log("ok: all assertions passed");
