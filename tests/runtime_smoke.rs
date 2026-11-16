use nexa_runtime::{WorkerError, WorkerPool};

#[test]
fn worker_pool_runs_independent_tasks() {
    let pool = WorkerPool::new(4).expect("pool should start");

    let first = pool.execute(|| 20 + 22).expect("first task should finish");
    let second = pool.execute(|| 6 * 7).expect("second task should finish");

    assert_eq!(first, 42);
    assert_eq!(second, 42);
}

#[test]
fn worker_panic_is_reported_without_killing_the_pool() {
    let pool = WorkerPool::new(1).expect("pool should start");

    assert_eq!(
        pool.execute(|| -> () { panic!("expected panic") }),
        Err(WorkerError::TaskPanicked)
    );
    assert_eq!(pool.execute(|| 9).expect("pool should still work"), 9);
}
