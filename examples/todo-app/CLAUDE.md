# todo-app

End-to-end example and practical guide demonstrating the full typewire pipeline. Designed as a comprehensive showcase of Typewire's API surface while remaining a readable, coherent app.

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
| `README.md` | User-facing guide: quick start, attributes, type mapping, patterns |

## Running

```sh
cargo xtask test e2e
```

Or manually:

```sh
cargo build -p todo-app --target wasm32-unknown-unknown --release
cargo run -p typewire --features cli -- target/wasm32-unknown-unknown/release/todo_app.wasm -o examples/todo-app/types.d.ts
wasm-bindgen target/wasm32-unknown-unknown/release/todo_app.wasm --out-dir examples/todo-app/pkg --target nodejs
cd examples/todo-app && npm install && npx tsc --noEmit && npx tsx test.ts
```

## Notes

- Requires `typewire/schemas` feature to embed schema records (see `Cargo.toml`)
- `types.gen.d.ts` is generated at test time and git-ignored
- The CLI strips the `typewire_schemas` section from the wasm binary by default

## What It Tests

- Transparent newtypes (`UserId`, `MessageId`, `Timestamp`)
- Externally tagged all-unit enum (`Priority`)
- Internally tagged enum with `rename_all_fields` (`MessageContent`)
- Adjacently tagged enum (`Command`, `ServerEvent`, `ReactionEvent`)
- Untagged enum (`ReadReceipt`)
- `rename_all = "camelCase"` on structs and enums
- `rename` on individual fields (`ResponseMeta.success` -> `ok`)
- `skip_serializing_if` on optional fields
- `default` on container (`SendOptions`)
- `base64` field encoding (`MessageContent::Image.data`)
- `HashMap<K, V>` (`Todo.metadata`)
- `Vec<T>` in many places
- `Option<T>` throughout
- Tuple struct (`Position`)
- Proxy types with validation (`NonEmptyString` via `try_from` + `into`)
- Error handling across wasm boundary (`Result<T, typewire::Error>`)
