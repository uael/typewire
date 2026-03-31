// End-to-end test: load the wasm module, exercise the exported functions,
// and verify the results conform to the generated TypeScript types.
//
// Type-check:  npx tsc --noEmit
// Run:         npx tsx test.ts

import { createRequire } from "node:module";
import { strict as assert } from "node:assert";

import type { Todo, TodoList, Command, Priority } from "./types.d.ts";
import type { create_todo as CreateTodoFn, apply_command as ApplyCommandFn } from "./pkg/todo_app.d.ts";

// wasm-bindgen --target nodejs emits CommonJS
const require = createRequire(import.meta.url);
const { create_todo, apply_command } = require("./pkg/todo_app.js") as {
  create_todo: typeof CreateTodoFn;
  apply_command: typeof ApplyCommandFn;
};

// -- create_todo round-trip --------------------------------------------------

const todo: Todo = create_todo({
  id: 1,
  title: "Write tests",
  completed: false,
  description: null,
  priority: "high" satisfies Priority,
  tags: ["dev"],
});

assert.equal(todo.id, 1);
assert.equal(todo.title, "Write tests");
assert.equal(todo.completed, false);
assert.equal(todo.description, undefined);
assert.equal(todo.priority, "high");
assert.deepEqual(todo.tags, ["dev"]);

// -- apply_command: Add ------------------------------------------------------

const addCmd: Command = { type: "Add", data: todo };
let list: TodoList = apply_command({ name: "Work", todos: [] }, addCmd);

assert.equal(list.name, "Work");
assert.equal(list.todos.length, 1);
assert.equal(list.todos[0].title, "Write tests");

// -- apply_command: Toggle ---------------------------------------------------

const toggleCmd: Command = { type: "Toggle", data: { id: 1 } };
list = apply_command(list, toggleCmd);
assert.equal(list.todos[0].completed, true);

// -- apply_command: SetPriority ----------------------------------------------

const setPriorityCmd: Command = {
  type: "SetPriority",
  data: { id: 1, priority: "low" satisfies Priority },
};
list = apply_command(list, setPriorityCmd);
assert.equal(list.todos[0].priority, "low");

// -- apply_command: Remove ---------------------------------------------------

const removeCmd: Command = { type: "Remove", data: { id: 1 } };
list = apply_command(list, removeCmd);
assert.equal(list.todos.length, 0);

// -- Error paths: invalid inputs should throw ----------------------------------

function assertThrows(fn: () => void, desc: string) {
  try {
    fn();
    assert.fail(`expected error for: ${desc}`);
  } catch (e) {
    if (e instanceof assert.AssertionError) throw e;
    // expected
  }
}

// Missing required field (title)
assertThrows(
  () => create_todo({ id: 1, completed: false, priority: "high", tags: [] } as any),
  "missing required field 'title'",
);

// Wrong field type (id should be number)
assertThrows(
  () => create_todo({ id: "not_a_number", title: "x", completed: false, priority: "high", tags: [] } as any),
  "wrong type for 'id'",
);

// Completely wrong type: number instead of object
assertThrows(
  () => create_todo(42 as any),
  "number instead of Todo object",
);

// Completely wrong type: string instead of object
assertThrows(
  () => create_todo("hello" as any),
  "string instead of Todo object",
);

// null argument
assertThrows(
  () => create_todo(null as any),
  "null instead of Todo object",
);

// undefined argument
assertThrows(
  () => create_todo(undefined as any),
  "undefined instead of Todo object",
);

// Invalid command type
assertThrows(
  () => apply_command({ name: "W", todos: [] } as any, { type: "Unknown", data: {} } as any),
  "unknown command variant",
);

// Missing command data
assertThrows(
  () => apply_command({ name: "W", todos: [] } as any, { type: "Toggle" } as any),
  "missing command data",
);

// Both arguments completely wrong
assertThrows(
  () => apply_command(null as any, null as any),
  "null list and null command",
);

console.log("ok: all assertions passed");
