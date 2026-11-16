use crate::lexer::{lex, Token, TokenKind};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StaticType {
    Int,
    Task,
}

#[derive(Debug, Clone)]
pub enum Instruction {
    Const(i64),
    Load(usize),
    Store(usize),
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Print,
    Pop,
    Submit(Vec<Instruction>),
    Wait,
    Jump(usize),
    JumpIfFalse(usize),
    Call(String, usize),
    Return,
    DefineFunction(String, Vec<String>, Vec<Instruction>),
    Halt,
}

pub fn compile(source: &str) -> Result<Vec<Instruction>, String> {
    let tokens = lex(source).map_err(|message| locate_error(source, &message))?;
    let mut parser = Parser {
        tokens,
        pos: 0,
        locals: Vec::new(),
        local_types: Vec::new(),
        scopes: Vec::new(),
        code: Vec::new(),
        loops: Vec::new(),
    };
    parser
        .program()
        .map_err(|message| parser.diagnostic(source, &message))
}

fn locate_error(source: &str, message: &str) -> String {
    let offset = message
        .split_whitespace()
        .last()
        .and_then(|part| part.parse::<usize>().ok());
    match offset {
        Some(offset) => {
            let (line, column) = line_column(source, offset);
            format!("{message} (line {line}, column {column})")
        }
        None => message.to_string(),
    }
}

