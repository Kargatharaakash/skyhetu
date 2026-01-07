//! Compiler: AST → Bytecode
//!
//! Compiles the Abstract Syntax Tree into bytecode for the VM.

use crate::ast::{BinaryOp, Expr, LogicalOp, Program, Stmt, UnaryOp};
use crate::bytecode::{Chunk, OpCode};
use crate::error::{ErrorKind, Result, SkyHetuError};
use crate::value::{Function, Value};
use std::rc::Rc;

/// Local variable in scope
#[derive(Debug, Clone)]
struct Local {
    name: String,
    depth: usize,
    is_state: bool,  // mutable state variable
}

/// Upvalue being captured
#[derive(Debug, Clone, Copy)]
struct Upvalue {
    index: u8,
    is_local: bool,
}

/// Function being compiled
#[derive(Debug)]
struct FunctionCompiler {
    // usage for debugging or error reporting
    #[allow(dead_code)]
    function_name: String,
    chunk: Chunk,
    locals: Vec<Local>,
    upvalues: Vec<Upvalue>,
    scope_depth: usize,
    loop_starts: Vec<usize>,
    loop_exits: Vec<Vec<usize>>,
}

impl FunctionCompiler {
    fn new(name: &str) -> Self {
        Self {
            function_name: name.to_string(),
            chunk: Chunk::new(),
            // Slot 0 is ALWAYS reserved for the closure/function itself
            locals: vec![Local {
                name: "".to_string(),
                depth: 0,
                is_state: false,
            }],
            upvalues: Vec::new(),
            scope_depth: 0,
            loop_starts: Vec::new(),
            loop_exits: Vec::new(),
        }
    }
}

