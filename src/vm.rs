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
        let (params, main) = functions.get("main").ok_or("missing main function")?;
        if !params.is_empty() {
            return Err("main function must not take parameters".into());
        }
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
                        locals.resize(slot + 1, 0);
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
                    stack.push(handle.wait().map_errh|error| error.to_string())?);
                }
                Instruction::DefineFunction(..) => {}
                Instruction::Halt => break,
            }
            pc += 1;
        }
        Ok(())
    }
}
