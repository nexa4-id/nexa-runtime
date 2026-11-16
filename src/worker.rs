use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    Run(Job),
    Shutdown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerError {
    ZeroWorkers,
    WorkerStopped,
    TaskPanicked,
}

impl std::fmt::Display for WorkerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ZeroWorkers => write!(formatter, "worker count must be greater than zero"),
            Self::WorkerStopped => write!(formatter, "worker pool stopped before returning a result"),
            Self::TaskPanicked => write!(formatter, "worker task panicked"),
        }
    }
}

impl std::error::Error for WorkerError {}

/// A small, safe-Rust thread pool used by the Nexa runtime.
pub struct WorkerPool {
    sender: mpsc::Sender<Message>,
    threads: Vec<JoinHandle<()>>,
}

impl WorkerPool {
    /// Creates a pool containing exactly size worker threads.
    pub fn new(size: usize) -> Result<Self, WorkerError> {
        if size == 0 {
            return Err(WorkerError::ZeroWorkers);
        }

        let (sender, receiver) = mpsc::channel::<Message>();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut threads = Vec::with_capacity(size);

        for index in 0..size {
            let thread_receiver = Arc::clone(&receiver);
            let builder = thread::Builder::new().name(format!("nexa-worker-{index}"));
            let handle = builder
                .spawn(move || loop {
                    let message = {
                        let lock = match thread_receiver.lock() {
                            Ok(lock) => lock,
                            Err(_) => break,
                        };
                        lock.recv()
                    };

                    match message {
                        Ok(Message::Run(job)) => job(),
                        Ok(Message::Shutdown) | Err(_) => break,
                    }
                })
                .map_err(|_| WorkerError::WorkerStopped)?;
            threads.push(handle);
        }

        Ok(Self { sender, threads })
    }

    /// Executes a task on the pool and returns its result.
    pub fn execute<F, R>(&self, task: F) -> Result<R, WorkerError>
    where
        F: FnOnce() -> R + Send + 'static,
        R: Send + 'static,
    {
        let (result_sender, result_receiver) = mpsc::channel();
        let job = Box::new(move || {
            let result = catch_unwind(AssertUnwindSafe(task))
                .map_err(|_| WorkerError::TaskPanicked);
            let _ = result_sender.send(result);
        });

        self.sender
            .send(Message::Run(job))
            .map_err(|_| WorkerError::WorkerStopped)?;

        result_receiver
            .recv()
            .map_err(|_| WorkerError::WorkerStopped)?
    }

    pub fn size(&self) -> usize {
        self.threads.len()
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

#[cfg(test)]
mod tests {
    use super::{WorkerError, WorkerPool};

    #[test]
    fn rejects_zero_workers() {
        assert_eq!(WorkerPool::new(0).err(), Some(WorkerError::ZeroWorkers));
    }

    #[test]
    fn executes_task_and_returns_value() {
        let pool = WorkerPool::new(2).expect("pool should start");
        let result = pool.execute(|| 21 * 2).expect("task should complete");
        assert_eq!(result, 42);
    }

    #[test]
    fn contains_task_panics() {
        let pool = WorkerPool::new(1).expect("pool should start");
        let result = pool.execute(|| panic!("intentional test panic"));
        assert_eq!(result, Err(WorkerError::TaskPanicked));

        let next = pool.execute(|| 7).expect("worker should remain available");
        assert_eq!(next, 7);
    }

    #[test]
    fn reports_pool_size() {
        let pool = WorkerPool::new(3).expect("pool should start");
        assert_eq!(pool.size(), 3);
    }
}
