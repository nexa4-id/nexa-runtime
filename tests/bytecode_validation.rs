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

#[test]
fn verifier_rejects_gapped_local_slot_layout() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![Instruction::Const(1), Instruction::Store(2), Instruction::Return],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("invalid local slot layout"), "{error}");
}

#[test]
fn verifier_rejects_local_initialized_on_only_one_branch() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Const(1),
            Instruction::JumpIfFalse(5),
            Instruction::Const(10),
            Instruction::Store(0),
            Instruction::Jump(6),
            Instruction::Jump(6),
            Instruction::Load(0),
            Instruction::Print,
            Instruction::Const(0),
            Instruction::Halt,
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("uninitialized"), "{error}");
}

#[test]
fn verifier_accepts_local_initialized_on_both_branches() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Const(1),
            Instruction::JumpIfFalse(5),
            Instruction::Const(10),
            Instruction::Store(0),
            Instruction::Jump(7),
            Instruction::Const(20),
            Instruction::Store(0),
            Instruction::Load(0),
            Instruction::Print,
            Instruction::Const(0),
            Instruction::Halt,
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    assert_eq!(vm.run(&code).unwrap(), &["10".to_string()]);
}

#[test]
fn verifier_rejects_local_initialized_only_inside_loop() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Const(1),
            Instruction::JumpIfFalse(5),
            Instruction::Const(9),
            Instruction::Store(0),
            Instruction::Jump(0),
            Instruction::Load(0),
            Instruction::Print,
            Instruction::Const(0),
            Instruction::Halt,
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("uninitialized"), "{error}");
}

#[test]
fn verifier_rejects_worker_capture_before_local_initialization() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Submit(vec![Instruction::Load(0), Instruction::Halt]),
            Instruction::Store(0),
            Instruction::Const(0),
            Instruction::Halt,
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("uninitialized"), "{error}");
}

#[test]
fn verifier_accepts_initialized_parameters() {
    let code = vec![
        Instruction::DefineFunction(
            "main".into(),
            vec![],
            vec![Instruction::Const(7), Instruction::Call("identity".into(), 1), Instruction::Print, Instruction::Const(0), Instruction::Halt],
        ),
        Instruction::DefineFunction(
            "identity".into(),
            vec!["value".into()],
            vec![Instruction::Load(0), Instruction::Return],
        ),
    ];
    let mut vm = Vm::new(1).unwrap();
    assert_eq!(vm.run(&code).unwrap(), &["7".to_string()]);
}


#[test]
fn verifier_rejects_task_handle_used_by_integer_operator() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Submit(vec![Instruction::Const(1), Instruction::Halt]),
            Instruction::Const(2),
            Instruction::Add,
            Instruction::Print,
            Instruction::Const(0),
            Instruction::Halt,
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("requires Int operands"), "{error}");
}

#[test]
fn verifier_rejects_wait_on_integer_value() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Const(0),
            Instruction::Wait,
            Instruction::Print,
            Instruction::Const(0),
            Instruction::Halt,
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("requires Task"), "{error}");
}

#[test]
fn verifier_rejects_task_handle_print() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Submit(vec![Instruction::Const(1), Instruction::Halt]),
            Instruction::Print,
            Instruction::Const(0),
            Instruction::Halt,
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("Print") && error.contains("Task"), "{error}");
}

#[test]
fn verifier_rejects_different_stack_types_at_branch_merge() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Const(1),
            Instruction::JumpIfFalse(5),
            Instruction::Submit(vec![Instruction::Const(1), Instruction::Halt]),
            Instruction::Jump(6),
            Instruction::Const(0),
            Instruction::Const(9),
            Instruction::Print,
            Instruction::Const(0),
            Instruction::Halt,
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("inconsistent stack types"), "{error}");
}

#[test]
fn verifier_rejects_different_local_types_at_branch_merge() {
    let code = vec![Instruction::DefineFunction(
        "main".into(),
        vec![],
        vec![
            Instruction::Const(1),
            Instruction::JumpIfFalse(6),
            Instruction::Submit(vec![Instruction::Const(1), Instruction::Halt]),
            Instruction::Store(0),
            Instruction::Jump(8),
            Instruction::Const(0),
            Instruction::Const(7),
            Instruction::Store(0),
            Instruction::Load(0),
            Instruction::Print,
            Instruction::Const(0),
            Instruction::Halt,
        ],
    )];
    let mut vm = Vm::new(1).unwrap();
    let error = vm.run(&code).unwrap_err();
    assert!(error.contains("inconsistent local types"), "{error}");
}
