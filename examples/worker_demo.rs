use nexa_runtime::WorkerPool;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let pool = WorkerPool::new(2)?;
    let value = pool.execute(|| 21 * 2)?;
    println!("{value}");
    Ok(())
}
