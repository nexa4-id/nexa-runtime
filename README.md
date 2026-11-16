# Nexa Runtime

Nexa is a Rust-inspired language and runtime project. This repository is being
rebuilt as a normal Rust crate; source code is tracked directly instead of
being hidden inside uploaded ZIP archives.

## Current bootstrap scope

This clean baseline provides a safe-Rust native worker pool:

- fixed-size worker threads;
- channel-based task dispatch;
- task result delivery;
- panic isolation at the task boundary;
- orderly worker shutdown;
- unit and integration smoke tests.

The Nexa lexer, parser, compiler, bytecode VM, language-level worker submit/wait
syntax, and the nexa run command are not implemented in this bootstrap commit.
They remain the next development milestones; this README deliberately does not
claim otherwise.

## Requirements

- Rust stable toolchain (rustfmt and clippy components)
- Cargo

## Build and verify

    cargo fmt --all -- --check
    cargo check --all-targets
    cargo test --all-targets
    cargo clippy --all-targets -- -D warnings

## Run the native worker smoke demo

    cargo run -- worker-demo
    cargo run -- worker-demo 4

Both commands are intended to print 42.

## Project layout

    .
    ├── .github/workflows/ci.yml
    ├── examples/worker_demo.rs
    ├── src/
    │   ├── lib.rs
    │   ├── main.rs
    │   └── worker.rs
    ├── tests/runtime_smoke.rs
    ├── Cargo.toml
    └── rust-toolchain.toml

## Development policy

- Keep the default branch stable; feature work belongs on a development branch.
- Prefer safe Rust and the standard library until benchmarks justify dependencies.
- Report test and benchmark results only when they were actually run.
- Do not claim language-level worker integration until compiler/runtime tests cover it.
