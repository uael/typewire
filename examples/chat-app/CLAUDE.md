# chat-app

Comprehensive end-to-end example demonstrating advanced typewire features.

## Pipeline

```
#[derive(Typewire)] types  ->  cargo build --target wasm32  ->  typewire CLI  ->  types.d.ts
                                                                                      |
                                                                                tsc --noEmit
                                                                                      |
                                                                             npx tsx test.ts
```

## Files

| File | Purpose |
|------|---------|
| `src/lib.rs` | Rust types with `#[derive(Typewire)]` + wasm-bindgen exports |
| `types.d.ts` | Checked-in TypeScript snapshot (e2e diffs against regenerated `types.gen.d.ts`) |
| `test.ts` | TypeScript test importing generated types + runtime assertions |
| `tsconfig.json` | TypeScript config for strict type-checking |
| `package.json` | Dev deps: `typescript`, `tsx`, `@types/node` |

## Running

```sh
cargo xtask test e2e
```

Or manually:

```sh
cargo build -p chat-app --target wasm32-unknown-unknown --release
cargo run -p typewire --features cli -- target/wasm32-unknown-unknown/release/chat_app.wasm -o examples/chat-app/types.d.ts
wasm-bindgen target/wasm32-unknown-unknown/release/chat_app.wasm --out-dir examples/chat-app/pkg --target nodejs
cd examples/chat-app && npm install && npx tsc --noEmit && npx tsx test.ts
```

## What It Tests

- Transparent newtypes (`UserId`, `ChannelId`, `MessageId`, `Timestamp`)
- Externally tagged all-unit enum (`ChannelKind`)
- Internally tagged enum with `rename_all_fields` (`MessageContent`)
- Adjacently tagged enum with `rename_all` + `rename_all_fields` (`ChatCommand`, `ServerEvent`, `ReactionEvent`)
- Untagged enum (`ReadReceipt`)
- `rename_all = "camelCase"` on structs and enums
- `rename` on individual fields (`ApiSuccess.success` -> `ok`)
- `skip_serializing_if` on optional fields
- `default` on container (`SendOptions`)
- `base64` field encoding (`MessageContent::Image.data`)
- `HashMap<K, V>` (`User.metadata`, `ServerEvent::PresenceBatch.updates`)
- `Vec<T>` in many places
- `Option<T>` throughout
- Tuple struct (`Position`)
- Nested structs (multiple levels: `Message` -> `MessageContent` -> `MessageId`)
- Recursive types (`Message.thread: Vec<Message>`)
- Proxy types with validation (`NonEmptyString` via `try_from` + `into`)
- Error handling across wasm boundary (`Result<T, typewire::Error>`)
- Paginated response patterns (`PaginatedMessages`, `PaginatedUsers`)
