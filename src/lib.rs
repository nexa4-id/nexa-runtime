//! Nexa runtime foundation.
//!
//! The worker pool is the first native runtime subsystem. The language
//! frontend and bytecode VM are added in subsequent implementation steps.

pub mod worker;

pub use worker::{WorkerError, WorkerPool};
