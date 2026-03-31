// End-to-end test: load the wasm module, exercise the exported functions,
// and verify the results conform to the generated TypeScript types.
//
// Type-check:  npx tsc --noEmit
// Run:         npx tsx test.ts

import { createRequire } from "node:module";
import { strict as assert } from "node:assert";

import type {
  UserId, ChannelId, MessageId, Timestamp, UserStatus, User, ChannelKind,
  Channel, MessageContent, ReactionEvent, Reaction, ReadReceipt, Message,
  PaginatedMessages, ResponseMeta, ApiSuccess, ApiErrorDetail, ApiError,
  SendOptions, ChatCommand, Position, TypingIndicator, ServerEvent,
  NonEmptyString, CreateChannelRequest,
} from "./types.d.ts";

import type {
  create_user as CreateUserFn,
  create_message as CreateMessageFn,
  describe_command as DescribeCommandFn,
  create_event as CreateEventFn,
  create_read_receipt as CreateReadReceiptFn,
  create_paginated_messages as CreatePaginatedMessagesFn,
  create_channel_request as CreateChannelRequestFn,
  create_api_success as CreateApiSuccessFn,
} from "./pkg/chat_app.d.ts";

// wasm-bindgen --target nodejs emits CommonJS
const require = createRequire(import.meta.url);
const {
  create_user,
  create_message,
  describe_command,
  create_event,
  create_read_receipt,
  create_paginated_messages,
  create_channel_request,
  create_api_success,
} = require("./pkg/chat_app.js") as {
  create_user: typeof CreateUserFn;
  create_message: typeof CreateMessageFn;
  describe_command: typeof DescribeCommandFn;
  create_event: typeof CreateEventFn;
  create_read_receipt: typeof CreateReadReceiptFn;
  create_paginated_messages: typeof CreatePaginatedMessagesFn;
  create_channel_request: typeof CreateChannelRequestFn;
  create_api_success: typeof CreateApiSuccessFn;
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
// 1. Transparent newtypes + User round-trip
// ---------------------------------------------------------------------------

const userId: UserId = "u-alice-1";
const channelId: ChannelId = "ch-general";
const messageId: MessageId = "msg-001";
const timestamp: Timestamp = 1700000000000;

const user: User = create_user({
  id: userId,
  displayName: "Alice",
  avatarUrl: "https://example.com/alice.png",
  status: "online" satisfies UserStatus,
  bio: "Hello world",
  metadata: { timezone: "UTC", locale: "en-US" },
});

assert.equal(user.id, userId);
assert.equal(user.displayName, "Alice");
assert.equal(user.avatarUrl, "https://example.com/alice.png");
assert.equal(user.status, "online");
assert.equal(user.bio, "Hello world");
assert.equal(user.metadata.timezone, "UTC");
assert.equal(user.metadata.locale, "en-US");

// -- User with optional fields omitted ---

const minimalUser: User = create_user({
  id: "u-bob-2",
  displayName: "Bob",
  avatarUrl: null,
  status: "away" satisfies UserStatus,
  bio: null,
  metadata: {},
});

assert.equal(minimalUser.displayName, "Bob");
assert.equal(minimalUser.avatarUrl, undefined); // skip_serializing_if removes null
assert.equal(minimalUser.status, "away");
assert.equal(minimalUser.bio, undefined);

// ---------------------------------------------------------------------------
// 2. Internally tagged enum (MessageContent)
// ---------------------------------------------------------------------------

const textContent: MessageContent = { type: "text", body: "Hello chat!" };
const imageContent: MessageContent = {
  type: "image",
  data: "AQID", // base64 of [1, 2, 3]
  width: 800,
  height: 600,
  altText: "A photo",
};
const fileContent: MessageContent = {
  type: "file",
  fileName: "doc.pdf",
  mimeType: "application/pdf",
  sizeBytes: 1024,
  data: "BAUG", // base64 of [4, 5, 6]
};
const replyContent: MessageContent = {
  type: "reply",
  parentId: "msg-000",
  body: "I agree!",
};
const systemContent: MessageContent = {
  type: "system",
  text: "Alice joined the channel",
};

// ---------------------------------------------------------------------------
// 3. Message round-trip (nested structs + enums)
// ---------------------------------------------------------------------------

const msg: Message = create_message({
  id: messageId,
  channelId,
  senderId: userId,
  content: textContent,
  reactions: [
    { emoji: "thumbsup", count: 3, userIds: ["u-1", "u-2", "u-3"] } satisfies Reaction,
  ],
  sentAt: timestamp,
  editedAt: null,
  thread: [],
});

assert.equal(msg.id, messageId);
assert.equal(msg.channelId, channelId);
assert.equal(msg.senderId, userId);
assert.deepEqual(msg.content, textContent);
assert.equal(msg.reactions.length, 1);
assert.equal(msg.reactions[0].emoji, "thumbsup");
assert.equal(msg.reactions[0].count, 3);
assert.deepEqual(msg.reactions[0].userIds, ["u-1", "u-2", "u-3"]);
assert.equal(msg.sentAt, timestamp);
assert.equal(msg.editedAt, undefined); // skip_serializing_if
assert.deepEqual(msg.thread, []);

// -- Message with thread (recursive nesting) ---

const threadMsg: Message = create_message({
  id: "msg-002",
  channelId,
  senderId: "u-bob-2",
  content: replyContent,
  reactions: [],
  sentAt: timestamp + 1000,
  editedAt: null,
  thread: [],
});

const parentMsg: Message = create_message({
  id: "msg-001",
  channelId,
  senderId: userId,
  content: textContent,
  reactions: [],
  sentAt: timestamp,
  editedAt: timestamp + 500,
  thread: [threadMsg],
});

assert.equal(parentMsg.thread.length, 1);
assert.equal(parentMsg.thread[0].id, "msg-002");
assert.equal(parentMsg.editedAt, timestamp + 500);

// -- Message with base64 image content ---

const imageMsg: Message = create_message({
  id: "msg-003",
  channelId,
  senderId: userId,
  content: imageContent,
  reactions: [],
  sentAt: timestamp,
  editedAt: null,
  thread: [],
});

assert.equal((imageMsg.content as { type: "image"; data: string }).data, "AQID");

// ---------------------------------------------------------------------------
// 4. ChatCommand — adjacently tagged with rename_all + rename_all_fields
// ---------------------------------------------------------------------------

const sendCmd: ChatCommand = {
  cmd: "sendMessage",
  args: {
    channelId,
    content: textContent,
    options: {
      notify: true,
      threadId: null,
      mentions: ["u-bob-2"],
    } satisfies SendOptions,
  },
};

const desc = describe_command(sendCmd);
assert.equal(desc, "send text: Hello chat!");

// -- Edit command ---

const editDesc = describe_command({
  cmd: "editMessage",
  args: {
    messageId,
    content: { type: "text", body: "Updated text" },
  },
});
assert.equal(editDesc, "edit to: Updated text");

// -- Delete command ---

const deleteDesc = describe_command({
  cmd: "deleteMessage",
  args: { messageId },
});
assert.equal(deleteDesc, "delete message");

// -- React command with adjacently tagged ReactionEvent ---

const reactDesc = describe_command({
  cmd: "react",
  args: {
    messageId,
    event: {
      action: "Add",
      payload: { emoji: "heart", userId },
    } satisfies ReactionEvent,
  },
});
assert.equal(reactDesc, "react: heart");

// -- React clear ---

const clearReactDesc = describe_command({
  cmd: "react",
  args: {
    messageId,
    event: { action: "Clear" } satisfies ReactionEvent,
  },
});
assert.equal(clearReactDesc, "clear reactions");

// -- Typing command ---

const typingDesc = describe_command({
  cmd: "typing",
  args: { channelId },
});
assert.equal(typingDesc, "typing");

// -- Create channel ---

const createDesc = describe_command({
  cmd: "createChannel",
  args: {
    name: "new-channel",
    kind: "Group" satisfies ChannelKind,
    memberIds: [userId, "u-bob-2"],
  },
});
assert.equal(createDesc, "create channel: new-channel");

// -- Update profile ---

const profileDesc = describe_command({
  cmd: "updateProfile",
  args: {
    displayName: "Alice B.",
    status: "donotdisturb" satisfies UserStatus,
    bio: null,
  },
});
assert.equal(profileDesc, "update profile");

// -- Mark read ---

const markReadDesc = describe_command({
  cmd: "markRead",
  args: { channelId, upTo: timestamp },
});
assert.equal(markReadDesc, "mark read");

// -- SendOptions with defaults (all fields optional) ---

const defaultOptionsCmd: ChatCommand = {
  cmd: "sendMessage",
  args: {
    channelId,
    content: textContent,
    options: {} as SendOptions, // all fields have defaults
  },
};

const defaultDesc = describe_command(defaultOptionsCmd);
assert.equal(defaultDesc, "send text: Hello chat!");

// ---------------------------------------------------------------------------
// 5. ServerEvent — adjacently tagged with various payloads
// ---------------------------------------------------------------------------

// -- messageReceived ---
const msgEvent: ServerEvent = create_event({
  event: "messageReceived",
  data: msg,
});
assert.equal(msgEvent.event, "messageReceived");

// -- messageEdited ---
const editEvent: ServerEvent = create_event({
  event: "messageEdited",
  data: {
    messageId,
    content: { type: "text", body: "Edited!" },
    editedAt: timestamp + 1000,
  },
});
assert.equal(editEvent.event, "messageEdited");

// -- messageDeleted ---
const deleteEvent: ServerEvent = create_event({
  event: "messageDeleted",
  data: { messageId },
});
assert.equal(deleteEvent.event, "messageDeleted");

// -- userTyping with Position tuple ---
const typingEvent: ServerEvent = create_event({
  event: "userTyping",
  data: {
    userId,
    channelId,
    cursorPosition: [10, 25] satisfies Position,
  } satisfies TypingIndicator,
});
assert.equal(typingEvent.event, "userTyping");

// -- userTyping without position ---
const typingNoPos: ServerEvent = create_event({
  event: "userTyping",
  data: {
    userId,
    channelId,
    cursorPosition: null,
  } satisfies TypingIndicator,
});
assert.equal(typingNoPos.event, "userTyping");

// -- userStatusChanged ---
const statusEvent: ServerEvent = create_event({
  event: "userStatusChanged",
  data: {
    userId,
    status: "offline" satisfies UserStatus,
  },
});
assert.equal(statusEvent.event, "userStatusChanged");

// -- reactionUpdated ---
const reactionEvent: ServerEvent = create_event({
  event: "reactionUpdated",
  data: {
    messageId,
    event: { action: "Remove", payload: { emoji: "heart", userId } },
  },
});
assert.equal(reactionEvent.event, "reactionUpdated");

// -- channelUpdated ---
const channelEvent: ServerEvent = create_event({
  event: "channelUpdated",
  data: {
    id: channelId,
    name: "general",
    kind: "Public" satisfies ChannelKind,
    memberIds: [userId],
    topic: "General discussion",
    createdAt: timestamp,
  } satisfies Channel,
});
assert.equal(channelEvent.event, "channelUpdated");

// -- presenceBatch with HashMap ---
const presenceEvent: ServerEvent = create_event({
  event: "presenceBatch",
  data: {
    updates: {
      "u-alice-1": "online" satisfies UserStatus,
      "u-bob-2": "away" satisfies UserStatus,
    },
  },
});
assert.equal(presenceEvent.event, "presenceBatch");

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
// 7. PaginatedMessages — pagination wrapper
// ---------------------------------------------------------------------------

const paginated: PaginatedMessages = create_paginated_messages({
  items: [msg],
  totalCount: 42,
  nextCursor: "cursor-abc",
  prevCursor: null,
});

assert.equal(paginated.items.length, 1);
assert.equal(paginated.totalCount, 42);
assert.equal(paginated.nextCursor, "cursor-abc");
assert.equal(paginated.prevCursor, undefined); // skip_serializing_if

// -- Empty page ---
const emptyPage: PaginatedMessages = create_paginated_messages({
  items: [],
  totalCount: 0,
  nextCursor: null,
  prevCursor: null,
});
assert.equal(emptyPage.items.length, 0);
assert.equal(emptyPage.totalCount, 0);

// ---------------------------------------------------------------------------
// 8. Proxy type validation (NonEmptyString via CreateChannelRequest)
// ---------------------------------------------------------------------------

const channelReq: CreateChannelRequest = create_channel_request({
  name: "my-channel" as NonEmptyString,
  kind: "Group" satisfies ChannelKind,
  memberIds: [userId, "u-bob-2"],
});

assert.equal(channelReq.name, "my-channel");
assert.equal(channelReq.kind, "Group");
assert.deepEqual(channelReq.memberIds, [userId, "u-bob-2"]);

// -- Empty name should fail (try_from validation) ---
assertThrows(
  () => create_channel_request({
    name: "" as NonEmptyString,
    kind: "Direct" satisfies ChannelKind,
    memberIds: [],
  }),
  "empty name should fail NonEmptyString validation",
);

// ---------------------------------------------------------------------------
// 9. ApiSuccess — rename + nested structs
// ---------------------------------------------------------------------------

const apiSuccess: ApiSuccess = create_api_success({
  ok: true,
  meta: {
    requestId: "req-123",
    serverTime: timestamp,
  } satisfies ResponseMeta,
  data: paginated,
});

assert.equal(apiSuccess.ok, true);
assert.equal(apiSuccess.meta.requestId, "req-123");
assert.equal(apiSuccess.meta.serverTime, timestamp);
assert.equal(apiSuccess.data.totalCount, 42);

// ---------------------------------------------------------------------------
// 10. Error handling — invalid inputs should throw
// ---------------------------------------------------------------------------

// Missing required field
assertThrows(
  () => create_user({ id: "x", status: "online" } as any),
  "missing required field 'displayName'",
);

// Wrong type
assertThrows(
  () => create_user(42 as any),
  "number instead of User object",
);

// null input
assertThrows(
  () => create_message(null as any),
  "null instead of Message object",
);

// Invalid enum variant
assertThrows(
  () => describe_command({ cmd: "unknown", args: {} } as any),
  "unknown command variant",
);

// Invalid MessageContent type
assertThrows(
  () => create_message({
    id: "x",
    channelId: "c",
    senderId: "u",
    content: { type: "invalid" } as any,
    reactions: [],
    sentAt: 0,
    editedAt: null,
    thread: [],
  }),
  "unknown message content type",
);

// Invalid UserStatus in ServerEvent
assertThrows(
  () => create_event({
    event: "userStatusChanged",
    data: { userId: "u-1", status: "invalid" as any },
  }),
  "unknown user status variant",
);

console.log("ok: all assertions passed");
