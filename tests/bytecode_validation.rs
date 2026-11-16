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

#[test]
fn verifier_rejects_stack_underflow_before_execution() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![Instruction::Add],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("stack underflow"), "{error}");
}

#[test]
fn verifier_rejects_inconsistent_stack_depth_at_branch_merge() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Const(1),
            Instruction::JumpIfFalse(4),
            Instruction::Const(7),
            Instruction::Jump(4),
            Instruction::Halt,
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("inconsistent stack depth"), "{error}");
}

#[test]
fn verifier_checks_call_arity_before_execution() {
    let code = vec![
        Instruction::DefineFunction(
            "main".into(),
            vec![],
            vec![Instruction::Const(1), Instruction::Call("one".into(), 0)],
        ),
        Instruction::DefineFunction(
            "one".into(),
            vec!["value".into()],
            vec![Instruction::Load(0), Instruction::Return],
        ),
    ];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("expects 1 argument"), "{error}");
}

#[test]
fn verifier_rejects_load_from_invalid_local_slot() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![Instruction::Load(3), Instruction::Print],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("invalid local slot"), "{error}");
}

#[test]
fn verifier_rejects_return_with_wrong_stack_depth() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![Instruction::Const(1), Instruction::Const(2), Instruction::Return],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("requires exactly one stack value"), "{error}");
}

#[test]
fn verifier_checks_invalid_jump_even_in_unreachable_code() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Const(1),
            Instruction::Return,
            Instruction::Jump(99),
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("invalid bytecode jump"), "{error}");
}

#[test]
fn verifier_checks_call_arity_even_in_unreachable_code() {
    let code = vec![
        Instruction::DefineFunction(
            "main".into(),
            vec![],
            vec![
                Instruction::Const(1),
                Instruction::Return,
                Instruction::Call("one".into(), 0),
            ],
        ),
        Instruction::DefineFunction(
            "one".into(),
            vec!["value".into()],
            vec![Instruction::Load(0), Instruction::Return],
        ),
    ];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("expects 1 argument"), "{error}");
}
