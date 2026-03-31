# chat-app

Comprehensive wasm example demonstrating advanced typewire features:
**derive, compile, TypeScript bindings, JS runtime test**.

## What it demonstrates

| Feature | Where |
|---------|-------|
| Transparent newtypes (`#[typewire(transparent)]`) | `UserId`, `ChannelId`, `MessageId`, `Timestamp` |
| Externally tagged enum (default) | `ChannelKind` (all-unit) |
| Internally tagged enum (`tag = "type"`) | `MessageContent` |
| Adjacently tagged enum (`tag` + `content`) | `ReactionEvent`, `ChatCommand`, `ServerEvent` |
| Untagged enum | `ReadReceipt` |
| `rename_all` / `rename_all_fields` | Most types use `camelCase` |
| `rename` on field | `ApiSuccess.success` renamed to `ok` |
| `skip_serializing_if` | Optional fields like `avatarUrl`, `bio`, `editedAt` |
| `default` on container | `SendOptions` with all-optional fields |
| `base64` field encoding | `MessageContent::Image.data`, `MessageContent::File.data` |
| `HashMap<K, V>` | `User.metadata`, `ServerEvent::PresenceBatch.updates` |
| `Vec<T>` | `Channel.memberIds`, `Reaction.userIds`, `Message.thread` |
| `Option<T>` | Many optional fields throughout |
| Tuple struct | `Position(f64, f64)` |
| Nested structs (multiple levels) | `Message` -> `MessageContent` -> `MessageId` |
| Recursive types | `Message.thread: Vec<Message>` |
| Proxy types (`try_from` + `into`) | `NonEmptyString` with validation |
| Error handling across wasm boundary | `Result<T, typewire::Error>` on all exports |
| Paginated response pattern | `PaginatedMessages`, `PaginatedUsers` |

## Build & test

```sh
# 1. Build the wasm module
cargo build -p chat-app --target wasm32-unknown-unknown --release

# 2. Generate TypeScript declarations
cargo run -p typewire --features cli -- \
  target/wasm32-unknown-unknown/release/chat_app.wasm \
  -o examples/chat-app/types.d.ts

# 3. Generate JS bindings and run the test
wasm-bindgen target/wasm32-unknown-unknown/release/chat_app.wasm \
  --out-dir examples/chat-app/pkg --target nodejs --no-typescript
cd examples/chat-app && npm install && npx tsc --noEmit && npx tsx test.ts
```

Or use the full test suite:

```sh
cargo xtask test e2e
```