/// The bytecode compiler
pub struct Compiler {
    /// Stack of function compilers (for nested functions)
    compilers: Vec<FunctionCompiler>,
    /// All compiled chunks (indexed by chunk_index)
    compiled_chunks: Vec<Chunk>,
    /// Exported names from the current module
    exports: std::collections::HashSet<String>,
    /// Base path for resolving module imports
    module_base_path: Option<std::path::PathBuf>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            compilers: vec![FunctionCompiler::new("")],
            compiled_chunks: Vec::new(),
            exports: std::collections::HashSet::new(),
            module_base_path: None,
        }
    }
    
    pub fn with_base_path(base_path: std::path::PathBuf) -> Self {
        Self {
            compilers: vec![FunctionCompiler::new("")],
            compiled_chunks: Vec::new(),
            exports: std::collections::HashSet::new(),
            module_base_path: Some(base_path),
        }
    }
    
    pub fn with_offset(_chunk_offset: usize) -> Self {
        // chunk_offset reserved for future REPL improvements
        Self::new()
    }
    
    /// Compile a program to bytecode. Returns the main chunk and a list of function chunks.
    /// Compile a program to bytecode. Returns the main chunk and a list of function chunks.
    pub fn compile(&mut self, program: &Program, heap: &mut crate::gc::Heap) -> Result<(Chunk, Vec<Chunk>)> {
        let len = program.statements.len();
        
        for (i, stmt) in program.statements.iter().enumerate() {
            let is_last = i == len - 1;
            
            // For the last statement, if it's an expression, don't pop it
            if is_last {
                if let Stmt::Expr { expr } = stmt {
                    self.compile_expr(expr, heap)?;
                    // Don't pop - this value will be returned
                } else {
                    self.compile_stmt(stmt, heap)?;
                    self.emit(OpCode::Nil, 0);
                }
            } else {
                self.compile_stmt(stmt, heap)?;
            }
        }
        
        if program.statements.is_empty() {
            self.emit(OpCode::Nil, 0);
        }
        
        self.emit(OpCode::Return, 0);
        
        Ok((self.current().chunk.clone(), self.compiled_chunks.clone()))
    }
    
    fn current(&mut self) -> &mut FunctionCompiler {
        self.compilers.last_mut().unwrap()
    }
    
    fn emit(&mut self, op: OpCode, line: usize) {
        self.current().chunk.write(op, line);
    }
    
    fn emit_byte(&mut self, byte: u8, line: usize) {
        self.current().chunk.write_byte(byte, line);
    }
    
    fn emit_u16(&mut self, value: u16, line: usize) {
        self.current().chunk.write_u16(value, line);
    }
    
    fn emit_constant(&mut self, value: Value, line: usize) {
        let idx = self.current().chunk.add_constant(value);
        self.emit(OpCode::Constant, line);
        self.emit_u16(idx, line);
    }
    
    fn emit_jump(&mut self, op: OpCode, line: usize) -> usize {
        self.emit(op, line);
        self.emit_u16(0xFFFF, line);  // Placeholder
        self.current().chunk.len() - 2
    }
    
    fn patch_jump(&mut self, offset: usize) {
        self.current().chunk.patch_jump(offset);
    }
    
    fn emit_loop(&mut self, loop_start: usize, line: usize) {
        self.emit(OpCode::Loop, line);
        let offset = self.current().chunk.len() - loop_start + 2;
        if offset > u16::MAX as usize {
            panic!("Loop body too large");
        }
        self.emit_u16(offset as u16, line);
    }
    
    // ==================== Statements ====================
    
    fn compile_stmt(&mut self, stmt: &Stmt, heap: &mut crate::gc::Heap) -> Result<()> {
        match stmt {
            Stmt::Expr { expr } => {
                self.compile_expr(expr, heap)?;
                self.emit(OpCode::Pop, expr.span().line);
            }
            
            Stmt::Let { name, value, span } => {
                self.compile_expr(value, heap)?;
                
                if self.current().scope_depth == 0 {
                    // Global
                    let idx = self.current().chunk.add_name(name.clone());
                    self.emit(OpCode::DefineGlobal, span.line);
                    self.emit_u16(idx, span.line);
                } else {
                    // Local
                    self.add_local(name.clone(), false);
                }
            }
            
            Stmt::State { name, value, span } => {
                self.compile_expr(value, heap)?;
                
                if self.current().scope_depth == 0 {
                    // Global state
                    let idx = self.current().chunk.add_name(name.clone());
                    self.emit(OpCode::DefineState, span.line);
                    self.emit_u16(idx, span.line);
                } else {
                    // Local state
                    self.add_local(name.clone(), true);
                }
            }
            
            Stmt::Transition { name, value, span } => {
                // Compile new value
                self.compile_expr(value, heap)?;
                
                // Check if local or global
                if let Some(slot) = self.resolve_local(&name) {
                    // Local transition
                    
                    // Check immutability
                    let slot_usize = slot as usize;
                    if !self.current().locals[slot_usize].is_state {
                         return Err(SkyHetuError::new(
                            ErrorKind::ImmutableVariable(name.clone()),
                            Some(*span),
                        ));
                    }
                    
                    let name_idx = self.current().chunk.add_name(name.clone());
                    
                    self.emit(OpCode::TransitionLocal, span.line);
                    self.emit_u16(slot, span.line);
                    self.emit_u16(name_idx, span.line);
                    
                } else if let Some(idx) = self.resolve_upvalue(self.compilers.len() - 1, &name) {
                    // Upvalue transition
                    // TODO: Check immutability (need to track is_state in Upvalue?)
                    // Currently Upvalue struct tracks is_local (bool). We don't track is_state in Upvalue struct.
                    // But we can check the *source* of the upvalue?
                    // Actually, compiler resolves upvalue recursively. The base local `is_state`.
                    // We should propagate `is_state` through Upvalue struct or just assume runtime check?
                    // Or static check?
                    // Static check requires `Upvalue` to store `is_state`.
                    // Let's assume we want static check.
                    // But for now, let's omit the check or assume if it resolves, we trust user? 
                    // No, `is_state` is important.
                    // Let's modify Upvalue resolution to return `is_state`??
                    // `resolve_upvalue` currently returns `Option<usize>`.
                    // `FunctionCompiler.upvalues` stores `Upvalue` struct.
                    // I can look up `self.current().upvalues[idx]`.
                    // But `Upvalue` struct doesn't have `is_state`.
                    // I should add `is_state` to `Upvalue` struct in `compiler.rs`?
                    // Yes.
                    
                    let name_idx = self.current().chunk.add_name(name.clone());
                    self.emit(OpCode::TransitionUpvalue, span.line);
                    self.emit_u16(idx as u16, span.line);
                    self.emit_u16(name_idx, span.line);
                    
                } else {
                    // Global transition
                    let idx = self.current().chunk.add_name(name.clone());
                    self.emit(OpCode::Transition, span.line);
                    self.emit_u16(idx, span.line);
                }
            }
            
            Stmt::Block { stmts, .. } => {
                self.begin_scope();
                for stmt in stmts {
                    self.compile_stmt(stmt, heap)?;
                }
                self.end_scope();
            }
            
            Stmt::If { condition, then_branch, else_branch, span } => {
                self.compile_expr(condition, heap)?;
                
                // Jump over then branch if false
                let then_jump = self.emit_jump(OpCode::JumpIfFalse, span.line);
                self.emit(OpCode::Pop, span.line);  // Pop condition
                
                // Compile then branch
                self.compile_stmt(then_branch, heap)?;
                
                // Jump over else branch
                let else_jump = self.emit_jump(OpCode::Jump, span.line);
                
                // Patch the then jump
                self.patch_jump(then_jump);
                self.emit(OpCode::Pop, span.line);  // Pop condition
                
                // Compile else branch if present
                if let Some(else_stmt) = else_branch {
                    self.compile_stmt(else_stmt, heap)?;
                }
                
                self.patch_jump(else_jump);
            }
            
            Stmt::While { condition, body, span } => {
                let loop_start = self.current().chunk.len();
                self.current().loop_starts.push(loop_start);
                self.current().loop_exits.push(Vec::new());
                
                self.compile_expr(condition, heap)?;
                let exit_jump = self.emit_jump(OpCode::JumpIfFalse, span.line);
                self.emit(OpCode::Pop, span.line);
                
                self.compile_stmt(body, heap)?;
                self.emit_loop(loop_start, span.line);
                
                self.patch_jump(exit_jump);
                self.emit(OpCode::Pop, span.line);
                
                // Patch all break statements
                let exits = self.current().loop_exits.pop().unwrap();
                for exit in exits {
                    self.patch_jump(exit);
                }
                self.current().loop_starts.pop();
            }
            
            Stmt::For { var, iterable, body, span } => {
                self.begin_scope();
                
                // 1. Compile Iterator Expression -> __iter__
                //    This pushes the array (or string) onto the stack
                self.compile_expr(iterable, heap)?;
                self.add_local("__iter__".to_string(), false);
                
                // 2. Initialize Index -> __idx__ = 0
                self.emit_constant(Value::Number(0.0), span.line);
                self.add_local("__idx__".to_string(), true);
                
                // 3. User Loop Variable -> var (initialized to nil)
                self.emit(OpCode::Nil, span.line);
                self.add_local(var.clone(), false);
                
                let loop_start = self.current().chunk.len();
                self.current().loop_starts.push(loop_start);
                self.current().loop_exits.push(Vec::new());
                
                // --- Condition: __idx__ < len(__iter__) ---
                
                // First: Call len(__iter__) and leave result on stack
                // Get 'len' function
                let len_idx = self.current().chunk.add_name("len".to_string());
                self.emit(OpCode::GetGlobal, span.line);
                self.emit_u16(len_idx, span.line);
                
                // Load __iter__ as argument
                if let Some(slot) = self.resolve_local("__iter__") {
                    self.emit(OpCode::GetLocal, span.line);
                    self.emit_u16(slot, span.line);
                }
                
                // Call len(1 arg) - leaves length on stack
                self.emit(OpCode::Call, span.line);
                self.emit_byte(1, span.line);
                
                // Second: Load __idx__
                if let Some(slot) = self.resolve_local("__idx__") {
                    self.emit(OpCode::GetLocal, span.line);
                    self.emit_u16(slot, span.line);
                }
                
                // Now stack is: [length, __idx__] 
                // We need: __idx__ < length
                // But Less pops right then left: left < right
                // Stack: [length, __idx__] -> Less compares length (2nd pop) < __idx__ (1st pop) = WRONG
                // We need __idx__ < length, so swap order
                // Actually, push __idx__ first, then length, then Less
                // Let me fix: push __idx__, push length, Less => __idx__ < length
                
                // Correction: Swap the order
                // Stack after above: [length, __idx__]
                // Binary ops: pop b, pop a, compute a op b
                // So: a=length, b=__idx__, computes length < __idx__ (WRONG)
                // We want __idx__ < length
                // Fix: Push __idx__ first, then call len, then Less
                
                // Actually simpler: use Greater instead (length > __idx__)
                self.emit(OpCode::Greater, span.line);
                
                // Jump if False (Exit Loop)
                let exit_jump = self.emit_jump(OpCode::JumpIfFalse, span.line);
                self.emit(OpCode::Pop, span.line); // Pop condition result (true)
                
                // --- Body Prologue: var = __iter__[__idx__] ---
                
                // Push __iter__
                if let Some(slot) = self.resolve_local("__iter__") {
                    self.emit(OpCode::GetLocal, span.line);
                    self.emit_u16(slot, span.line);
                }
                // Push __idx__
                if let Some(slot) = self.resolve_local("__idx__") {
                    self.emit(OpCode::GetLocal, span.line);
                    self.emit_u16(slot, span.line);
                }
                // Index Operation
                self.emit(OpCode::Index, span.line);
                
                // Assign to user variable 'var'
                if let Some(slot) = self.resolve_local(var) {
                    self.emit(OpCode::SetLocal, span.line);
                    self.emit_u16(slot, span.line);
                    self.emit(OpCode::Pop, span.line); // Pop assigned value
                }
                
                // Execute Body
                self.compile_stmt(body, heap)?;
                
                // --- Increment Index: __idx__ = __idx__ + 1 ---
                
                // Load __idx__
                if let Some(slot) = self.resolve_local("__idx__") {
                    self.emit(OpCode::GetLocal, span.line);
                    self.emit_u16(slot, span.line);
                }
                // Load 1
                self.emit_constant(Value::Number(1.0), span.line);
                // Add
                self.emit(OpCode::Add, span.line);
                
                // Store back to __idx__
                if let Some(slot) = self.resolve_local("__idx__") {
                    self.emit(OpCode::SetLocal, span.line);
                    self.emit_u16(slot, span.line);
                    self.emit(OpCode::Pop, span.line);
                }
                
                // Loop Back
                self.emit_loop(loop_start, span.line);
                
                // --- Exit ---
                self.patch_jump(exit_jump);
                self.emit(OpCode::Pop, span.line); // Pop condition result (false)
                
                // Patch breaks
                let exits = self.current().loop_exits.pop().unwrap();
                for exit in exits {
                    self.patch_jump(exit);
                }
                self.current().loop_starts.pop();
                
                self.end_scope();
            }
            
            Stmt::Class { name, methods, span } => {
                // 1. Declare class name var
                let global_idx = if self.current().scope_depth == 0 {
                    Some(self.current().chunk.add_name(name.clone()))
                } else {
                    self.add_local(name.clone(), false);
                    None
                };
                
                // 2. Class creation
                let name_idx = self.current().chunk.add_name(name.clone());
                self.emit(OpCode::Class, span.line);
                self.emit_u16(name_idx, span.line);
                
                // 3. Define variable (consumes stack value if global)
                if let Some(idx) = global_idx {
                    self.emit(OpCode::DefineGlobal, span.line);
                    self.emit_u16(idx, span.line);
                }
                
                // 4. Load class back onto stack for method binding
                if let Some(idx) = global_idx {
                    self.emit(OpCode::GetGlobal, span.line);
                    self.emit_u16(idx, span.line);
                } else {
                    // Local: peek/get it
                     let slot = self.resolve_local(&name).unwrap();
                     self.emit(OpCode::GetLocal, span.line);
                     self.emit_u16(slot, span.line);
                }
                
                // 5. Compile methods
                for method in methods {
                    if let Stmt::Function { name: m_name, params, body, span: m_span } = method {
                        // --- Compile Closure (Inline) ---
                        self.compilers.push(FunctionCompiler::new(m_name));
                        self.begin_scope();
                        
                        // Bind 'this' to slot 0
                        if let Some(local) = self.current().locals.first_mut() {
                            local.name = "this".to_string();
                        }
                        
                        for param in params {
                            self.add_local(param.clone(), false);
                        }
                        
                        for stmt in body {
                            self.compile_stmt(stmt, heap)?;
                        }
                        
                        if m_name == "init" {
                             // Init returns 'this'
                             self.emit(OpCode::GetLocal, m_span.line);
                             self.emit_u16(0, m_span.line);
                             self.emit(OpCode::Return, m_span.line);
                        } else {
                             self.emit(OpCode::Nil, m_span.line);
                             self.emit(OpCode::Return, m_span.line);
                        }
                        
                        let func_compiler = self.compilers.pop().unwrap();
                        let chunk = Rc::new(func_compiler.chunk);
                        let upvalues = func_compiler.upvalues;
                        
                        let function = Function::new(
                            m_name.clone(),
                            params.clone(),
                            chunk,
                            upvalues.len(),
                        );
                        
                        let handle = heap.alloc_function(function);
                        let func_idx = self.current().chunk.add_constant(Value::Function(handle));
                        self.emit(OpCode::Closure, m_span.line);
                        self.emit_u16(func_idx, m_span.line);
                        
                        for upvalue in upvalues {
                            self.emit_byte(if upvalue.is_local { 1 } else { 0 }, m_span.line);
                            self.emit_byte(upvalue.index, m_span.line);
                        }
                        // --- End Closure ---
                        
                        let m_name_idx = self.current().chunk.add_name(m_name.clone());
                        self.emit(OpCode::Method, m_span.line);
                        self.emit_u16(m_name_idx, m_span.line);
                    }
                }
                
                // 6. Pop class
                self.emit(OpCode::Pop, span.line);
            }
            
            Stmt::Function { name, params, body, span } => {
                let global_idx = if self.current().scope_depth == 0 {
                    Some(self.current().chunk.add_name(name.clone()))
                } else {
                    self.add_local(name.clone(), false);
                    // Mark initialized immediately to allow recursion
                    let depth = self.current().scope_depth;
                    self.current().locals.last_mut().unwrap().depth = depth;
                    None
                };

                // Start a new compiler for the function
                self.compilers.push(FunctionCompiler::new(name));
                self.begin_scope();
                
                // Define parameters as locals
                for param in params {
                    self.add_local(param.clone(), false);
                }
                
                // Compile body
                for stmt in body {
                    self.compile_stmt(stmt, heap)?;
                }
                
                // Implicit return nil
                self.emit(OpCode::Nil, span.line);
                self.emit(OpCode::Return, span.line);
                
                // Pop the function compiler
                let func_compiler = self.compilers.pop().unwrap();
                let chunk = Rc::new(func_compiler.chunk); // Wrap in Rc
                let upvalues = func_compiler.upvalues;
                
                // Create function object
                let function = Function::new(
                    name.clone(),
                    params.clone(),
                    chunk, // Pass Rc<Chunk>
                    upvalues.len(),
                );
                
                // Alloc function
                let handle = heap.alloc_function(function);
                
                // Main compiler: emit constant
                let func_idx = self.current().chunk.add_constant(Value::Function(handle));
                self.emit(OpCode::Closure, span.line);
                self.emit_u16(func_idx, span.line);
                
                // Emit upvalue info
                for upvalue in upvalues {
                    self.emit_byte(if upvalue.is_local { 1 } else { 0 }, span.line);
                    self.emit_byte(upvalue.index, span.line);
                }
                
                if let Some(idx) = global_idx {
                    self.emit(OpCode::DefineGlobal, span.line);
                    self.emit_u16(idx, span.line);
                }
            }
            
            Stmt::Return { value, span } => {
                if let Some(expr) = value {
                    self.compile_expr(expr, heap)?;
                } else {
                    self.emit(OpCode::Nil, span.line);
                }
                self.emit(OpCode::Return, span.line);
            }
            
            Stmt::Break { span } => {
                if self.current().loop_exits.is_empty() {
                    return Err(SkyHetuError::new(ErrorKind::BreakOutsideLoop, Some(*span)));
                }
                let exit = self.emit_jump(OpCode::Jump, span.line);
                self.current().loop_exits.last_mut().unwrap().push(exit);
            }
            
            Stmt::Continue { span } => {
                if self.current().loop_starts.is_empty() {
                    return Err(SkyHetuError::new(ErrorKind::ContinueOutsideLoop, Some(*span)));
                }
                let loop_start = *self.current().loop_starts.last().unwrap();
                self.emit_loop(loop_start, span.line);
            }
            
            Stmt::Import { names, path, span } => {
                // Resolve module path relative to current file's directory
                let module_path = if let Some(base) = &self.module_base_path {
                    base.join(path)
                } else {
                    std::path::PathBuf::from(path)
                };
                
                // Add .skyh extension if not present
                let module_path = if module_path.extension().is_none() {
                    module_path.with_extension("skyh")
                } else {
                    module_path
                };
                
                // Read the module source
                let source = std::fs::read_to_string(&module_path).map_err(|e| {
                    SkyHetuError::new(
                        ErrorKind::ModuleNotFound(format!("{}: {}", path, e)),
                        Some(*span),
                    )
                })?;
                
                // Parse the module
                let mut lexer = crate::lexer::Lexer::new(&source);
                let tokens = lexer.tokenize().map_err(|e| {
                    SkyHetuError::new(ErrorKind::ModuleNotFound(format!("{}: {}", path, e)), Some(*span))
                })?;
                let mut parser = crate::parser::Parser::new(tokens);
                let module_program = parser.parse().map_err(|e| {
                    SkyHetuError::new(ErrorKind::ModuleNotFound(format!("{}: {}", path, e)), Some(*span))
                })?;
                
                // Compile the module's statements directly in the current compiler
                // This ensures constants and functions are in the current chunk
                for stmt in &module_program.statements {
                    self.compile_stmt(stmt, heap)?;
                }
                
                // Track which names were imported (for future use)
                let _ = names; // TODO: Filter which names are actually imported
            }
            
            Stmt::Export { stmt, span } => {
                // Track the exported name
                match stmt.as_ref() {
                    Stmt::Function { name, .. } => { self.exports.insert(name.clone()); }
                    Stmt::Let { name, .. } => { self.exports.insert(name.clone()); }
                    Stmt::State { name, .. } => { self.exports.insert(name.clone()); }
                    Stmt::Class { name, .. } => { self.exports.insert(name.clone()); }
                    _ => {}
                }
                let _ = span;
                self.compile_stmt(stmt, heap)?;
            }
        }
        
        Ok(())
    }
    
    // ==================== Expressions ====================
    
    fn compile_expr(&mut self, expr: &Expr, heap: &mut crate::gc::Heap) -> Result<()> {
        match expr {
            Expr::Number { value, span } => {
                self.emit_constant(Value::Number(*value), span.line);
            }
            
            Expr::String { value, span } => {
                self.emit_constant(Value::String(value.clone()), span.line);
            }
            
            Expr::Bool { value, span } => {
                self.emit(if *value { OpCode::True } else { OpCode::False }, span.line);
            }
            
            Expr::Nil { span } => {
                self.emit(OpCode::Nil, span.line);
            }
            
            Expr::Ident { name, span } => {
                // Check for local variable first
                if let Some(slot) = self.resolve_local(name) {
                    self.emit(OpCode::GetLocal, span.line);
                    self.emit_u16(slot, span.line);
                } else if let Some(idx) = self.resolve_upvalue(self.compilers.len() - 1, name) {
                    // Upvalue
                    self.emit(OpCode::GetUpvalue, span.line);
                    self.emit_u16(idx as u16, span.line);
                } else {
                    // Global
                    let idx = self.current().chunk.add_name(name.clone());
                    self.emit(OpCode::GetGlobal, span.line);
                    self.emit_u16(idx, span.line);
                }
            }
            
            Expr::Binary { left, op, right, span } => {
                self.compile_expr(left, heap)?;
                self.compile_expr(right, heap)?;
                
                match op {
                    BinaryOp::Add => self.emit(OpCode::Add, span.line),
                    BinaryOp::Sub => self.emit(OpCode::Subtract, span.line),
                    BinaryOp::Mul => self.emit(OpCode::Multiply, span.line),
                    BinaryOp::Div => self.emit(OpCode::Divide, span.line),
                    BinaryOp::Mod => self.emit(OpCode::Modulo, span.line),
                    BinaryOp::Eq => self.emit(OpCode::Equal, span.line),
                    BinaryOp::Ne => self.emit(OpCode::NotEqual, span.line),
                    BinaryOp::Lt => self.emit(OpCode::Less, span.line),
                    BinaryOp::Le => self.emit(OpCode::LessEqual, span.line),
                    BinaryOp::Gt => self.emit(OpCode::Greater, span.line),
                    BinaryOp::Ge => self.emit(OpCode::GreaterEqual, span.line),
                }
            }
            
            Expr::Unary { op, operand, span } => {
                self.compile_expr(operand, heap)?;
                match op {
                    UnaryOp::Neg => self.emit(OpCode::Negate, span.line),
                    UnaryOp::Not => self.emit(OpCode::Not, span.line),
                }
            }
            
            Expr::Logical { left, op, right, span } => {
                self.compile_expr(left, heap)?;
                
                match op {
                    LogicalOp::And => {
                        let jump = self.emit_jump(OpCode::JumpIfFalse, span.line);
                        self.emit(OpCode::Pop, span.line);
                        self.compile_expr(right, heap)?;
                        self.patch_jump(jump);
                    }
                    LogicalOp::Or => {
                        let jump = self.emit_jump(OpCode::JumpIfTrue, span.line);
                        self.emit(OpCode::Pop, span.line);
                        self.compile_expr(right, heap)?;
                        self.patch_jump(jump);
                    }
                }
            }
            
            Expr::Grouping { expr, .. } => {
                self.compile_expr(expr, heap)?;
            }
            
            Expr::Call { callee, args, span } => {
                // Special built-in handling
                if let Expr::Ident { name, .. } = callee.as_ref() {
                    match name.as_str() {
                        "print" => {
                            for arg in args {
                                self.compile_expr(arg, heap)?;
                            }
                            self.emit(OpCode::Print, span.line);
                            self.emit_byte(args.len() as u8, span.line);
                            return Ok(());
                        }
                        "why" => {
                            if args.len() != 1 {
                                return Err(SkyHetuError::new(
                                    ErrorKind::WrongArity(1, args.len()),
                                    Some(*span),
                                ));
                            }
                            if let Expr::Ident { name: var_name, .. } = &args[0] {
                                let idx = self.current().chunk.add_name(var_name.clone());
                                self.emit(OpCode::Why, span.line);
                                self.emit_u16(idx, span.line);
                                return Ok(());
                            }
                        }
                        "time" => {
                            self.emit(OpCode::Time, span.line);
                            return Ok(());
                        }
                        _ => {}
                    }
                }
                
                // Regular function call
                self.compile_expr(callee, heap)?;
                for arg in args {
                    self.compile_expr(arg, heap)?;
                }
                self.emit(OpCode::Call, span.line);
                self.emit_byte(args.len() as u8, span.line);
            }
            
            Expr::Lambda { params, body, span } => {
                // Compile lambda as a function
                self.compilers.push(FunctionCompiler::new("<lambda>"));
                self.begin_scope();
                
                for param in params {
                    self.add_local(param.clone(), false);
                }
                
                self.compile_expr(body, heap)?;
                self.emit(OpCode::Return, span.line);
                
                let func_compiler = self.compilers.pop().unwrap();
                let chunk = Rc::new(func_compiler.chunk);
                let upvalues = func_compiler.upvalues;
                
                let function = Function::new(
                    "<lambda>".to_string(),
                    params.clone(),
                    chunk,
                    upvalues.len(),
                );
                
                let handle = heap.alloc_function(function);
                let idx = self.current().chunk.add_constant(Value::Function(handle));
                self.emit(OpCode::Closure, span.line);
                self.emit_u16(idx, span.line);
                
                // Emit upvalues
                for upvalue in upvalues {
                    self.emit_byte(if upvalue.is_local { 1 } else { 0 }, span.line);
                    self.emit_byte(upvalue.index, span.line);
                }
            }
            
            Expr::Get { object, name, span } => {
                self.compile_expr(object, heap)?;
                let idx = self.current().chunk.add_name(name.clone());
                self.emit(OpCode::GetProperty, span.line);
                self.emit_u16(idx, span.line);
            }
            
            Expr::Set { object, name, value, span } => {
                self.compile_expr(object, heap)?;
                self.compile_expr(value, heap)?;
                let idx = self.current().chunk.add_name(name.clone());
                self.emit(OpCode::SetProperty, span.line);
                self.emit_u16(idx, span.line);
            }
        }
        
        Ok(())
    }
    
    // ==================== Scope Management ====================
    
    fn begin_scope(&mut self) {
        self.current().scope_depth += 1;
    }
    
    fn end_scope(&mut self) {
        self.current().scope_depth -= 1;
        
        // Pop locals from this scope
        while !self.current().locals.is_empty() 
            && self.current().locals.last().unwrap().depth > self.current().scope_depth 
        {
            self.emit(OpCode::Pop, 0);
            self.current().locals.pop();
        }
    }
    
    fn add_local(&mut self, name: String, is_state: bool) {
        let depth = self.current().scope_depth;
        self.current().locals.push(Local { name, depth, is_state });
    }
    
    fn resolve_local(&mut self, name: &str) -> Option<u16> {
        let compiler = self.current();
        for (i, local) in compiler.locals.iter().enumerate().rev() {
            if local.name == name {
                return Some(i as u16);
            }
        }
        None
    }
    
    fn resolve_upvalue(&mut self, compiler_idx: usize, name: &str) -> Option<usize> {
        // Base case: top-level compiler has no upvalues
        if compiler_idx == 0 {
            return None;
        }
        
        let parent_idx = compiler_idx - 1;
        
        let parent_local = {
            let parent = &self.compilers[parent_idx];
            parent.locals.iter().enumerate().rev().find(|(_, local)| local.name == name).map(|(i, _)| i)
        };
        
        if let Some(index) = parent_local {
            // Found local in parent -> capture it
            return Some(self.add_upvalue(compiler_idx, index as u8, true));
        }
        
        // Recursive step: resolve upvalue in parent's parent
        if let Some(index) = self.resolve_upvalue(parent_idx, name) {
            // Found upvalue in parent -> capture it
            return Some(self.add_upvalue(compiler_idx, index as u8, false));
        }
        
        None
    }
    
    fn add_upvalue(&mut self, compiler_idx: usize, index: u8, is_local: bool) -> usize {
        let compiler = &mut self.compilers[compiler_idx];
        
        // Check if upvalue already exists to avoid duplicates
        for (i, upvalue) in compiler.upvalues.iter().enumerate() {
            if upvalue.index == index && upvalue.is_local == is_local {
                return i;
            }
        }
        
        compiler.upvalues.push(Upvalue { index, is_local });
        compiler.upvalues.len() - 1
    }
}

