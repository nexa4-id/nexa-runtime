use std::env;
use std::process::ExitCode;

use nexa_runtime::WorkerPool;

fn print_help() {
    println!("Nexa runtime 0.3.0");
    println!();
    println!("Usage:");
    println!("  nexa help");
    println!("  nexa worker-demo [workers]");
    println!();
    println!("The language runner is not part of this bootstrap commit yet.");
}

fn run_worker_demo(worker_count: usize) -> Result<(), String> {
    let pool = WorkerPool::new(worker_count).map_err(|error| error.to_string())?;
    let answer = pool
        .execute(|| 21_i64 * 2)
        .map_err(|error| error.to_string())?;
    println!("{answer}");
    Ok(())
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let command = args.next().unwrap_or_else(|| "help".to_string());

    let result = match command.as_str() {
        "help" | "--help" | "-h" => {
            print_help();
            Ok(())
        }
        "worker-demo" => {
            let workers = match args.next() {
                Some(value) => match value.parse::<usize>() {
                    Ok(count) if count > 0 => count,
                    _ => {
                        eprintln!("error: worker count must be a positive integer");
                        return ExitCode::from(2);
                    }
                },
                None => 2,
            };
            run_worker_demo(workers)
        }
        other => Err(format!("unknown command: {other}. Run nexa help.")),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}
