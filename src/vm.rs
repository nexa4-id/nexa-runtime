use crate::compiler::Instruction;
use crate::worker::{TaskHandle, WorkerError, WorkerPool};
use std::collections::{HashMap, HashSet};

type Functions = HashMap<String, (Vec<String>, Vec<Instruction>)>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StackType {
    Int,
    Task,
}

#[derive(Debug, Clone)]
struct FunctionSignature {
    parameter_types: Vec<StackType>,
    return_type: StackType,
}

type Signatures = HashMap<String, FunctionSignature>;

pub struct Vm {
    pool: WorkerPool,
    output: Vec<String>,
}
impl Vm {
    pub fn new(workers: usize) -> Result<Self, WorkerError> {
        Ok(Self {
            pool: WorkerPool::new(workers)?,
            output: Vec::new(),
        })
    }
    pub fn run(&mut self, code: &[Instruction]) -> Result<&[String], String> {
        self.output.clear();
        let mut functions = Functions::new();
        for instruction in code {
            if let Instruction::DefineFunction(name, params, body) = instruction {
                if functions
                    .insert(name.clone(), (params.clone(), body.clone()))
                    .is_some()
                {
                    return Err(format!("duplicate function '{name}'"));
                }
            }
        }
        let (params, _) = functions.get("main").ok_or("missing main function")?;
        if !params.is_empty() {
            return Err("main function must not take parameters".into());
        }

        let mut signatures = Signatures::new();
        for (name, (params, _)) in &functions {
            let mut unique_params = HashSet::new();
            for param in params {
                if !unique_params.insert(param) {
                    return Err(format!(
                        "duplicate parameter '{param}' in function '{name}'"
                    ));
                }
            }
            signatures.insert(
                name.clone(),
                FunctionSignature {
                    parameter_types: vec![StackType::Int; params.len()],
                    return_type: StackType::Int,
                },
            );
        }

        for (name, (_, body)) in &functions {
            let signature = signatures
                .get(name)
                .expect("signature is created for every function");
            Self::validate_bytecode(body, signature, &functions, &signatures, name)?;
        }
        let (_, main) = functions.get("main").expect("main was checked above");
        let (mut tasks, mut stack) = (Vec::new(), Vec::new());
        Self::execute(
            main,
            &mut stack,
            &mut Vec::new(),
            &mut tasks,
            &self.pool,
            &functions,
            &mut self.output,
        )?;
        Ok(&self.output)
    }

    fn validate_bytecode(
        code: &[Instruction],
        signature: &FunctionSignature,
        functions: &Functions,
        signatures: &Signatures,
        owner: &str,
    ) -> Result<(), String> {
        let parameter_count = signature.parameter_types.len();
        let mut frame_slots = parameter_count;
        for instruction in code {
            if let Instruction::Store(slot) = instruction {
                let required_slots = slot
                    .checked_add(1)
                    .ok_or_else(|| format!("invalid local slot {slot} in '{owner}'"))?;
                frame_slots = frame_slots.max(required_slots);
            }
        }

        let mut declared_slots = vec![false; frame_slots];
        for slot in declared_slots.iter_mut().take(parameter_count) {
            *slot = true;
        }
        for instruction in code {
            if let Instruction::Store(slot) = instruction {
                declared_slots[*slot] = true;
            }
        }
        if let Some(slot) = declared_slots.iter().position(|declared| !declared) {
            return Err(format!(
                "invalid local slot layout in '{owner}': slot {slot} is undeclared"
            ));
        }

        let mut initial_locals = vec![None; frame_slots];
        for local in initial_locals.iter_mut().take(parameter_count) {
            *local = Some(StackType::Int);
        }
        Self::validate_typed_bytecode(
            code,
            initial_locals,
            functions,
            signatures,
            owner,
            signature.return_type,
            false,
        )
    }

