use std::process::Command;

fn run_example(path: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_nexa"))
        .args(["run", path, "--workers", "2"])
        .output()
        .expect("nexa CLI should start")
}

#[test]
fn worker_basic_example_runs_through_cli() {
    let output = run_example("examples/worker-basic.nx");

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
}

#[test]
fn control_flow_example_runs_through_cli() {
    let output = run_example("examples/control-flow.nx");

    assert!(
        output.status.success(),
        "CLI failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "15");
}
