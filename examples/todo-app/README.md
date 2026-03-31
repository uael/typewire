# todo-app

End-to-end example demonstrating the full typewire pipeline:
**derive → compile → typegen → type-check → runtime test**.

## Quick start

```sh
cargo xtask test e2e
```

## Manual steps

```sh
# 1. Build the wasm module
cargo build -p todo-app --target wasm32-unknown-unknown --release

# 2. Generate TypeScript type declarations
cargo run -p typewire --features cli -- \
  target/wasm32-unknown-unknown/release/todo_app.wasm \
  -o examples/todo-app/types.d.ts

# 3. Generate JS/TS bindings
wasm-bindgen target/wasm32-unknown-unknown/release/todo_app.wasm \
  --out-dir examples/todo-app/pkg --target nodejs

# 4. Install deps, type-check, and run the test
cd examples/todo-app && npm install && npx tsc --noEmit && npx tsx test.ts
```
