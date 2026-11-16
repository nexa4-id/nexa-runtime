//! Safe, dependency-free native worker pool.

use std::any::Any;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    Run(Job),
    Shutdown,
}

/// A failure returned by a native worker task.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskError {
    /// The task panicked. The panic is contained inside the worker thread.
    Panicked,
    /// The worker stopped before delivering a result.
    WorkerDisconnected,
}

impl fmt::Display for TaskError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Panicked => formatter.write_str("worker task panicked"),
            Self::WorkerDisconnected => formatter.write_str("worker disconnected before returning a result"),
        }
    }
}

impl std::error::Error for TaskError {}

/// A single-consumer result handle. Calling wait consumes the handle.
pub struct TaskHandle<T> {
    receiver: mpsc::Receiver<Result<T, TaskError>>,
}

impl<T> TaskHandle<T> {
    /// Wait for the task and retrieve its result.
    pub fn wait(self) -> Result<T, TaskError> {
        self.receiver.recv().unwrap_or(Err(TaskError::WorkerDisconnected))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkerStats {
    pub submitted: usize,
    pub completed: usize,
    pub pending: usize,
}

/// A fixed-size pool of native worker threads.
pub struct WorkerPool {
    sender: mpsc::Sender<Message>,
    threads: Vec<JoinHandle<()>>,
    submitted: Arc<AtomicUsize>,
    completed: Arc<AtomicUsize>,
    pending: Arc<AtomicUsize>,
}

impl WorkerPool {
    /// Create a pool. A zero-sized request is normalized to one worker.
    pub fn new(worker_count: usize) -> Self {
        let worker_count = worker_count.max(1);
        let (sender, receiver) = mpsc::channel::<Message>();
        let receiver = Arc::new(Mutex::new(receiver));
        let submitted = Arc::new(AtomicUsize::new(0));
        let completed = Arc::new(AtomicUsize::new(0));
        let pending = Arc::new(AtomicUsize::new(0));
        let mut threads = Vec::with_capacity(worker_count);

        for index in 0..worker_count {
            let receiver = Arc::clone(&receiver);
            let completed_count = Arc::clone(&completed);
            let pending_count = Arc::clone(&pending);
            let name = format!("nexa-worker-{index}");
            let handle = thread::Builder::new()
                .name(name)
                .spawn(move || loop {
                    let message = {
                        let guard = match receiver.lock() {
                            Ok(guard) => guard,
                            Err(_) => return,
                        };
                        guard.recv()
                    };

                    match message {
                        Ok(Message::Run(job)) => {
                            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(job));
                            pending_count.fetch_sub(1, Ordering::AcqRel);
                            completed_count.fetch_add(1, Ordering::Relaxed);
                        }
                        Ok(Message::Shutdown) | Err(_) => break,
                    }
                })
                .expect("failed to spawn Nexa worker thread");
            threads.push(handle);
        }

        Self {
            sender,
            threads,
            submitted,
            completed,
            pending,
        }
    }

    /// Submit a Send + 'static task to the worker pool.
    pub fn submit<F, T>(&self, task: F) -> TaskHandle<T>
    where
        F: FnOnce() -> T + Send + 'static,
        T: Send + 'static,
    {
        let (result_sender, result_receiver) = mpsc::channel();
        self.submitted.fetch_add(1, Ordering::Relaxed);
        self.pending.fetch_add(1, Ordering::AcqRel);

        let job = Box::new(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(task))
                .map_err(|payload: Box<dyn Any + Send>| {
                    drop(payload);
                    TaskError::Panicked
                });
            let _ = result_sender.send(result);
        });

        if self.sender.send(Message::Run(job)).is_err() {
            self.pending.fetch_sub(1, Ordering::AcqRel);
        }

        TaskHandle {
            receiver: result_receiver,
        }
    }

    pub fn worker_count(&self) -> usize {
        self.threads.len()
    }

    pub fn stats(&self) -> WorkerStats {
        WorkerStats {
            submitted: self.submitted.load(Ordering::Relaxed),
            completed: self.completed.load(Ordering::Relaxed),
            pending: self.pending.load(Ordering::Acquire),
        }
    }
}

impl Drop for WorkerPool {
    fn drop(&mut self) {
        for _ in 0..self.threads.len() {
            let _ = self.sender.send(Message::Shutdown);
        }
        for handle in self.threads.drain(..) {
            let _ = handle.join();
        }
    }
}
