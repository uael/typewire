# todo-app

Minimal wasm app demonstrating the typewire end-to-end pipeline:
**derive → compile → extract TypeScript bindings**.

## Build

```sh
# 1. Build the wasm module
cargo build -p todo-app --target wasm32-unknown-unknown --release

# 2. Generate TypeScript declarations from the compiled binary
#    (reads the typewire_schemas section, emits .d.ts, then strips the section)
cargo run -p typewire --features cli -- \
  target/wasm32-unknown-unknown/release/todo_app.wasm \
  -o examples/todo-app/types.d.ts

# 3. Inspect the output
cat examples/todo-app/types.d.ts
```