    fn validate_typed_bytecode(
        code: &[Instruction],
        initial_locals: Vec<Option<StackType>>,
        functions: &Functions,
        signatures: &Signatures,
        owner: &str,
        return_type: StackType,
        worker_body: bool,
    ) -> Result<(), String> {
        if code.is_empty() {
            return Err(format!("empty bytecode body in '{owner}'"));
        }

        for (index, instruction) in code.iter().enumerate() {
            match instruction {
                Instruction::Jump(target) | Instruction::JumpIfFalse(target)
                    if *target > code.len() =>
                {
                    return Err(format!(
                        "invalid bytecode jump at instruction {index}: target {target} exceeds code length {}",
                        code.len()
                    ));
                }
                Instruction::Call(name, argc) => {
                    let signature = signatures.get(name).ok_or_else(|| {
                        format!("'{owner}' calls unknown function '{name}'")
                    })?;
                    if signature.parameter_types.len() != *argc {
                        return Err(format!(
                            "function '{name}' expects {} argument(s), got {argc}",
                            signature.parameter_types.len()
                        ));
                    }
                }
                _ => {}
            }
        }

        // Each program point carries an exact operand-stack type sequence and
        // the types definitely initialized in each local slot.
        let mut states: Vec<Option<(Vec<StackType>, Vec<Option<StackType>>)>> =
            vec![None; code.len() + 1];
        let mut pending = vec![(0usize, Vec::<StackType>::new(), initial_locals)];

        while let Some((pc, mut stack, mut locals)) = pending.pop() {
            if pc > code.len() {
                return Err(format!("invalid bytecode control-flow target in '{owner}'"));
            }

            if let Some((previous_stack, previous_locals)) = &states[pc] {
                if previous_stack != &stack {
                    return Err(format!(
                        "inconsistent stack types in '{owner}' at instruction {pc}: {previous_stack:?} versus {stack:?}"
                    ));
                }
                let mut merged = previous_locals.clone();
                for (slot, (previous, incoming)) in
                    merged.iter_mut().zip(locals.iter()).enumerate()
                {
                    match (*previous, *incoming) {
                        (Some(left), Some(right)) if left != right => {
                            return Err(format!(
                                "inconsistent local types in '{owner}' at instruction {pc}: slot {slot} is {left:?} versus {right:?}"
                            ));
                        }
                        (Some(_), None) => *previous = None,
                        _ => {}
                    }
                }
                if &merged == previous_locals {
                    continue;
                }
                states[pc] = Some((stack.clone(), merged.clone()));
                locals = merged;
            } else {
                states[pc] = Some((stack.clone(), locals.clone()));
            }

            if pc == code.len() {
                return Err(format!(
                    "control flow in '{owner}' falls through without {}",
                    if worker_body { "Halt" } else { "Return" }
                ));
            }

            let instruction = &code[pc];
            match instruction {
                Instruction::Const(_) => stack.push(StackType::Int),
                Instruction::Load(slot) => {
                    let ty = locals.get(*slot).copied().flatten().ok_or_else(|| {
                        format!(
                            "load from uninitialized or invalid local slot {slot} in '{owner}' at instruction {pc}"
                        )
                    })?;
                    stack.push(ty);
                }
                Instruction::Store(slot) => {
                    let ty = stack.pop().ok_or_else(|| {
                        format!(
                            "stack underflow in '{owner}' at instruction {pc}: Store needs 1 value"
                        )
                    })?;
                    let local = locals.get_mut(*slot).ok_or_else(|| {
                        format!("invalid local slot {slot} in '{owner}'")
                    })?;
                    *local = Some(ty);
                }
                Instruction::Add | Instruction::Sub | Instruction::Mul | Instruction::Div
                | Instruction::Eq | Instruction::Ne | Instruction::Lt | Instruction::Le
                | Instruction::Gt | Instruction::Ge => {
                    let right = stack.pop().ok_or_else(|| {
                        format!("stack underflow in '{owner}' at instruction {pc}")
                    })?;
                    let left = stack.pop().ok_or_else(|| {
                        format!("stack underflow in '{owner}' at instruction {pc}")
                    })?;
                    if left != StackType::Int || right != StackType::Int {
                        return Err(format!(
                            "integer operation in '{owner}' at instruction {pc} requires Int operands, got {left:?} and {right:?}"
                        ));
                    }
                    stack.push(StackType::Int);
                }
                Instruction::Print => {
                    let value = stack.pop().ok_or_else(|| {
                        format!("stack underflow in '{owner}' at instruction {pc}")
                    })?;
                    if value != StackType::Int {
                        return Err(format!(
                            "Print in '{owner}' at instruction {pc} requires Int, got {value:?}"
                        ));
                    }
                }
                Instruction::Pop => {
                    stack.pop().ok_or_else(|| {
                        format!("stack underflow in '{owner}' at instruction {pc}")
                    })?;
                }
                Instruction::Submit(body) => {
                    Self::validate_typed_bytecode(
                        body,
                        locals.clone(),
                        functions,
                        signatures,
                        owner,
                        StackType::Int,
                        true,
                    )?;
                    stack.push(StackType::Task);
                }
                Instruction::Wait => {
                    let handle = stack.pop().ok_or_else(|| {
                        format!("stack underflow in '{owner}' at instruction {pc}")
                    })?;
                    if handle != StackType::Task {
                        return Err(format!(
                            "Wait in '{owner}' at instruction {pc} requires Task, got {handle:?}"
                        ));
                    }
                    stack.push(StackType::Int);
                }
                Instruction::Jump(_) => {}
                Instruction::JumpIfFalse(_) => {
                    let condition = stack.pop().ok_or_else(|| {
                        format!("stack underflow in '{owner}' at instruction {pc}")
                    })?;
                    if condition != StackType::Int {
                        return Err(format!(
                            "JumpIfFalse in '{owner}' at instruction {pc} requires Int, got {condition:?}"
                        ));
                    }
                }
                Instruction::Call(name, argc) => {
                    let signature = signatures
                        .get(name)
                        .expect("call target was checked before dataflow verification");
                    for expected in signature.parameter_types.iter().rev().take(*argc) {
                        let argument = stack.pop().ok_or_else(|| {
                            format!("stack underflow in '{owner}' at function call '{name}'")
                        })?;
                        if argument != *expected {
                            return Err(format!(
                                "function '{name}' argument type mismatch: expected {expected:?}, got {argument:?}"
                            ));
                        }
                    }
                    stack.push(signature.return_type);
                }
                Instruction::Return | Instruction::Halt => {
                    let is_halt = matches!(instruction, Instruction::Halt);
                    if is_halt != worker_body {
                        return Err(format!(
                            "{} is invalid in '{}': expected {}",
                            if is_halt { "Halt" } else { "Return" },
                            owner,
                            if worker_body { "Halt" } else { "Return" }
                        ));
                    }
                    let expected = return_type;
                    if stack.as_slice() != [expected] {
                        return Err(format!(
                            "{} in '{owner}' requires exactly one {expected:?} value, found {stack:?}",
                            if is_halt { "Halt" } else { "Return" }
                        ));
                    }
                    continue;
                }
                Instruction::DefineFunction(..) => {
                    return Err(format!("nested function definition in '{owner}'"));
                }
            }

            match instruction {
                Instruction::Jump(target) => pending.push((*target, stack, locals)),
                Instruction::JumpIfFalse(target) => {
                    pending.push((*target, stack.clone(), locals.clone()));
                    pending.push((pc + 1, stack, locals));
                }
                _ => pending.push((pc + 1, stack, locals)),
            }
        }
        Ok(())
    }

