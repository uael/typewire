// End-to-end test: load the wasm module, exercise the exported functions,
// and verify the results conform to the generated TypeScript types.
//
// Type-check:  npx tsc --noEmit
// Run:         npx tsx test.ts

import { createRequire } from "node:module";
import { strict as assert } from "node:assert";

import type { Todo, TodoList, Command, Priority, TodoId } from "./types.d.ts";

// wasm-bindgen --target nodejs emits CommonJS
const require = createRequire(import.meta.url);
const { create_todo, apply_command } = require("./pkg/todo_app.js") as {
  create_todo(value: unknown): Todo;
  apply_command(list: unknown, cmd: unknown): TodoList;
};

// -- create_todo round-trip --------------------------------------------------

const todo: Todo = create_todo({
  id: 1,
  title: "Write tests",
  completed: false,
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

// -- TodoId type check -------------------------------------------------------

const id: TodoId = 42;
assert.equal(id, 42);

// -- Type-level assertions (verified at tsc --noEmit time) --------------------

// Discriminated union narrowing: after checking type, data is narrowed.
function assertNarrowing(cmd: Command) {
  if (cmd.type === "Add") {
    // TypeScript should narrow data to Todo
    const _title: string = cmd.data.title;
    const _completed: boolean = cmd.data.completed;
  } else if (cmd.type === "Toggle") {
    // TypeScript should narrow data to { id: number }
    const _id: number = cmd.data.id;
  } else if (cmd.type === "SetPriority") {
    const _p: Priority = cmd.data.priority;
  }
}
assertNarrowing(addCmd);

// Priority is a string literal union — only valid values are accepted.
const _p1: Priority = "low";
const _p2: Priority = "medium";
const _p3: Priority = "high";

// Optional field: description is string | null, not string | undefined.
const _d: string | null = todo.description;

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
  () => create_todo({ id: 1, completed: false, priority: "high", tags: [] }),
  "missing required field 'title'",
);

// Wrong field type (id should be number)
assertThrows(
  () => create_todo({ id: "not_a_number", title: "x", completed: false, priority: "high", tags: [] }),
  "wrong type for 'id'",
);

// Invalid command type
assertThrows(
  () => apply_command({ name: "W", todos: [] }, { type: "Unknown", data: {} }),
  "unknown command variant",
);

// Missing command data
assertThrows(
  () => apply_command({ name: "W", todos: [] }, { type: "Toggle" }),
  "missing command data",
);

console.log("ok: all assertions passed");
