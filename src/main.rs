use nexa_runtime::{compiler, Vm};
use std::{env, fs, process::ExitCode};
fn main() -> ExitCode {
    let mut a = env::args().skip(1);
    let cmd = a.next().unwrap_or_else(|| "help".into());
    let r = match cmd.as_str() {
        "run" => {
            let path = match a.next() {
                Some(x) => x,
                None => {
                    eprintln!("usage: nexa run <file.nx> [--workers N]");
                    return ExitCode::from(2);
                }
            };
            let mut n = std::thread::available_parallelism()
                .map(|x| x.get())
                .unwrap_or(2);
            while let Some(x) = a.next() {
                if x == "--workers" {
                    n = match a.next().and_then(|v| v.parse::<usize>().ok()) {
                        Some(v) if v > 0 => v,
                        _ => {
                            eprintln!("invalid worker count");
                            return ExitCode::from(2);
                        }
                    }
                }
            }
            match fs::read_to_string(path) {
                Ok(s) => match compiler::compile(&s) {
                    Ok(c) => match Vm::new(n) {
                        Ok(mut vm) => match vm.run(&c) {
                            Ok(lines) => {
                                for line in lines {
                                    println!("{line}")
                                }
                                Ok(())
                            }
                            Err(e) => Err(e),
                        },
                        Err(e) => Err(e.to_string()),
                    },
                    Err(e) => Err(e),
                },
                Err(e) => Err(e.to_string()),
            }
        }
        "help" | "--help" => {
            println!("Nexa 0.3\nUsage: nexa run <file.nx> [--workers N]");
            Ok(())
        }
        _ => Err(format!("unknown command: {cmd}")),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
