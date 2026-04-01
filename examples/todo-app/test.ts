// End-to-end test: load the wasm module, exercise the exported functions,
// and verify the results conform to the generated TypeScript types.
//
// Type-check:  npx tsc --noEmit
// Run:         npx tsx test.ts

import { createRequire } from "node:module";
import { strict as assert } from "node:assert";

import type {
  UserId, MessageId, Timestamp, Priority, Todo, TodoList, MessageContent,
  Command, SendOptions, ReactionEvent, ReadReceipt, Position, TypingIndicator,
  ServerEvent, NonEmptyString, ResponseMeta,
} from "./types.d.ts";

import type {
  create_todo as CreateTodoFn,
  apply_command as ApplyCommandFn,
  describe_command as DescribeCommandFn,
  create_event as CreateEventFn,
  create_read_receipt as CreateReadReceiptFn,
  create_response_meta as CreateResponseMetaFn,
  validate_non_empty as ValidateNonEmptyFn,
  get_todo_title as GetTodoTitleFn,
  count_completed as CountCompletedFn,
  filter_by_priority as FilterByPriorityFn,
  init as InitFn,
  dispatch as DispatchFn,
  view as ViewFn,
} from "./pkg/todo_app.d.ts";

// wasm-bindgen --target nodejs emits CommonJS
const require = createRequire(import.meta.url);
const {
  create_todo,
  apply_command,
  describe_command,
  create_event,
  create_read_receipt,
  create_response_meta,
  validate_non_empty,
  get_todo_title,
  count_completed,
  filter_by_priority,
  init,
  dispatch,
  view,
} = require("./pkg/todo_app.js") as {
  create_todo: typeof CreateTodoFn;
  apply_command: typeof ApplyCommandFn;
  describe_command: typeof DescribeCommandFn;
  create_event: typeof CreateEventFn;
  create_read_receipt: typeof CreateReadReceiptFn;
  create_response_meta: typeof CreateResponseMetaFn;
  validate_non_empty: typeof ValidateNonEmptyFn;
  get_todo_title: typeof GetTodoTitleFn;
  count_completed: typeof CountCompletedFn;
  filter_by_priority: typeof FilterByPriorityFn;
  init: typeof InitFn;
  dispatch: typeof DispatchFn;
  view: typeof ViewFn;
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function assertThrows(fn: () => void, desc: string) {
  try {
    fn();
    assert.fail(`expected error for: ${desc}`);
  } catch (e) {
    if (e instanceof assert.AssertionError) throw e;
    // expected
  }
}

// ---------------------------------------------------------------------------
// 1. Transparent newtypes + Todo round-trip
// ---------------------------------------------------------------------------

const userId: UserId = "u-alice-1";
const messageId: MessageId = "msg-001";
const timestamp: Timestamp = 1700000000000;

const todo: Todo = create_todo({
  id: 1,
  title: "Write tests",
  completed: false,
  description: null,
  priority: "high" satisfies Priority,
  tags: ["dev"],
  createdAt: timestamp,
  metadata: { category: "work", color: "red" },
  extra: { score: 42, labels: ["a", "b"] },
});

assert.equal(todo.id, 1);
assert.equal(todo.title, "Write tests");
assert.equal(todo.completed, false);
assert.equal(todo.description, undefined); // skip_serializing_if removes null
assert.equal(todo.priority, "high");
assert.deepEqual(todo.tags, ["dev"]);
assert.equal(todo.createdAt, timestamp);
assert.equal(todo.metadata.category, "work");
assert.equal(todo.metadata.color, "red");
assert.equal(todo.extra.score, 42);
assert.deepEqual(todo.extra.labels, ["a", "b"]);

// ---------------------------------------------------------------------------
// 2. apply_command: Add / Toggle / SetPriority / Remove
// ---------------------------------------------------------------------------

const addCmd: Command = { type: "Add", data: todo };
let list: TodoList = apply_command({ name: "Work", todos: [] }, addCmd);

assert.equal(list.name, "Work");
assert.equal(list.todos.length, 1);
assert.equal(list.todos[0].title, "Write tests");

const toggleCmd: Command = { type: "Toggle", data: { id: 1 } };
list = apply_command(list, toggleCmd);
assert.equal(list.todos[0].completed, true);

const setPriorityCmd: Command = {
  type: "SetPriority",
  data: { id: 1, priority: "low" satisfies Priority },
};
list = apply_command(list, setPriorityCmd);
assert.equal(list.todos[0].priority, "low");

const removeCmd: Command = { type: "Remove", data: { id: 1 } };
list = apply_command(list, removeCmd);
assert.equal(list.todos.length, 0);

// ---------------------------------------------------------------------------
// 3. describe_command — internally tagged MessageContent + adjacent Command
// ---------------------------------------------------------------------------

const textContent: MessageContent = { type: "text", body: "Hello!" };
const imageContent: MessageContent = {
  type: "image",
  data: "AQID", // base64 of [1, 2, 3]
  width: 800,
  height: 600,
  altText: "A photo",
};

const sendTextDesc = describe_command({
  type: "SendMessage",
  data: {
    todoId: 1,
    content: textContent,
    options: {
      notify: true,
      threadId: null,
      mentions: [userId],
    } satisfies SendOptions,
  },
});
assert.equal(sendTextDesc, "send text: Hello!");

const sendImageDesc = describe_command({
  type: "SendMessage",
  data: {
    todoId: 1,
    content: imageContent,
    options: {} as SendOptions, // all fields have defaults
  },
});
assert.equal(sendImageDesc, "send image: 800x600");

const replyDesc = describe_command({
  type: "SendMessage",
  data: {
    todoId: 1,
    content: { type: "reply", parentId: messageId, body: "I agree!" },
    options: {} as SendOptions,
  },
});
assert.equal(replyDesc, "reply: I agree!");

const systemDesc = describe_command({
  type: "SendMessage",
  data: {
    todoId: 1,
    content: { type: "system", text: "Alice joined" },
    options: {} as SendOptions,
  },
});
assert.equal(systemDesc, "system: Alice joined");

// ---------------------------------------------------------------------------
// 4. ReactionEvent — adjacently tagged enum
// ---------------------------------------------------------------------------

const reactDesc = describe_command({
  type: "Add",
  data: todo,
});
assert.equal(reactDesc, "add: Write tests");

// ---------------------------------------------------------------------------
// 5. ServerEvent — adjacently tagged with various payloads
// ---------------------------------------------------------------------------

// -- messageReceived ---
const msgEvent: ServerEvent = create_event({
  event: "messageReceived",
  data: {
    todoId: 1,
    content: textContent,
    sentAt: timestamp,
  },
});
assert.equal(msgEvent.event, "messageReceived");

// -- userTyping with Position tuple ---
const typingEvent: ServerEvent = create_event({
  event: "userTyping",
  data: {
    userId,
    cursorPosition: [10, 25] satisfies Position,
  } satisfies TypingIndicator,
});
assert.equal(typingEvent.event, "userTyping");

// -- userTyping without position ---
const typingNoPos: ServerEvent = create_event({
  event: "userTyping",
  data: {
    userId,
    cursorPosition: null,
  } satisfies TypingIndicator,
});
assert.equal(typingNoPos.event, "userTyping");

// -- reactionUpdated ---
const reactionEvent: ServerEvent = create_event({
  event: "reactionUpdated",
  data: {
    messageId,
    event: {
      action: "Add",
      payload: { emoji: "heart", userId },
    } satisfies ReactionEvent,
  },
});
assert.equal(reactionEvent.event, "reactionUpdated");

// -- reactionUpdated with Clear ---
const clearEvent: ServerEvent = create_event({
  event: "reactionUpdated",
  data: {
    messageId,
    event: { action: "Clear" } satisfies ReactionEvent,
  },
});
assert.equal(clearEvent.event, "reactionUpdated");

// -- ping (no payload) ---
const pingEvent: ServerEvent = create_event({
  event: "ping",
});
assert.equal(pingEvent.event, "ping");

// ---------------------------------------------------------------------------
// 6. Untagged enum (ReadReceipt)
// ---------------------------------------------------------------------------

// Simple variant — just a timestamp number
const simpleReceipt: ReadReceipt = create_read_receipt(1700000000000);
assert.equal(simpleReceipt, 1700000000000);

// Detailed variant — object with timestamp + device
const detailedReceipt: ReadReceipt = create_read_receipt({
  timestamp: 1700000000000,
  device: "mobile",
});
assert.equal(
  (detailedReceipt as { timestamp: number; device: string }).timestamp,
  1700000000000,
);
assert.equal(
  (detailedReceipt as { timestamp: number; device: string }).device,
  "mobile",
);

// ---------------------------------------------------------------------------
// 7. ResponseMeta — rename on fields
// ---------------------------------------------------------------------------

const meta: ResponseMeta = create_response_meta({
  requestId: "req-123",
  ok: true,
  serverTime: timestamp,
});

assert.equal(meta.requestId, "req-123");
assert.equal(meta.ok, true);
assert.equal(meta.serverTime, timestamp);

// ---------------------------------------------------------------------------
// 8. Proxy type validation (NonEmptyString)
// ---------------------------------------------------------------------------

const validated = validate_non_empty("hello" as NonEmptyString);
assert.equal(validated, "hello");

// Empty string should fail (try_from validation)
assertThrows(
  () => validate_non_empty("" as NonEmptyString),
  "empty string should fail NonEmptyString validation",
);

// ---------------------------------------------------------------------------
// 9. Rich API — get_todo_title, count_completed, filter_by_priority
// ---------------------------------------------------------------------------

const todoA: Todo = create_todo({
  id: 10,
  title: "Alpha",
  completed: true,
  description: null,
  priority: "high",
  tags: [],
  createdAt: timestamp,
  metadata: {},
  extra: {},
});

const todoB: Todo = create_todo({
  id: 20,
  title: "Beta",
  completed: false,
  description: "some desc",
  priority: "low",
  tags: ["test"],
  createdAt: timestamp,
  metadata: {},
  extra: {},
});

const richList: TodoList = apply_command(
  apply_command(
    { name: "Mixed", todos: [] },
    { type: "Add", data: todoA },
  ),
  { type: "Add", data: todoB },
);

assert.equal(get_todo_title(richList, 10), "Alpha");
assert.equal(get_todo_title(richList, 999), "not found");
assert.equal(count_completed(richList), 1);

const highOnly = filter_by_priority(richList, "high");
assert.equal(highOnly.todos.length, 1);
assert.equal(highOnly.todos[0].title, "Alpha");

const lowOnly = filter_by_priority(richList, "low");
assert.equal(lowOnly.todos.length, 1);
assert.equal(lowOnly.todos[0].title, "Beta");

// ---------------------------------------------------------------------------
// 10. Stateful API — dispatch + view with patch_js
// ---------------------------------------------------------------------------

init("Stateful");

// Materialize the initial view.
// We use `any` since `view()` mutates the object in-place and TS cannot
// track property additions from the wasm side.
const vm: Record<string, any> = {};
view(vm);
assert.equal((vm as any).name, "Stateful");
assert.deepEqual((vm as any).todos, []);

// Dispatch an Add command — view should reflect the new todo
dispatch({
  type: "Add",
  data: {
    id: 100,
    title: "Stateful todo",
    completed: false,
    description: null,
    priority: "medium",
    tags: ["state"],
    createdAt: timestamp,
    metadata: {},
    extra: {},
  },
});
view(vm);
assert.equal((vm as any).todos.length, 1);
assert.equal((vm as any).todos[0].title, "Stateful todo");
assert.equal((vm as any).todos[0].completed, false);

// Keep reference to the todos array to verify patch_js preserves identity
const todosRef = (vm as any).todos;

// Toggle the todo — patch_js should update in-place
dispatch({ type: "Toggle", data: { id: 100 } });
view(vm);
assert.equal((vm as any).todos[0].completed, true);
assert.equal((vm as any).todos, todosRef, "patch_js should preserve array identity");

// Remove the todo
dispatch({ type: "Remove", data: { id: 100 } });
view(vm);
assert.equal((vm as any).todos.length, 0);

// ---------------------------------------------------------------------------
// 11. Error handling — invalid inputs should throw
// ---------------------------------------------------------------------------

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

// Invalid ServerEvent variant
assertThrows(
  () => create_event({ event: "unknown", data: {} } as any),
  "unknown server event variant",
);

console.log("ok: all assertions passed");
