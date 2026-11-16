use nexa_runtime::{compiler::Instruction, Vm};

#[test]
fn vm_rejects_out_of_range_jump_before_execution() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![Instruction::Jump(99)],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("invalid bytecode jump"), "{error}");
}

#[test]
fn vm_validates_nested_worker_bytecode() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Submit(vec![Instruction::JumpIfFalse(10)]),
            Instruction::Halt,
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("invalid bytecode jump"), "{error}");
}

#[test]
fn vm_accepts_jump_to_exact_end_of_code() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![Instruction::Jump(1)],
    )];
    let mut vm = Vm::new(1).unwrap();
    assert!(vm.run(&code).is_ok());
}
