# Nexa 0.3

A Rust-hosted language runtime with a lexer, compiler, stack VM, and native worker pool.

## Run

```sh
cargo run -- run examples/worker-basic.nx --workers 4
cargo run -- run examples/control-flow.nx --workers 4
```

Expected output for `worker-basic.nx` is `42`; expected output for `control-flow.nx` is `15`. These are examples and intended outputs; consult CI for verified results.

## Language subset

- Functions: `fn name(params) { ... }`, positional integer arguments, calls, `return expression;`, and recursion.
- Locals: integer `let` declarations, reassignment, lexical block scopes, and nested shadowing.
- Branching: `if`, `else if`, and `else`.
- Loops: `while`, `break`, and `continue`.
- Operators: `== != < <= > >=`, `+ - * /`.
- Output: `print(expression);`.
- Native tasks: `worker.submit { expression }` captures active integer locals by value; `worker.wait(task)` consumes a task handle once.
- Compile-time semantic checks: duplicate declarations/functions/parameters, unknown variables/functions, invalid function arity, and loop control outside a loop are rejected.

## Current limitations

The language is intentionally integer-only. There are no strings, booleans as a separate type, explicit type annotations, closures, modules, structs, exceptions, or break-values. Conditions use zero as false and non-zero as true. Worker task failures are surfaced through `worker.wait`. The current worker implementation creates a small nested pool for worker tasks that themselves execute nested worker operations; this is correct for the supported subset but is a future optimization target.

## Validation

`cargo fmt --all -- --check`, `cargo check --all-targets`, `cargo test --all-targets`, and `cargo clippy --all-targets -- -D warnings` run in the GitHub Actions matrix on Linux, Windows, and macOS.
