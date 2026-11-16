# Nexa 0.3
A Rust-hosted language runtime with a lexer, parser/compiler, stack VM, and native worker pool.

## Run
```sh
cargo run -- run examples/worker-basic.nx --workers 4
cargo run -- run examples/control-flow.nx --workers 4
```
The worker example is intended to print `42`; the control-flow example prints `15`.

## Language subset
- `fn name(params) { ... }`, calls, and `return expression;` (integer arguments/results; recursion supported).
- Integer `let` declarations and reassignment of existing locals.
- `if (condition) { ... } else { ... }`, `while (condition) { ... }`.
- Comparisons `== != < <= > >=`, arithmetic `+ - * /`, and `print(expr);`.
- `worker.submit { expression }` captures integer locals by value; `worker.wait(task)` consumes a task handle once.

This remains a deliberately small integer language: no strings, static type system, closures, break/continue, or modules yet.
