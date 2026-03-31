#![cfg(target_arch = "wasm32")]

use std::collections::HashMap;

use typewire::Typewire;
use wasm_bindgen::prelude::*;

// ===========================================================================
// Transparent newtypes
// ===========================================================================

/// Opaque user identifier — serializes as a plain string.
#[derive(Clone, PartialEq, Eq, Hash, Typewire)]
#[typewire(transparent)]
pub struct UserId(String);

/// Opaque channel identifier.
#[derive(Clone, PartialEq, Eq, Hash, Typewire)]
#[typewire(transparent)]
pub struct ChannelId(String);

/// Opaque message identifier.
#[derive(Clone, PartialEq, Eq, Hash, Typewire)]
#[typewire(transparent)]
pub struct MessageId(String);

/// Unix timestamp in milliseconds — serializes as a number.
#[derive(Clone, PartialEq, Typewire)]
#[typewire(transparent)]
pub struct Timestamp(f64);

// ===========================================================================
// Core domain types
// ===========================================================================

/// User presence status — all-unit enum (external tagging → string union).
#[derive(Clone, PartialEq, Eq, Typewire)]
#[typewire(rename_all = "lowercase")]
pub enum UserStatus {
  Online,
  Away,
  DoNotDisturb,
  Offline,
}

/// A chat user with profile and presence data.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct User {
  pub id: UserId,
  pub display_name: String,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub avatar_url: Option<String>,
  pub status: UserStatus,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub bio: Option<String>,
  /// Arbitrary key-value metadata (e.g. timezone, locale).
  pub metadata: HashMap<String, String>,
}

/// Channel kind — externally tagged enum (default).
#[derive(Clone, PartialEq, Eq, Typewire)]
pub enum ChannelKind {
  Direct,
  Group,
  Public,
}

/// A chat channel.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct Channel {
  pub id: ChannelId,
  pub name: String,
  pub kind: ChannelKind,
  pub member_ids: Vec<UserId>,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub topic: Option<String>,
  pub created_at: Timestamp,
}

// ===========================================================================
// Message content — internally tagged enum
// ===========================================================================

/// Content variants inside a message.
/// Demonstrates internally tagged enum (`tag = "type"`).
#[derive(Clone, Typewire)]
#[typewire(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum MessageContent {
  /// Plain text message.
  Text { body: String },
  /// Image with binary data and dimensions.
  Image {
    #[typewire(base64)]
    data: Vec<u8>,
    width: u32,
    height: u32,
    alt_text: Option<String>,
  },
  /// File attachment.
  File {
    file_name: String,
    mime_type: String,
    size_bytes: u32,
    #[typewire(base64)]
    data: Vec<u8>,
  },
  /// A reply referencing another message.
  Reply { parent_id: MessageId, body: String },
  /// System-generated message (join, leave, rename, etc.).
  System { text: String },
}

// ===========================================================================
// Reactions — adjacently tagged enum
// ===========================================================================

/// A reaction event on a message.
/// Demonstrates adjacently tagged enum (`tag` + `content`).
#[derive(Clone, Typewire)]
#[typewire(tag = "action", content = "payload", rename_all_fields = "camelCase")]
pub enum ReactionEvent {
  Add { emoji: String, user_id: UserId },
  Remove { emoji: String, user_id: UserId },
  Clear,
}

/// Aggregated reaction count for a single emoji.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct Reaction {
  pub emoji: String,
  pub count: u32,
  pub user_ids: Vec<UserId>,
}

// ===========================================================================
// Read receipts — untagged enum
// ===========================================================================

/// Read receipt — parsed from either a timestamp number or an object.
/// Demonstrates untagged enum.
#[derive(Clone, Typewire)]
#[typewire(untagged)]
pub enum ReadReceipt {
  /// Simple: just a timestamp.
  Simple(Timestamp),
  /// Detailed: timestamp + device info.
  Detailed { timestamp: Timestamp, device: String },
}

// ===========================================================================
// Message — the main aggregate
// ===========================================================================

/// A single chat message with all nested types.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct Message {
  pub id: MessageId,
  pub channel_id: ChannelId,
  pub sender_id: UserId,
  pub content: MessageContent,
  pub reactions: Vec<Reaction>,
  pub sent_at: Timestamp,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub edited_at: Option<Timestamp>,
  /// Thread replies.
  pub thread: Vec<Message>,
}

// ===========================================================================
// Pagination — generic wrapper (monomorphized for schema)
// ===========================================================================

/// Paginated response wrapper — demonstrates generic struct.
/// Since TypeScript codegen uses monomorphized names, we use concrete
/// instantiations rather than a single generic `Paginated<T>`.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct PaginatedMessages {
  pub items: Vec<Message>,
  pub total_count: u32,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub next_cursor: Option<String>,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub prev_cursor: Option<String>,
}

#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct PaginatedUsers {
  pub items: Vec<User>,
  pub total_count: u32,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub next_cursor: Option<String>,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub prev_cursor: Option<String>,
}

// ===========================================================================
// API response — demonstrates default, flatten, rename
// ===========================================================================

