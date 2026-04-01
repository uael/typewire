import { useState } from "react";
import { observer } from "mobx-react-lite";
import type { Priority } from "../types.d.ts";
import { vm, addTodo, toggleTodo, removeTodo, setPriority } from "./store.ts";

const PRIORITIES: Priority[] = ["low", "medium", "high"];

const TodoItem = observer(function TodoItem({ todo }: { todo: (typeof vm.todos)[number] }) {
  return (
    <div className={`todo-item ${todo.completed ? "completed" : ""}`}>
      <input
        type="checkbox"
        checked={todo.completed}
        onChange={() => toggleTodo(todo.id)}
      />
      <span className="todo-title">{todo.title}</span>
      <select
        className={`todo-priority ${todo.priority}`}
        value={todo.priority}
        onChange={(e) => setPriority(todo.id, e.target.value as Priority)}
      >
        {PRIORITIES.map((p) => (
          <option key={p} value={p}>{p}</option>
        ))}
      </select>
      <button className="btn-icon" onClick={() => removeTodo(todo.id)} title="Remove">
        &#x2715;
      </button>
    </div>
  );
});

export const App = observer(function App() {
  const [title, setTitle] = useState("");
  const [priority, setPri] = useState<Priority>("medium");

  const handleAdd = () => {
    const trimmed = title.trim();
    if (!trimmed) return;
    addTodo(trimmed, priority);
    setTitle("");
  };

  const completed = vm.todos.filter((t) => t.completed).length;

  return (
    <>
      <h1>{vm.name || "Todo App"}</h1>

      <div className="add-form">
        <input
          placeholder="What needs to be done?"
          value={title}
          onChange={(e) => setTitle(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && handleAdd()}
        />
        <select value={priority} onChange={(e) => setPri(e.target.value as Priority)}>
          {PRIORITIES.map((p) => (
            <option key={p} value={p}>{p}</option>
          ))}
        </select>
        <button onClick={handleAdd}>Add</button>
      </div>

      {vm.todos.map((todo) => (
        <TodoItem key={todo.id} todo={todo} />
      ))}

      <div className="stats">
        {vm.todos.length} items, {completed} completed
      </div>
    </>
  );
});
