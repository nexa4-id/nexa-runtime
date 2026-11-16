# Nexa Runtime

Nexa is a Rust-inspired language/runtime project. This branch is the clean
Rust workspace foundation for v0.3 native-worker development.

## Current scope

- A dependency-free native worker pool using one standard-library queue per worker.
- Round-robin task dispatch and single-consumer task result handles.
- Panic containment so a panicking task does not terminate its worker thread.
- Worker counters for submitted, completed, and pending tasks.
- A small CLI worker demonstration.

The language lexer, parser, bytecode compiler, VM integration, and the source-level
syntax `worker.submit { ... }` / `worker.wait(task)` are not implemented in
this clean foundation yet. They must be added as separate, tested compiler/VM
features; this repository does not claim those language features already work.

## Requirements

- Rust stable (edition 2021)
- Cargo

## Build and test

```sh
cargo fmt --all -- --check
cargo check --all-targets
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo build --release
```

## Run the worker demo

```sh
cargo run -- worker-demo --workers 4
```

Expected output:

```text
42
```

## API example

```rust
use nexa_runtime::WorkerPool;

fn main() {
    let pool = WorkerPool::new(4);
    let task = pool.submit(|| 21 * 2);
    assert_eq!(task.wait().unwrap(), 42);
}
```

## Design notes

The worker engine uses only safe Rust and the standard library. Each worker owns
its receive queue, avoiding a shared receiver lock on the scheduling hot path.
Dropping the pool queues shutdown messages and joins every worker thread.

This is an initial v0.3 foundation, not a claim of production readiness or a
completed Nexa language implementation. Benchmark results are intentionally
not published until measured on a documented machine and toolchain.
