use crate::compiler::Instruction;
use crate::worker::{TaskHandle, WorkerError, WorkerPool};
use std::collections::HashMap;

type Functions = HashMap<String, (Vec<String>, Vec<Instruction>)>;

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
        for (name, (params, body)) in &functions {
            Self::validate_bytecode(body, params.len(), &functions, name)?;
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
        parameter_count: usize,
        functions: &Functions,
        owner: &str,
    ) -> Result<(), String> {
        let mut frame_slots = parameter_count;
        for instruction in code {
            if let Instruction::Store(slot) = instruction {
                let required_slots = slot.checked_add(1).ok_or_else(|| {
                    format!("invalid local slot {slot} in '{owner}'")
                })?;
                frame_slots = frame_slots.max(required_slots);
            }
        }
        let mut initial_slots = vec![false; frame_slots];
        for slot in initial_slots.iter_mut().take(parameter_count) {
            *slot = true;
        }
        Self::validate_bytecode_with_state(
            code,
            frame_slots,
            initial_slots,
            functions,
            owner,
        )
    }

    fn validate_bytecode_with_state(
        code: &[Instruction],
        frame_slots: usize,
        initial_slots: Vec<bool>,
        functions: &Functions,
        owner: &str,
    ) -> Result<(), String> {
        if initial_slots.len() != frame_slots {
            return Err(format!("invalid local frame layout in '{owner}'"));
        }

        let mut declared_slots = initial_slots.clone();
        for (index, instruction) in code.iter().enumerate() {
            match instruction {
                Instruction::Store(slot) => {
                    if *slot >= frame_slots {
                        return Err(format!("invalid local slot {slot} in '{owner}'"));
                    }
                    declared_slots[*slot] = true;
                }
                Instruction::Jump(target) | Instruction::JumpIfFalse(target)
                    if *target > code.len() =>
                {
                    return Err(format!(
                        "invalid bytecode jump at instruction {index}: target {target} exceeds code length {}",
                        code.len()
                    ));
                }
                Instruction::Call(name, argc) => {
                    let (params, _) = functions.get(name).ok_or_else(|| {
                        format!("'{owner}' calls unknown function '{name}'")
                    })?;
                    if params.len() != *argc {
                        return Err(format!(
                            "function '{name}' expects {} argument(s), got {argc}",
                            params.len()
                        ));
                    }
                }
                _ => {}
            }
        }
        if let Some(slot) = declared_slots.iter().position(|declared| !declared) {
            return Err(format!(
                "invalid local slot layout in '{owner}': slot {slot} is undeclared"
            ));
        }

        let mut depths: Vec<Option<usize>> = vec![None; code.len() + 1];
        let mut initialized: Vec<Option<Vec<bool>>> = vec![None; code.len() + 1];
        let mut pending = vec![(0usize, 0usize, initial_slots)];
        while let Some((pc, depth, slots)) = pending.pop() {
            if pc > code.len() {
                return Err(format!("invalid bytecode control-flow target in '{owner}'"));
            }
            if let Some(previous_depth) = depths[pc] {
                if previous_depth != depth {
                    return Err(format!(
                        "inconsistent stack depth in '{owner}' at instruction {pc}: {previous_depth} versus {depth}"
                    ));
                }
                let previous_slots = initialized[pc]
                    .as_ref()
                    .expect("stack depth and local state are recorded together");
                let merged: Vec<bool> = previous_slots
                    .iter()
                    .zip(slots.iter())
                    .map(|(left, right)| *left && *right)
                    .collect();
                if &merged != previous_slots {
                    initialized[pc] = Some(merged.clone());
                    pending.push((pc, depth, merged));
                }
                continue;
            }
            depths[pc] = Some(depth);
            initialized[pc] = Some(slots.clone());
            if pc == code.len() {
                if depth != 0 {
                    return Err(format!(
                        "stack is not empty at end of '{owner}' (depth {depth})"
                    ));
                }
                continue;
            }

            let instruction = &code[pc];
            let (required, produced) = match instruction {
                Instruction::Const(_) => (0, 1),
                Instruction::Load(slot) => {
                    if *slot >= frame_slots || !slots[*slot] {
                        return Err(format!(
                            "load from uninitialized or invalid local slot {slot} in '{owner}' at instruction {pc}"
                        ));
                    }
                    (0, 1)
                }
                Instruction::Store(slot) => {
                    if *slot >= frame_slots {
                        return Err(format!("invalid local slot {slot} in '{owner}'"));
                    }
                    (1, 0)
                }
                Instruction::Add | Instruction::Sub | Instruction::Mul | Instruction::Div
                | Instruction::Eq | Instruction::Ne | Instruction::Lt | Instruction::Le
                | Instruction::Gt | Instruction::Ge => (2, 1),
                Instruction::Print | Instruction::Pop | Instruction::JumpIfFalse(_) => (1, 0),
                Instruction::Submit(body) => {
                    Self::validate_bytecode_with_state(
                        body,
                        frame_slots,
                        slots.clone(),
                        functions,
                        owner,
                    )?;
                    (0, 1)
                }
                Instruction::Wait => (1, 1),
                Instruction::Jump(_) => (0, 0),
                Instruction::Call(name, argc) => {
                    let (params, _) = functions.get(name).ok_or_else(|| {
                        format!("'{owner}' calls unknown function '{name}'")
                    })?;
                    if params.len() != *argc {
                        return Err(format!(
                            "function '{name}' expects {} argument(s), got {argc}",
                            params.len()
                        ));
                    }
                    (*argc, 1)
                }
                Instruction::Return => {
                    if depth != 1 {
                        return Err(format!(
                            "return in '{owner}' requires exactly one stack value, found {depth}"
                        ));
                    }
                    continue;
                }
                Instruction::Halt => {
                    if depth != 1 {
                        return Err(format!(
                            "halt in '{owner}' requires exactly one stack value, found {depth}"
                        ));
                    }
                    continue;
                }
                Instruction::DefineFunction(..) => {
                    return Err(format!("nested function definition in '{owner}'"));
                }
            };
            if depth < required {
                return Err(format!(
                    "stack underflow in '{owner}' at instruction {pc}: needs {required}, has {depth}"
                ));
            }
            let next_depth = depth - required + produced;
            let mut next_slots = slots;
            if let Instruction::Store(slot) = instruction {
                next_slots[*slot] = true;
            }
            match instruction {
                Instruction::Jump(target) => pending.push((*target, next_depth, next_slots)),
                Instruction::JumpIfFalse(target) => {
                    pending.push((*target, next_depth, next_slots.clone()));
                    pending.push((pc + 1, next_depth, next_slots));
                }
                _ => pending.push((pc + 1, next_depth, next_slots)),
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
