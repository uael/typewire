# todo-app

End-to-end example demonstrating the full typewire pipeline.

## Pipeline

```
#[derive(Typewire)] types  →  cargo build --target wasm32  →  typewire CLI  →  types.d.ts
                                                                                    ↓
                                                                              tsc --noEmit
                                                                                    ↓
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

- Struct with `rename_all`, optional fields, nested types
- Enum with adjacent tagging (`tag = "type"`, `content = "data"`)
- Transparent newtype
- TypeScript type-checking catches mismatches between generated types and test usage
- Node.js runtime verifies `create_todo` and `apply_command` round-trip correctly