/// Common metadata attached to every API response.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct ResponseMeta {
  pub request_id: String,
  pub server_time: Timestamp,
}

/// Successful API response with nested metadata.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct ApiSuccess {
  #[typewire(rename = "ok")]
  pub success: bool,
  pub meta: ResponseMeta,
  pub data: PaginatedMessages,
}

/// Error details with optional chain.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct ApiErrorDetail {
  pub code: String,
  pub message: String,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub field_name: Option<String>,
}

/// Error API response.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct ApiError {
  #[typewire(rename = "ok")]
  pub success: bool,
  pub meta: ResponseMeta,
  pub errors: Vec<ApiErrorDetail>,
}

// ===========================================================================
// Commands — full event system with skip_serializing_if, defaults
// ===========================================================================

/// Options for sending a message.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase", default)]
pub struct SendOptions {
  pub notify: bool,
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub thread_id: Option<MessageId>,
  pub mentions: Vec<UserId>,
}

impl Default for SendOptions {
  fn default() -> Self {
    Self { notify: true, thread_id: None, mentions: Vec::new() }
  }
}

/// Chat commands — comprehensive enum covering many patterns.
#[derive(Clone, Typewire)]
#[typewire(
  tag = "cmd",
  content = "args",
  rename_all = "camelCase",
  rename_all_fields = "camelCase"
)]
pub enum ChatCommand {
  /// Send a new message to a channel.
  SendMessage { channel_id: ChannelId, content: MessageContent, options: SendOptions },
  /// Edit an existing message.
  EditMessage { message_id: MessageId, content: MessageContent },
  /// Delete a message.
  DeleteMessage { message_id: MessageId },
  /// React to a message.
  React { message_id: MessageId, event: ReactionEvent },
  /// Mark messages as read.
  MarkRead { channel_id: ChannelId, up_to: Timestamp },
  /// Update user profile.
  UpdateProfile { display_name: Option<String>, status: Option<UserStatus>, bio: Option<String> },
  /// Create a new channel.
  CreateChannel { name: String, kind: ChannelKind, member_ids: Vec<UserId> },
  /// Typing indicator.
  Typing { channel_id: ChannelId },
}

// ===========================================================================
// Server events — tuples + nested enums
// ===========================================================================

/// Coordinates for positioning (demonstrates tuple struct).
#[derive(Clone, PartialEq, Typewire)]
pub struct Position(pub f64, pub f64);

/// Typing indicator with position tuple.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct TypingIndicator {
  pub user_id: UserId,
  pub channel_id: ChannelId,
  /// Cursor position (line, column) — demonstrates tuple usage.
  #[typewire(skip_serializing_if = "Option::is_none")]
  pub cursor_position: Option<Position>,
}

/// Server-sent events.
#[derive(Clone, Typewire)]
#[typewire(
  tag = "event",
  content = "data",
  rename_all = "camelCase",
  rename_all_fields = "camelCase"
)]
pub enum ServerEvent {
  /// New message received.
  MessageReceived(Message),
  /// Message was edited.
  MessageEdited { message_id: MessageId, content: MessageContent, edited_at: Timestamp },
  /// Message was deleted.
  MessageDeleted { message_id: MessageId },
  /// User started typing.
  UserTyping(TypingIndicator),
  /// User status changed.
  UserStatusChanged { user_id: UserId, status: UserStatus },
  /// Reaction added or removed.
  ReactionUpdated { message_id: MessageId, event: ReactionEvent },
  /// Channel metadata updated.
  ChannelUpdated(Channel),
  /// Presence update with user-to-status map.
  PresenceBatch { updates: HashMap<String, UserStatus> },
  /// Connection health — no payload.
  Ping,
}

// ===========================================================================
// Proxy types — serde(from/into) and serde(try_from/into)
// ===========================================================================

/// A non-empty string validated at the boundary.
/// Demonstrates `#[serde(try_from = "String", into = "String")]`.
#[derive(Clone, PartialEq, Eq, Typewire)]
#[typewire(try_from = "String", into = "String")]
pub struct NonEmptyString(String);

impl TryFrom<String> for NonEmptyString {
  type Error = &'static str;
  fn try_from(s: String) -> Result<Self, Self::Error> {
    if s.is_empty() { Err("string must not be empty") } else { Ok(Self(s)) }
  }
}

impl From<NonEmptyString> for String {
  fn from(v: NonEmptyString) -> Self {
    v.0
  }
}

/// Channel name that must be non-empty.
#[derive(Clone, Typewire)]
#[typewire(rename_all = "camelCase")]
pub struct CreateChannelRequest {
  pub name: NonEmptyString,
  pub kind: ChannelKind,
  pub member_ids: Vec<UserId>,
}

// ===========================================================================
// Re-export typewire-generated types into the wasm-bindgen .d.ts
// ===========================================================================

