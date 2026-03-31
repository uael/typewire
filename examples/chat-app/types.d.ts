export interface ApiError {
  ok: boolean;
  meta: ResponseMeta;
  errors: ApiErrorDetail[];
}

export interface ApiErrorDetail {
  code: string;
  message: string;
  fieldName: string | null;
}

export interface ApiSuccess {
  ok: boolean;
  meta: ResponseMeta;
  data: PaginatedMessages;
}

export interface Channel {
  id: ChannelId;
  name: string;
  kind: ChannelKind;
  memberIds: UserId[];
  topic: string | null;
  createdAt: Timestamp;
}

export type ChannelId = string;

export type ChannelKind = "Direct" | "Group" | "Public";

export type ChatCommand =
  | { cmd: "sendMessage"; args: { channelId: ChannelId; content: MessageContent; options: SendOptions } }
  | { cmd: "editMessage"; args: { messageId: MessageId; content: MessageContent } }
  | { cmd: "deleteMessage"; args: { messageId: MessageId } }
  | { cmd: "react"; args: { messageId: MessageId; event: ReactionEvent } }
  | { cmd: "markRead"; args: { channelId: ChannelId; upTo: Timestamp } }
  | { cmd: "updateProfile"; args: { displayName: string | null; status: UserStatus | null; bio: string | null } }
  | { cmd: "createChannel"; args: { name: string; kind: ChannelKind; memberIds: UserId[] } }
  | { cmd: "typing"; args: { channelId: ChannelId } };

export interface CreateChannelRequest {
  name: NonEmptyString;
  kind: ChannelKind;
  memberIds: UserId[];
}

export interface Message {
  id: MessageId;
  channelId: ChannelId;
  senderId: UserId;
  content: MessageContent;
  reactions: Reaction[];
  sentAt: Timestamp;
  editedAt: Timestamp | null;
  thread: Message[];
}

export type MessageContent =
  | { type: "text"; body: string }
  | { type: "image"; data: string; width: number; height: number; altText: string | null }
  | { type: "file"; fileName: string; mimeType: string; sizeBytes: number; data: string }
  | { type: "reply"; parentId: MessageId; body: string }
  | { type: "system"; text: string };

export type MessageId = string;

export type NonEmptyString = string;

export interface PaginatedMessages {
  items: Message[];
  totalCount: number;
  nextCursor: string | null;
  prevCursor: string | null;
}

export interface PaginatedUsers {
  items: User[];
  totalCount: number;
  nextCursor: string | null;
  prevCursor: string | null;
}

export type Position = [number, number];

export interface Reaction {
  emoji: string;
  count: number;
  userIds: UserId[];
}

export type ReactionEvent =
  | { action: "Add"; payload: { emoji: string; userId: UserId } }
  | { action: "Remove"; payload: { emoji: string; userId: UserId } }
  | { action: "Clear" };

export type ReadReceipt =
  | Timestamp
  | { timestamp: Timestamp; device: string };

export interface ResponseMeta {
  requestId: string;
  serverTime: Timestamp;
}

export interface SendOptions {
  notify?: boolean;
  threadId?: MessageId | null;
  mentions?: UserId[];
}

export type ServerEvent =
  | { event: "messageReceived"; data: Message }
  | { event: "messageEdited"; data: { messageId: MessageId; content: MessageContent; editedAt: Timestamp } }
  | { event: "messageDeleted"; data: { messageId: MessageId } }
  | { event: "userTyping"; data: TypingIndicator }
  | { event: "userStatusChanged"; data: { userId: UserId; status: UserStatus } }
  | { event: "reactionUpdated"; data: { messageId: MessageId; event: ReactionEvent } }
  | { event: "channelUpdated"; data: Channel }
  | { event: "presenceBatch"; data: { updates: Record<string, UserStatus> } }
  | { event: "ping" };

export type Timestamp = number;

export interface TypingIndicator {
  userId: UserId;
  channelId: ChannelId;
  cursorPosition: Position | null;
}

export interface User {
  id: UserId;
  displayName: string;
  avatarUrl: string | null;
  status: UserStatus;
  bio: string | null;
  metadata: Record<string, string>;
}

export type UserId = string;

export type UserStatus = "online" | "away" | "donotdisturb" | "offline";

