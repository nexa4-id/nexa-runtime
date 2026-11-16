use nexa_runtime::{compiler, Vm};

fn run(source: &str) -> Vec<String> {
    let code = compiler::compile(source).expect("source should compile");
    let mut vm = Vm::new(2).expect("worker pool should start");
    vm.run(&code).expect("program should run").to_vec()
}

#[test]
fn if_else_and_else_if() {
    assert_eq!(
        run("fn main(){if(1==0){print(0);}else if(2==2){print(2);}else{print(3);}}"),
        vec!["2"]
    );
}

#[test]
fn while_loop() {
    assert_eq!(
        run("fn main(){let i=0;while(i<3){print(i);i=i+1;}}"),
        vec!["0", "1", "2"]
    );
}

#[test]
fn break_and_continue() {
    assert_eq!(
        run("fn main(){let i=0;while(i<5){i=i+1;if(i==2){continue;}if(i==4){break;}print(i);}}"),
        vec!["1", "3"]
    );
}

#[test]
fn nested_scope_can_shadow_but_does_not_leak() {
    assert_eq!(
        run("fn main(){let x=1;if(1){let x=2;print(x);}print(x);}"),
        vec!["2", "1"]
    );
    assert!(compiler::compile("fn main(){if(1){let hidden=7;}print(hidden);}").is_err());
}

#[test]
fn same_scope_duplicate_is_rejected() {
    assert!(compiler::compile("fn main(){let x=1;let x=2;}")
        .unwrap_err()
        .contains("duplicate local"));
}

#[test]
fn function_arguments_return_and_recursion() {
    assert_eq!(run("fn twice(x){return x*2;}fn fact(n){if(n<=1){return 1;}else{return n*fact(n-1);}}fn main(){print(twice(21));print(fact(5));}"), vec!["42", "120"]);
}

#[test]
fn function_calls_inside_worker() {
    assert_eq!(run("fn twice(x){return x*2;}fn main(){let base=21;let task=worker.submit{twice(base)};print(worker.wait(task));}"), vec!["42"]);
}

#[test]
fn unknown_function_and_bad_arity_are_compile_errors() {
    assert!(compiler::compile("fn main(){print(missing(1));}")
        .unwrap_err()
        .contains("unknown function"));
    assert!(
        compiler::compile("fn twice(x){return x*2;}fn main(){print(twice());}")
            .unwrap_err()
            .contains("expects 1")
    );
}

#[test]
fn loop_control_outside_loop_is_rejected() {
    assert!(compiler::compile("fn main(){break;}")
        .unwrap_err()
        .contains("outside a loop"));
    assert!(compiler::compile("fn main(){continue;}")
        .unwrap_err()
        .contains("outside a loop"));
}

#[test]
fn non_main_functions_must_return_on_every_branch() {
    assert!(compiler::compile(
        "fn value(flag){if(flag){return 1;}else{return 2;}}fn main(){print(value(1));}",
    ).is_ok());

    let missing_else = compiler::compile(
        "fn value(flag){if(flag){return 1;}}fn main(){print(value(1));}",
    ).unwrap_err();
    assert!(missing_else.contains("may finish without returning"), "{missing_else}");

    let loop_only = compiler::compile(
        "fn value(flag){while(flag){return 1;}}fn main(){print(value(1));}",
    ).unwrap_err();
    assert!(loop_only.contains("may finish without returning"), "{loop_only}");
}

#[test]
fn unreachable_statements_after_return_are_rejected() {
    let error = compiler::compile("fn value(){return 1;print(2);}fn main(){}")
        .unwrap_err();
    assert!(error.contains("unreachable statement"), "{error}");
}

#[test]
fn main_may_fall_through_without_explicit_return() {
    assert!(compiler::compile("fn main(){print(42);}").is_ok());
}