#[wasm_bindgen(typescript_custom_section)]
const TS_IMPORTS: &str = r#"import type {
  UserId, ChannelId, MessageId, Timestamp, UserStatus, User, ChannelKind,
  Channel, MessageContent, ReactionEvent, Reaction, ReadReceipt, Message,
  PaginatedMessages, PaginatedUsers, ResponseMeta, ApiSuccess, ApiErrorDetail,
  ApiError, SendOptions, ChatCommand, Position, TypingIndicator, ServerEvent,
  NonEmptyString, CreateChannelRequest
} from '../types.d.ts';"#;

// ===========================================================================
// Exported wasm functions
// ===========================================================================

/// Round-trip a User through the wasm boundary.
///
/// # Errors
///
/// Returns an error if the value is not a valid `User`.
#[expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#[wasm_bindgen(unchecked_return_type = "User")]
pub fn create_user(
  #[wasm_bindgen(unchecked_param_type = "User")] value: User,
) -> Result<User, typewire::Error> {
  Ok(value)
}

/// Round-trip a Message through the wasm boundary.
///
/// # Errors
///
/// Returns an error if the value is not a valid `Message`.
#[expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#[wasm_bindgen(unchecked_return_type = "Message")]
pub fn create_message(
  #[wasm_bindgen(unchecked_param_type = "Message")] value: Message,
) -> Result<Message, typewire::Error> {
  Ok(value)
}

/// Apply a ChatCommand and return a description of what happened.
///
/// # Errors
///
/// Returns an error if the command is not valid.
#[wasm_bindgen]
pub fn describe_command(
  #[wasm_bindgen(unchecked_param_type = "ChatCommand")] cmd: ChatCommand,
) -> Result<String, typewire::Error> {
  let desc = match cmd {
    ChatCommand::SendMessage { channel_id: _, content, options: _ } => match content {
      MessageContent::Text { body } => format!("send text: {body}"),
      MessageContent::Image { width, height, .. } => {
        format!("send image: {width}x{height}")
      }
      MessageContent::File { file_name, .. } => format!("send file: {file_name}"),
      MessageContent::Reply { body, .. } => format!("reply: {body}"),
      MessageContent::System { text } => format!("system: {text}"),
    },
    ChatCommand::EditMessage { message_id: _, content } => match content {
      MessageContent::Text { body } => format!("edit to: {body}"),
      _ => "edit message".to_string(),
    },
    ChatCommand::DeleteMessage { .. } => "delete message".to_string(),
    ChatCommand::React { event, .. } => match event {
      ReactionEvent::Add { emoji, .. } => format!("react: {emoji}"),
      ReactionEvent::Remove { emoji, .. } => format!("unreact: {emoji}"),
      ReactionEvent::Clear => "clear reactions".to_string(),
    },
    ChatCommand::MarkRead { .. } => "mark read".to_string(),
    ChatCommand::UpdateProfile { .. } => "update profile".to_string(),
    ChatCommand::CreateChannel { name, .. } => format!("create channel: {name}"),
    ChatCommand::Typing { .. } => "typing".to_string(),
  };
  Ok(desc)
}

/// Round-trip a ServerEvent.
///
/// # Errors
///
/// Returns an error if the value is not a valid `ServerEvent`.
#[expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#[wasm_bindgen(unchecked_return_type = "ServerEvent")]
pub fn create_event(
  #[wasm_bindgen(unchecked_param_type = "ServerEvent")] value: ServerEvent,
) -> Result<ServerEvent, typewire::Error> {
  Ok(value)
}

/// Round-trip an untagged ReadReceipt.
///
/// # Errors
///
/// Returns an error if the value is not a valid `ReadReceipt`.
#[expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#[wasm_bindgen(unchecked_return_type = "ReadReceipt")]
pub fn create_read_receipt(
  #[wasm_bindgen(unchecked_param_type = "ReadReceipt")] value: ReadReceipt,
) -> Result<ReadReceipt, typewire::Error> {
  Ok(value)
}

/// Round-trip a paginated messages response.
///
/// # Errors
///
/// Returns an error if the value is not valid.
#[expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#[wasm_bindgen(unchecked_return_type = "PaginatedMessages")]
pub fn create_paginated_messages(
  #[wasm_bindgen(unchecked_param_type = "PaginatedMessages")] value: PaginatedMessages,
) -> Result<PaginatedMessages, typewire::Error> {
  Ok(value)
}

/// Round-trip a CreateChannelRequest (exercises proxy type validation).
///
/// # Errors
///
/// Returns an error if the name is empty.
#[expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#[wasm_bindgen(unchecked_return_type = "CreateChannelRequest")]
pub fn create_channel_request(
  #[wasm_bindgen(unchecked_param_type = "CreateChannelRequest")] value: CreateChannelRequest,
) -> Result<CreateChannelRequest, typewire::Error> {
  Ok(value)
}

/// Round-trip an ApiSuccess response (exercises flatten).
///
/// # Errors
///
/// Returns an error if the value is not valid.
#[expect(clippy::missing_const_for_fn, reason = "wasm_bindgen exports cannot be const")]
#[wasm_bindgen(unchecked_return_type = "ApiSuccess")]
pub fn create_api_success(
  #[wasm_bindgen(unchecked_param_type = "ApiSuccess")] value: ApiSuccess,
) -> Result<ApiSuccess, typewire::Error> {
  Ok(value)
}