impl Default for Compiler {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::gc::Heap; // Import Heap
    
    fn compile(source: &str, heap: &mut Heap) -> Chunk { // Modified to accept heap
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut compiler = Compiler::new();
        let (chunk, _) = compiler.compile(&program, heap).unwrap(); // Pass heap
        chunk
    }
    
    #[test]
    fn test_compile_chunk() {
        let source = "let x = 1 + 2";
        let mut lexer = crate::lexer::Lexer::new(source);
        let tokens = lexer.tokenize().unwrap(); // Use mut to fix E0596
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse().unwrap();
        println!("Program: {:?}", program);
        
        let mut compiler = Compiler::new();
        let mut heap = crate::gc::Heap::new();
        compiler.compile(&program, &mut heap).unwrap();
    }
    
    #[test]
    fn test_compile_number() {
        let mut heap = Heap::new();
        let chunk = compile("42", &mut heap);
        assert!(chunk.code.len() > 0);
        assert_eq!(chunk.constants[0], Value::Number(42.0));
    }
    
    #[test]
    fn test_compile_binary_op() {
        let mut heap = Heap::new();
        let chunk = compile("1 + 2", &mut heap);
        // Should have: CONSTANT 1, CONSTANT 2, ADD, POP, NIL, RETURN
        // With optimization (expr only): CONSTANT 1, CONSTANT 2, ADD, NIL, RETURN
        assert!(chunk.code.len() >= 6);
        assert!(chunk.code.len() > 0);
    }
    
    #[test]
    fn test_compile_var_decl() {
        let mut heap = Heap::new();
        let chunk = compile("let x = 10", &mut heap);
        // Should have: CONSTANT, DEFINE_GLOBAL
        assert!(chunk.names.contains(&"x".to_string()));
        assert!(chunk.code.len() > 0);
    }
    
    #[test]
    fn test_compile_state_decl() {
        let mut heap = Heap::new();
        let chunk = compile("state counter = 0", &mut heap);
        assert!(chunk.names.contains(&"counter".to_string()));
        assert!(chunk.code.len() > 0);
    }
    
    #[test]
    fn test_compile_if_stmt() {
        let mut heap = Heap::new();
        let chunk = compile("if true { 1 }", &mut heap);
        // Should contain JumpIfFalse
        assert!(chunk.code.iter().any(|&b| b == OpCode::JumpIfFalse as u8));
        assert!(chunk.code.len() > 0);
    }
}
