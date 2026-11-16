# Nexa 0.3

A Rust-hosted language runtime with a lexer, compiler, stack VM, and native worker pool.

## Run

```sh
cargo run -- run examples/worker-basic.nx --workers 4
cargo run -- run examples/control-flow.nx --workers 4
```

Expected output for `worker-basic.nx` is `42`; expected output for `control-flow.nx` is `15`. These are examples and intended outputs; consult CI for verified results.

## Language subset

- Functions: `fn name(value: int) -> int { ... }` and `fn name(task: task) -> task { ... }`, positional typed arguments, typed returns, calls, and recursion. Unannotated legacy signatures remain integer-typed; `main` defaults to `int` and may fall through with an implicit zero return.
- Locals: inferred integer values and task handles, reassignment with stable inferred types, lexical block scopes, and nested shadowing.
- Branching: `if`, `else if`, and `else`.
- Loops: `while`, `break`, and `continue`.
- Operators: `== != < <= > >=`, `+ - * /`.
- Output: `print(expression);`.
- Native tasks: `worker.submit { expression }` captures active integer locals by value; `worker.wait(task)` consumes a task handle once.
- Compile-time semantic checks: duplicate declarations/functions/parameters, unknown variables/functions, invalid function arity, loop control outside a loop, task-handle misuse, non-integer operator/print/return/worker results, missing function return paths, and unreachable statements after guaranteed returns are rejected.
- Control-flow validation: non-`main` functions must return an integer on every statically guaranteed path; `if`/`else` branches are analyzed, while loops are not assumed to execute. The VM validates jump targets, stack underflow, typed stack agreement at branch merges, function signatures and argument types, exact return contracts, worker-result contracts, call arity, and local-slot bounds and initialization before execution. Its forward CFG worklist iterates to a fixed point, intersecting local initialization across all incoming paths and rejecting conflicting local types. Worker captures are checked against the initialization state at submission time. Structural checks also reject invalid local loads and nested function definitions, including when they occur in unreachable bytecode.

## Current limitations

The language's computed values are integers and task handles. Function signatures support `int` and `task` parameters and return values; task handles cannot be printed or used in arithmetic, and `worker.wait` consumes them at runtime. There are no strings, booleans as a separate type, explicit local-variable type annotations, closures, modules, structs, exceptions, or break-values. Conditions use zero as false and non-zero as true. Worker task failures are surfaced through `worker.wait`. The current worker implementation creates a small nested pool for worker tasks that themselves execute nested worker operations; this is correct for the supported subset but is a future optimization target.

## Validation

`cargo fmt --all -- --check`, `cargo check --all-targets`, `cargo test --all-targets`, and `cargo clippy --all-targets -- -D warnings` run in the GitHub Actions matrix on Linux, Windows, and macOS.


### Type-aware bytecode verification

Before execution, the VM performs a control-flow data-flow pass over operand-stack and local-slot types. The verifier distinguishes integer values from worker task handles, checks that arithmetic, comparisons, conditions, printing, function arguments, and returns use integers, and requires `worker.wait` to consume a task handle. Branch merges must agree on operand-stack types and cannot merge conflicting initialized local types. Worker bytecode is checked using the captured local type state.


Function bytecode must terminate every reachable path with `Return` and leave exactly one integer matching its declared return signature. Worker blocks use a separate contract: every reachable path must terminate with `Halt` and leave exactly one integer result. Falling through either body, using the other terminator, duplicate parameter names, or calling a function with mismatched argument types is rejected before execution.


### Typed function signatures

Function parameters use `name: int` or `name: task`, and an optional explicit return annotation follows `->`. For example:

```rust
fn make_task() -> task {
    return worker.submit { 21 * 2 };
}

fn wait_task(task: task) -> int {
    return worker.wait(task);
}

fn main() {
    print(wait_task(make_task()));
}
```

The compiler pre-collects signatures before compiling bodies, so forward calls and recursion are type-checked. The VM independently reconstructs the signatures from bytecode and verifies argument types, parameter slots, return types, and task-handle use before execution. Legacy unannotated parameters and return values default to `int`.