fn line_column(source: &str, offset: usize) -> (usize, usize) {
    let mut line = 1;
    let mut column = 1;
    for ch in source.chars().take(offset) {
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
    locals: Vec<String>,
    local_types: Vec<StaticType>,
    scopes: Vec<Vec<String>>,
    code: Vec<Instruction>,
    loops: Vec<Vec<usize>>,
}

impl Parser {
    fn diagnostic(&self, source: &str, message: &str) -> String {
        let offset = self
            .tokens
            .get(self.pos.saturating_sub(1))
            .or_else(|| self.tokens.get(self.pos))
            .map(|token| token.offset)
            .unwrap_or_else(|| source.chars().count());
        let (line, column) = line_column(source, offset);
        format!("{message} (line {line}, column {column})")
    }

    fn kind(&self) -> &TokenKind {
        &self.tokens[self.pos].kind
    }
    fn take(&mut self) -> TokenKind {
        let kind = self.tokens[self.pos].kind.clone();
        self.pos += 1;
        kind
    }
    fn eat(&mut self, expected: TokenKind) -> Result<(), String> {
        let actual = self.take();
        if std::mem::discriminant(&actual) == std::mem::discriminant(&expected) {
            Ok(())
        } else {
            Err(format!("expected {expected:?}, got {actual:?}"))
        }
    }
    fn ident(&mut self) -> Result<String, String> {
        match self.take() {
            TokenKind::Ident(name) => Ok(name),
            other => Err(format!("expected identifier, got {other:?}")),
        }
    }
    fn lookup(&self, name: &str) -> Option<usize> {
        self.locals.iter().rposition(|local| local == name)
    }
    fn enter_scope(&mut self) {
        self.scopes.push(Vec::new());
    }
    fn leave_scope(&mut self) {
        if let Some(names) = self.scopes.pop() {
            for name in names.into_iter().rev() {
                if self.locals.last() == Some(&name) {
                    self.locals.pop();
                    self.local_types.pop();
                }
            }
        }
    }
    fn declare(&mut self, name: String, ty: StaticType) -> Result<usize, String> {
        let scope = self
            .scopes
            .last_mut()
            .ok_or("internal compiler error: missing scope")?;
        if scope.contains(&name) {
            return Err(format!("duplicate local '{name}' in the same scope"));
        }
        let slot = self.locals.len();
        scope.push(name.clone());
        self.locals.push(name);
        self.local_types.push(ty);
        Ok(slot)
    }
    fn program(&mut self) -> Result<Vec<Instruction>, String> {
        let mut functions: HashMap<String, usize> = HashMap::new();
        let mut definitions = Vec::new();
        while *self.kind() != TokenKind::Eof {
            self.eat(TokenKind::Fn)?;
            let name = self.ident()?;
            self.eat(TokenKind::LParen)?;
            let mut params = Vec::new();
            if *self.kind() != TokenKind::RParen {
                loop {
                    let param = self.ident()?;
                    if params.contains(&param) {
                        return Err(format!(
                            "duplicate parameter '{param}' in function '{name}'"
                        ));
                    }
                    params.push(param);
                    if *self.kind() != TokenKind::Comma {
                        break;
                    }
                    self.take();
                }
            }
            self.eat(TokenKind::RParen)?;
            self.eat(TokenKind::LBrace)?;
            if functions.insert(name.clone(), params.len()).is_some() {
                return Err(format!("duplicate function '{name}'"));
            }
            self.locals = params.clone();
            self.local_types = vec![StaticType::Int; params.len()];
            self.scopes = vec![params.clone()];
            self.code.clear();
            self.loops.clear();
            let mut definitely_returns = false;
            while *self.kind() != TokenKind::RBrace && *self.kind() != TokenKind::Eof {
                if definitely_returns {
                    return Err(format!("unreachable statement in function '{name}'"));
                }
                definitely_returns = self.statement()?;
            }
            self.eat(TokenKind::RBrace)?;
            if name != "main" && !definitely_returns {
                return Err(format!("function '{name}' may finish without returning an integer"));
            }
            let mut body = std::mem::take(&mut self.code);
            body.push(Instruction::Const(0));
            body.push(Instruction::Return);
            definitions.push(Instruction::DefineFunction(name, params, body));
            self.locals.clear();
            self.local_types.clear();
            self.scopes.clear();
        }
        if !functions.contains_key("main") {
            return Err("missing fn main()".into());
        }
        if functions["main"] != 0 {
            return Err("main function must not take parameters".into());
        }
        for definition in &definitions {
            if let Instruction::DefineFunction(name, _, body) = definition {
                Self::validate_calls(body, &functions, name)?;
            }
        }
        Ok(definitions)
    }
    fn validate_calls(
        code: &[Instruction],
        functions: &HashMap<String, usize>,
        owner: &str,
    ) -> Result<(), String> {
        for instruction in code {
            match instruction {
                Instruction::Call(name, argc) => {
                    let expected = functions.get(name).ok_or_else(|| {
                        format!("function '{owner}' calls unknown function '{name}'")
                    })?;
                    if expected != argc {
                        return Err(format!(
                            "function '{name}' expects {expected} argument(s), got {argc}"
                        ));
                    }
                }
                Instruction::Submit(body) => Self::validate_calls(body, functions, owner)?,
                _ => {}
            }
        }
        Ok(())
    }
    fn block(&mut self) -> Result<bool, String> {
        self.eat(TokenKind::LBrace)?;
        self.enter_scope();
        let mut definitely_returns = false;
        while *self.kind() != TokenKind::RBrace && *self.kind() != TokenKind::Eof {
            if definitely_returns {
                return Err("unreachable statement in block".into());
            }
            definitely_returns = self.statement()?;
        }
        self.eat(TokenKind::RBrace)?;
        self.leave_scope();
        Ok(definitely_returns)
    }
    fn statement(&mut self) -> Result<bool, String> {
        match self.take() {
            TokenKind::Let => {
                let name = self.ident()?;
                self.eat(TokenKind::Equal)?;
                let ty = self.expression(0)?;
                let slot = self.declare(name, ty)?;
                self.code.push(Instruction::Store(slot));
                self.eat(TokenKind::Semicolon)?;
                Ok(false)
            }
            TokenKind::Ident(name) => {
                self.eat(TokenKind::Equal)?;
                let slot = self
                    .lookup(&name)
                    .ok_or_else(|| format!("assignment to unknown variable '{name}'"))?;
                let ty = self.expression(0)?;
                if self.local_types[slot] != ty {
                    return Err(format!(
                        "cannot assign {ty:?} to {name} ({:?})",
                        self.local_types[slot]
                    ));
                }
                self.code.push(Instruction::Store(slot));
                self.eat(TokenKind::Semicolon)?;
                Ok(false)
            }
            TokenKind::Print => {
                self.eat(TokenKind::LParen)?;
                let ty = self.expression(0)?;
                if ty != StaticType::Int {
                    return Err("print expects an integer expression".into());
                }
                self.eat(TokenKind::RParen)?;
                self.eat(TokenKind::Semicolon)?;
                self.code.push(Instruction::Print);
                Ok(false)
            }
            TokenKind::If => self.if_statement(),
            TokenKind::While => self.while_statement(),
            TokenKind::Break => {
                self.eat(TokenKind::Semicolon)?;
                let jumps = self.loops.last_mut().ok_or("'break' used outside a loop")?;
                jumps.push(self.code.len());
                self.code.push(Instruction::Jump(usize::MAX));
                Ok(false)
            }
            TokenKind::Continue => {
                self.eat(TokenKind::Semicolon)?;
                let jumps = self
                    .loops
                    .last_mut()
                    .ok_or("'continue' used outside a loop")?;
                jumps.push(usize::MAX - self.code.len());
                self.code.push(Instruction::Jump(usize::MAX));
                Ok(false)
            }
            TokenKind::Return => {
                let ty = self.expression(0)?;
                if ty != StaticType::Int {
                    return Err("return expects an integer expression".into());
                }
                self.eat(TokenKind::Semicolon)?;
                self.code.push(Instruction::Return);
                Ok(true)
            }
            other => Err(format!("unsupported statement {other:?}")),
        }
    }
    fn if_statement(&mut self) -> Result<bool, String> {
        self.eat(TokenKind::LParen)?;
        let condition_type = self.expression(0)?;
        if condition_type != StaticType::Int {
            return Err("if condition must be an integer (zero is false)".into());
        }
        self.eat(TokenKind::RParen)?;
        let branch = self.code.len();
        self.code.push(Instruction::JumpIfFalse(usize::MAX));
        let then_returns = self.block()?;
        if *self.kind() == TokenKind::Else {
            self.take();
            let end_jump = self.code.len();
            self.code.push(Instruction::Jump(usize::MAX));
            let else_start = self.code.len();
            self.code[branch] = Instruction::JumpIfFalse(else_start);
            let else_returns = if *self.kind() == TokenKind::If {
                self.take();
                self.if_statement()?
            } else {
                self.block()?
            };
            let end = self.code.len();
            self.code[end_jump] = Instruction::Jump(end);
            Ok(then_returns && else_returns)
        } else {
            let end = self.code.len();
            self.code[branch] = Instruction::JumpIfFalse(end);
            Ok(false)
        }
    }
    fn while_statement(&mut self) -> Result<bool, String> {
        let condition = self.code.len();
        self.eat(TokenKind::LParen)?;
        let condition_type = self.expression(0)?;
        if condition_type != StaticType::Int {
            return Err("while condition must be an integer (zero is false)".into());
        }
        self.eat(TokenKind::RParen)?;
        let exit = self.code.len();
        self.code.push(Instruction::JumpIfFalse(usize::MAX));
        self.loops.push(Vec::new());
        self.block()?;
        self.code.push(Instruction::Jump(condition));
        let end = self.code.len();
        self.code[exit] = Instruction::JumpIfFalse(end);
        for encoded in self.loops.pop().unwrap_or_default() {
            if encoded == usize::MAX {
                continue;
            }
            if encoded > usize::MAX / 2 {
                let at = usize::MAX - encoded;
                self.code[at] = Instruction::Jump(condition);
            } else {
                self.code[encoded] = Instruction::Jump(end);
            }
        }
        Ok(false)
    }
    fn expression(&mut self, min_precedence: u8) -> Result<StaticType, String> {
        let mut expression_type = match self.take() {
            TokenKind::Number(value) => {
                self.code.push(Instruction::Const(value));
                StaticType::Int
            },
            TokenKind::Ident(name) => {
                if *self.kind() == TokenKind::LParen {
                    self.take();
                    let mut argc = 0;
                    if *self.kind() != TokenKind::RParen {
                        loop {
                            let arg_type = self.expression(0)?;
                            if arg_type != StaticType::Int {
                                return Err("function arguments must be integers".into());
                            }
                            argc += 1;
                            if *self.kind() != TokenKind::Comma {
                                break;
                            }
                            self.take();
                        }
                    }
                    self.eat(TokenKind::RParen)?;
                    self.code.push(Instruction::Call(name, argc));
                    StaticType::Int
                } else {
                    let slot = self
                        .lookup(&name)
                        .ok_or_else(|| format!("unknown variable '{name}'"))?;
                    self.code.push(Instruction::Load(slot));
                    self.local_types[slot]
                }
            }
            TokenKind::LParen => {
                let ty = self.expression(0)?;
                self.eat(TokenKind::RParen)?;
                ty
            }
            TokenKind::Worker => {
                self.eat(TokenKind::Dot)?;
                match self.take() {
                    TokenKind::Submit => {
                        self.eat(TokenKind::LBrace)?;
                        let outer_code = std::mem::take(&mut self.code);
                        let task_type = self.expression(0)?;
                        if task_type != StaticType::Int {
                            return Err("worker task must produce an integer".into());
                        }
                        let mut task_code = std::mem::replace(&mut self.code, outer_code);
                        task_code.push(Instruction::Halt);
                        self.eat(TokenKind::RBrace)?;
                        self.code.push(Instruction::Submit(task_code));
                        StaticType::Task
                    }
                    TokenKind::Wait => {
                        self.eat(TokenKind::LParen)?;
                        let handle_type = self.expression(0)?;
                        if handle_type != StaticType::Task {
                            return Err("worker.wait expects a task handle".into());
                        }
                        self.eat(TokenKind::RParen)?;
                        self.code.push(Instruction::Wait);
                        StaticType::Int
                    }
                    other => return Err(format!("expected submit or wait, got {other:?}")),
                }
            }
            other => return Err(format!("expected expression, got {other:?}")),
        };
        loop {
            let (precedence, op) = match self.kind() {
                TokenKind::EqualEqual => (1, Some(Instruction::Eq)),
                TokenKind::BangEqual => (1, Some(Instruction::Ne)),
                TokenKind::Less => (1, Some(Instruction::Lt)),
                TokenKind::LessEqual => (1, Some(Instruction::Le)),
                TokenKind::Greater => (1, Some(Instruction::Gt)),
                TokenKind::GreaterEqual => (1, Some(Instruction::Ge)),
                TokenKind::Plus => (2, Some(Instruction::Add)),
                TokenKind::Minus => (2, Some(Instruction::Sub)),
                TokenKind::Star => (3, Some(Instruction::Mul)),
                TokenKind::Slash => (3, Some(Instruction::Div)),
                _ => (0, None),
            };
            if precedence == 0 || precedence < min_precedence {
                break;
            }
            if expression_type != StaticType::Int {
                return Err("task handles cannot be used with operators".into());
            }
            self.take();
            let right_type = self.expression(precedence + 1)?;
            if right_type != StaticType::Int {
                return Err("operators require integer operands".into());
            }
            self.code.push(op.expect("operator was matched"));
            expression_type = StaticType::Int;
        }
        Ok(expression_type)
    }
}
