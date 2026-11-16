use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;
enum Message { Run(Job), Shutdown }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkerError { ZeroWorkers, WorkerStopped, TaskPanicked, TaskFailed(String) }
impl std::fmt::Display for WorkerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self { Self::TaskFailed(message) => write!(f, "task failed: {message}"), other => write!(f, "{other:?}") }
    }
}
impl std::error::Error for WorkerError {}

pub struct TaskHandle<R> { receiver: mpsc::Receiver<Result<R, WorkerError>> }
impl<R> TaskHandle<R> {
    pub fn wait(self) -> Result<R, WorkerError> {
        self.receiver.recv().map_err(|_| WorkerError::WorkerStopped)?
    }
}

pub struct WorkerPool { sender: mpsc::Sender<Message>, threads: Vec<JoinHandle<()>> }
impl WorkerPool {
    pub fn new(count: usize) -> Result<Self, WorkerError> {
        if count == 0 { return Err(WorkerError::ZeroWorkers); }
        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut threads = Vec::with_capacity(count);
        for index in 0..count {
            let shared = Arc::clone(&receiver);
            let thread = thread::Builder::new().name(format!("nexa-worker-{index}"))
                .spawn(move || loop {
                    let message = {
                        let guard = match shared.lock() { Ok(guard) => guard, Err(_) => break };
                        guard.recv()
                    };
                    match message { Ok(Message::Run(job)) => job(), Ok(Message::Shutdown) | Err(_) => break }
                }).map_err(|_| WorkerError::WorkerStopped)?;
            threads.push(thread);
        }
        Ok(Self { sender, threads })
    }
    pub fn submit<F, R>(&self, task: F) -> Result<TaskHandle<R>, WorkerError>
    where F: FnOnce() -> Result<R, WorkerError> + Send + 'static, R: Send + 'static {
        let (sender, receiver) = mpsc::channel();
        let job = Box::new(move || {
            let result = catch_unwind(AssertUnwindSafe(task)).unwrap_or(Err(WorkerError::TaskPanicked));
            let _ = sender.send(result);
        });
        self.sender.send(Message::Run(job)).map_err(|_| WorkerError::WorkerStopped)?;
        Ok(TaskHandle { receiver })
    }
    pub fn execute<F, R>(&self, task: F) -> Result<R, WorkerError>
    where F: FnOnce() -> R + Send + 'static, R: Send + 'static {
        self.submit(move || Ok(task()))?.wait()
    }
    pub fn size(&self) -> usize { self.threads.len() }
}
impl Drop for WorkerPool {
    fn drop(&mut self) {
        for _ in 0..self.threads.len() { let _ = self.sender.send(Message::Shutdown); }
        for thread in self.threads.drain(..) { let _ = thread.join(); }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn task_returns_value() { let pool=WorkerPool::new(2).unwrap(); assert_eq!(pool.execute(||42).unwrap(),42); }
    #[test] fn panic_is_contained() { let pool=WorkerPool::new(1).unwrap(); assert_eq!(pool.execute(||panic!("test")),Err(WorkerError::TaskPanicked)); assert_eq!(pool.execute(||7).unwrap(),7); }
    #[test] fn zero_workers_rejected() { assert_eq!(WorkerPool::new(0).err(),Some(WorkerError::ZeroWorkers)); }
}