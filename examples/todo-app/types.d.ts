export type Command =
  | { type: "Add"; data: Todo }
  | { type: "Toggle"; data: { id: number } }
  | { type: "Remove"; data: { id: number } }
  | { type: "SetPriority"; data: { id: number; priority: Priority } }
  | { type: "SendMessage"; data: { todoId: number; content: MessageContent; options: SendOptions } };

export type MessageContent =
  | { type: "text"; body: string }
  | { type: "image"; data: string; width: number; height: number; altText: string | null }
  | { type: "reply"; parentId: MessageId; body: string }
  | { type: "system"; text: string };

export type MessageId = string;

export type NonEmptyString = string;

export type Position = [number, number];

export type Priority = "low" | "medium" | "high";

export type ReactionEvent =
  | { action: "Add"; payload: { emoji: string; userId: UserId } }
  | { action: "Remove"; payload: { emoji: string; userId: UserId } }
  | { action: "Clear" };

export type ReadReceipt =
  | Timestamp
  | { timestamp: Timestamp; device: string };

export interface ResponseMeta {
  requestId: string;
  ok: boolean;
  serverTime: Timestamp;
}

export interface SendOptions {
  notify?: boolean;
  threadId?: MessageId | null;
  mentions?: UserId[];
}

export type ServerEvent =
  | { event: "messageReceived"; data: { todoId: number; content: MessageContent; sentAt: Timestamp } }
  | { event: "userTyping"; data: TypingIndicator }
  | { event: "reactionUpdated"; data: { messageId: MessageId; event: ReactionEvent } }
  | { event: "ping" };

export type Timestamp = number;

export interface Todo {
  id: number;
  title: string;
  completed: boolean;
  description: string | null;
  priority: Priority;
  tags: string[];
  createdAt: Timestamp;
  metadata: Record<string, string>;
  extra: Record<string, any>;
}

export interface TodoList {
  name: string;
  todos: Todo[];
}

export interface TypingIndicator {
  userId: UserId;
  cursorPosition: Position | null;
}

export type UserId = string;

