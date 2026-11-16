use nexa_runtime::{compiler, Vm};
#[test]
fn worker_capture_wait_prints_42() {
    let s="fn main(){let base=21;let task=worker.submit{base*2};let result=worker.wait(task);print(result);}";
    let c = compiler::compile(s).unwrap();
    let mut vm = Vm::new(2).unwrap();
    assert_eq!(vm.run(&c).unwrap(), &["42".to_string()]);
}
#[test]
fn unknown_name_is_compile_error() {
    assert!(compiler::compile("fn main(){let x=missing;}").is_err());
}


#[test]
fn typed_task_functions_execute_across_calls() {
    let source = r#"
        fn wait_task(task: task) -> int {
            return worker.wait(task);
        }
        fn make_task() -> task {
            return worker.submit { 21 * 2 };
        }
        fn main() {
            let task = make_task();
            print(wait_task(task));
        }
    "#;
    let code = compiler::compile(source).unwrap();
    let mut vm = Vm::new(2).unwrap();
    assert_eq!(vm.run(&code).unwrap(), &["42".to_string()]);
}

#[test]
fn typed_integer_function_calls_execute() {
    let source = r#"
        fn double(value: int) -> int {
            return value * 2;
        }
        fn main() {
            print(double(21));
        }
    "#;
    let code = compiler::compile(source).unwrap();
    let mut vm = Vm::new(1).unwrap();
    assert_eq!(vm.run(&code).unwrap(), &["42".to_string()]);
}
