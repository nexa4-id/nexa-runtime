use nexa_runtime::WorkerPool;
use std::env;
use std::process::ExitCode;

fn print_help() {
    println!("Nexa Runtime 0.3.0");
    println!();
    println!("Usage:");
    println!("  nexa --version");
    println!("  nexa worker-demo [--workers N]");
}

fn worker_demo(args: &[String]) -> Result<(), String> {
    let mut workers = thread_count_default();
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--workers" {
            let value = args.get(index + 1).ok_or("--workers requires a number")?;
            workers = value.parse::<usize>().map_err(|_| "invalid worker count")?;
            index += 2;
        } else {
            return Err(format!("unknown argument: {}", args[index]));
        }
    }

    let pool = WorkerPool::new(workers);
    let task = pool.submit(|| 21_i64 * 2);
    println!("{}", task.wait().map_err(|error| error.to_string())?);
    Ok(())
}

fn thread_count_default() -> usize {
    std::thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--version") | Some("-V") => {
            println!("nexa {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        Some("worker-demo") => match worker_demo(&args.collect::<Vec<_>>()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error}");
                ExitCode::FAILURE
            }
        },
        Some("help") | Some("--help") | Some("-h") | None => {
            print_help();
            ExitCode::SUCCESS
        }
        Some(command) => {
            eprintln!("unknown command: {command}");
            print_help();
            ExitCode::FAILURE
        }
    }
}
