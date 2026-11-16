use nexa_runtime::compiler;

#[test]
fn task_handles_are_inferred_and_wait_returns_integer() {
    let source =
        "fn main(){let task=worker.submit{21*2};let result=worker.wait(task);print(result);}";
    assert!(compiler::compile(source).is_ok());
}

#[test]
fn task_handle_cannot_be_printed_or_used_in_arithmetic() {
    let printed =
        compiler::compile("fn main(){let task=worker.submit{1};print(task);}").unwrap_err();
    assert!(printed.contains("print expects an integer"), "{printed}");

    let arithmetic =
        compiler::compile("fn main(){let task=worker.submit{1};let x=task+1;}").unwrap_err();
    assert!(
        arithmetic.contains("task handles cannot be used"),
        "{arithmetic}"
    );
}

#[test]
fn wait_requires_a_task_handle() {
    let error = compiler::compile("fn main(){let value=worker.wait(7);}").unwrap_err();
    assert!(
        error.contains("worker.wait expects a task handle"),
        "{error}"
    );
}

#[test]
fn local_assignment_cannot_change_inferred_type() {
    let error = compiler::compile("fn main(){let task=worker.submit{1};task=2;}").unwrap_err();
    assert!(error.contains("cannot assign"), "{error}");
}

#[test]
fn worker_blocks_must_produce_integers() {
    let error =
        compiler::compile("fn main(){let first=worker.submit{1};let second=worker.submit{first};}")
            .unwrap_err();
    assert!(
        error.contains("worker task must produce an integer"),
        "{error}"
    );
}

#[test]
fn task_handles_are_rejected_in_conditions_and_function_arguments() {
    let condition =
        compiler::compile("fn main(){let task=worker.submit{1};if(task){print(1);}}").unwrap_err();
    assert!(
        condition.contains("if condition must be an integer"),
        "{condition}"
    );

    let argument = compiler::compile(
        "fn consume(value){return value;}fn main(){let task=worker.submit{1};consume(task);}",
    )
    .unwrap_err();
    assert!(
        argument.contains("expects Int"),
        "{argument}"
    );
}


#[test]
fn typed_function_signatures_accept_int_and_task_parameters_and_returns() {
    let source = r#"
        fn wait_task(task: task) -> int {
            return worker.wait(task);
        }
        fn make_task() -> task {
            return worker.submit { 40 + 2 };
        }
        fn main() {
            let result = wait_task(make_task());
            print(result);
        }
    "#;
    assert!(compiler::compile(source).is_ok());
}

#[test]
fn typed_function_signatures_support_forward_calls_and_task_identity() {
    let source = r#"
        fn main() {
            let task = identity(make_task());
            print(worker.wait(task));
        }
        fn identity(value: task) -> task {
            return value;
        }
        fn make_task() -> task {
            return worker.submit { 42 };
        }
    "#;
    assert!(compiler::compile(source).is_ok());
}

#[test]
fn typed_function_calls_reject_wrong_argument_types() {
    let int_to_task = compiler::compile(
        "fn needs_task(value: task) -> int { return worker.wait(value); } fn main() { print(needs_task(1)); }"
    ).unwrap_err();
    assert!(int_to_task.contains("expects Task"), "{int_to_task}");

    let task_to_int = compiler::compile(
        "fn needs_int(value: int) -> int { return value; } fn main() { let t=worker.submit{1}; print(needs_int(t)); }"
    ).unwrap_err();
    assert!(task_to_int.contains("expects Int"), "{task_to_int}");
}

#[test]
fn typed_function_return_contract_rejects_wrong_types() {
    let wrong_return = compiler::compile(
        "fn make_task() -> task { return 7; } fn main() {}"
    ).unwrap_err();
    assert!(wrong_return.contains("return type mismatch"), "{wrong_return}");

    let missing_return = compiler::compile(
        "fn make_task() -> task { let value=1; } fn main() {}"
    ).unwrap_err();
    assert!(missing_return.contains("may finish without returning"), "{missing_return}");
}

#[test]
fn legacy_unannotated_functions_remain_integer_typed() {
    let source = "fn identity(value) { return value; } fn main() { print(identity(42)); }";
    assert!(compiler::compile(source).is_ok());
}


#[test]
fn worker_submit_cannot_capture_task_handle() {
    let error = compiler::compile(
        "fn main() { let task=worker.submit{1}; let other=worker.submit{worker.wait(task)}; }"
    ).unwrap_err();
    assert!(error.contains("cannot capture task handle"), "{error}");
}
