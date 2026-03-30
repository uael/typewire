export type Command =
  | { type: "Add"; data: Todo }
  | { type: "Toggle"; data: { id: number } }
  | { type: "Remove"; data: { id: number } }
  | { type: "SetPriority"; data: { id: number; priority: Priority } };

export type Priority = "low" | "medium" | "high";

export interface Todo {
  id: number;
  title: string;
  completed: boolean;
  description: string | null;
  priority: Priority;
  tags: string[];
}

export type TodoId = number;

export interface TodoList {
  name: string;
  todos: Todo[];
}

