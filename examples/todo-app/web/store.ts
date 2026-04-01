// MobX-observable view model backed by wasm state + patch_js.
//
// The wasm module owns the canonical TodoList state. On each dispatch,
// we call `view(vm)` which uses `patch_js` to structurally diff the
// Rust state into the JS observable -- only changed properties are
// touched, so MobX sees fine-grained updates.

import { makeAutoObservable } from "mobx";
import type { TodoList, Command, Priority } from "../types.d.ts";

let wasmModule: typeof import("../pkg/todo_app.js") | null = null;

// The view model is a plain object that `patch_js` mutates in place.
// MobX observes it, so any property change triggers a re-render.
const vm: TodoList = makeAutoObservable({
  name: "",
  todos: [],
} as TodoList);

export async function initStore() {
  // Dynamic import so vite can bundle the wasm
  const wasm = await import("../pkg/todo_app.js");
  wasmModule = wasm;
  wasm.init("My Todos");
  wasm.view(vm);
}

export function dispatch(cmd: Command) {
  if (!wasmModule) return;
  wasmModule.dispatch(cmd);
  wasmModule.view(vm);
}

let nextId = 1;

export function addTodo(title: string, priority: Priority = "medium") {
  dispatch({
    type: "Add",
    data: {
      id: nextId++,
      title,
      completed: false,
      description: null,
      priority,
      tags: [],
      createdAt: Date.now(),
      metadata: {},
      extra: {},
    },
  });
}

export function toggleTodo(id: number) {
  dispatch({ type: "Toggle", data: { id } });
}

export function removeTodo(id: number) {
  dispatch({ type: "Remove", data: { id } });
}

export function setPriority(id: number, priority: Priority) {
  dispatch({ type: "SetPriority", data: { id, priority } });
}

export { vm };
