//! Nexa runtime foundation.
//!
//! The language frontend is intentionally kept separate from the native worker
//! engine. This crate currently provides the worker execution API used by the
//! upcoming compiler/VM integration.

pub mod worker;

pub use worker::{TaskError, TaskHandle, WorkerPool, WorkerStats};

#[cfg(test)]
mod tests {
    use super::{TaskError, WorkerPool};

    #[test]
    fn executes_and_returns_task_result() {
        let pool = WorkerPool::new(2);
        let task = pool.submit(|| 21 * 2);
        assert_eq!(task.wait().unwrap(), 42);
    }

    #[test]
    fn contains_task_panics() {
        let pool = WorkerPool::new(1);
        let task = pool.submit(|| -> usize { panic!("expected test panic") });
        assert_eq!(task.wait(), Err(TaskError::Panicked));
        let next = pool.submit(|| 7);
        assert_eq!(next.wait().unwrap(), 7);
    }

    #[test]
    fn tracks_completed_work() {
        let pool = WorkerPool::new(2);
        let tasks = (0..8).map(|value| pool.submit(move || value)).collect::<Vec<_>>();
        let values = tasks.into_iter().map(|task| task.wait().unwrap()).collect::<Vec<_>>();
        assert_eq!(values, (0..8).collect::<Vec<_>>());
        assert_eq!(pool.stats().completed, 8);
        assert_eq!(pool.stats().pending, 0);
    }

    #[test]
    fn uses_at_least_one_worker() {
        let pool = WorkerPool::new(0);
        assert_eq!(pool.worker_count(), 1);
        assert_eq!(pool.submit(|| 1).wait().unwrap(), 1);
    }
}
