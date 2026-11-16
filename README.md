# Nexa 0.3

A small Rust-hosted language runtime. This milestone adds a lexer, expression parser/compiler, stack VM, and language-level native worker submission/wait.

## Run

```sh
cargo run -- run examples/worker-basic.nx --workers 4
```

Expected output: `42`.

## Validate

```sh
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Supported language subset: `fn main() { ... }`, integer `let` bindings, `print(expr)`, integer arithmetic, `worker.submit { expr }`, and `worker.wait(task)`. Worker blocks capture currently defined integer locals by value at submission time. Handles are single-use. The parser/compiler and VM are intentionally small; functions, strings, types, modules, and general control flow are not implemented yet.