    fn execute(
        code: &[Instruction],
        stack: &mut Vec<i64>,
        locals: &mut Vec<i64>,
        tasks: &mut Vec<Option<TaskHandle<i64>>>,
        pool: &WorkerPool,
        functions: &Functions,
        output: &mut Vec<String>,
    ) -> Result<(), String> {
        let mut pc = 0;
        while pc < code.len() {
            match &code[pc] {
                Instruction::Const(value) => stack.push(*value),
                Instruction::Load(slot) => {
                    stack.push(*locals.get(*slot).ok_or("invalid local slot")?)
                }
                Instruction::Store(slot) => {
                    let value = stack.pop().ok_or("stack underflow")?;
                    if *slot >= locals.len() {
                        locals.resize(*slot + 1, 0);
                    }
                    locals[*slot] = value;
                }
                Instruction::Add
                | Instruction::Sub
                | Instruction::Mul
                | Instruction::Div
                | Instruction::Eq
                | Instruction::Ne
                | Instruction::Lt
                | Instruction::Le
                | Instruction::Gt
                | Instruction::Ge => {
                    let right = stack.pop().ok_or("stack underflow")?;
                    let left = stack.pop().ok_or("stack underflow")?;
                    let value = match &code[pc] {
                        Instruction::Add => left.checked_add(right),
                        Instruction::Sub => left.checked_sub(right),
                        Instruction::Mul => left.checked_mul(right),
                        Instruction::Div => {
                            if right == 0 {
                                None
                            } else {
                                left.checked_div(right)
                            }
                        }
                        Instruction::Eq => Some((left == right) as i64),
                        Instruction::Ne => Some((left != right) as i64),
                        Instruction::Lt => Some((left < right) as i64),
                        Instruction::Le => Some((left <= right) as i64),
                        Instruction::Gt => Some((left > right) as i64),
                        Instruction::Ge => Some((left >= right) as i64),
                        _ => None,
                    }
                    .ok_or("arithmetic overflow or division by zero")?;
                    stack.push(value);
                }
                Instruction::Print => {
                    output.push(stack.pop().ok_or("stack underflow")?.to_string())
                }
                Instruction::Pop => {
                    stack.pop().ok_or("stack underflow")?;
                }
                Instruction::Jump(target) => {
                    pc = *target;
                    continue;
                }
                Instruction::JumpIfFalse(target) => {
                    if stack.pop().ok_or("stack underflow")? == 0 {
                        pc = *target;
                        continue;
                    }
                }
                Instruction::Call(name, argc) => {
                    let (params, body) = functions
                        .get(name)
                        .ok_or_else(|| format!("unknown function '{name}'"))?;
                    if params.len() != *argc {
                        return Err(format!(
                            "function '{name}' expects {} argument(s), got {argc}",
                            params.len()
                        ));
                    }
                    let start = stack
                        .len()
                        .checked_sub(*argc)
                        .ok_or("stack underflow in function call")?;
                    let mut frame = stack.split_off(start);
                    let mut result = Vec::new();
                    Self::execute(
                        body,
                        &mut result,
                        &mut frame,
                        tasks,
                        pool,
                        functions,
                        output,
                    )?;
                    stack.push(result.pop().unwrap_or(0));
                }
                Instruction::Return => return Ok(()),
                Instruction::Submit(body) => {
                    let captures = locals.clone();
                    let task_code = body.clone();
                    let function_table = functions.clone();
                    let handle = pool
                        .submit(move || {
                            let nested_pool = WorkerPool::new(1)?;
                            let (mut task_stack, mut task_handles, mut ignored_output) =
                                (Vec::new(), Vec::new(), Vec::new());
                            let mut task_locals = captures;
                            Self::execute(
                                &task_code,
                                &mut task_stack,
                                &mut task_locals,
                                &mut task_handles,
                                &nested_pool,
                                &function_table,
                                &mut ignored_output,
                            )
                            .map_err(WorkerError::TaskFailed)?;
                            task_stack.pop().ok_or_else(|| {
                                WorkerError::TaskFailed("worker block produced no value".into())
                            })
                        })
                        .map_err(|error| error.to_string())?;
                    tasks.push(Some(handle));
                    stack.push((tasks.len() - 1) as i64);
                }
                Instruction::Wait => {
                    let raw = stack.pop().ok_or("stack underflow")?;
                    let index = usize::try_from(raw).map_err(|_| "invalid task handle")?;
                    let handle = tasks
                        .get_mut(index)
                        .and_then(Option::take)
                        .ok_or("invalid or reused task handle")?;
                    stack.push(handle.wait().map_err(|error| error.to_string())?);
                }
                Instruction::DefineFunction(..) => {}
                Instruction::Halt => break,
            }
            pc += 1;
        }
        Ok(())
    }
}
