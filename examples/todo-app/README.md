# todo-app

Minimal wasm app demonstrating the typewire end-to-end pipeline:
**derive → compile → TypeScript bindings → JS runtime test**.

## Build & test

```sh
# 1. Build the wasm module
cargo build -p todo-app --target wasm32-unknown-unknown --release

# 2. Generate TypeScript declarations
cargo run -p typewire --features cli -- \
  target/wasm32-unknown-unknown/release/todo_app.wasm \
  --no-strip -o examples/todo-app/types.d.ts

# 3. Generate JS bindings and run the test
wasm-bindgen target/wasm32-unknown-unknown/release/todo_app.wasm \
  --out-dir examples/todo-app/pkg --target nodejs --no-typescript
node examples/todo-app/test.mjs
```
