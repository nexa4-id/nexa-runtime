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
        argument.contains("function arguments must be integers"),
        "{argument}"
    );
}
