//! Stack-based Virtual Machine for SkyHetu
//!
//! Executes bytecode with causality tracking.

use std::collections::HashMap;
use std::rc::Rc;
use crate::bytecode::{Chunk, OpCode};
use crate::causality::CausalityLog;
use crate::error::{ErrorKind, Result, SkyHetuError};

use crate::value::{NativeFn, Value};

/// Maximum stack size
const STACK_MAX: usize = 2048;

/// Maximum call depth
const FRAMES_MAX: usize = 64;

/// A call frame for function calls
#[derive(Debug, Clone)]
struct CallFrame {
    /// The closure being executed
    closure: crate::gc::Handle,
    
    /// The chunk being executed (cached from closure)
    chunk: Rc<Chunk>,
    
    /// Instruction pointer
    ip: usize,
    
    /// Stack slot where this frame begins
    slot: usize,
}

impl CallFrame {
    fn new(closure: crate::gc::Handle, chunk: Rc<Chunk>, slot: usize) -> Self {
        Self {
            closure,
            chunk,
            ip: 0,
            slot,
        }
    }
}

/// Binding in the VM
#[derive(Debug, Clone)]
struct Binding {
    value: Value,
    is_state: bool,
}

/// The Virtual Machine
pub struct VM {
    /// Value stack
    stack: Vec<Value>,
    
    /// Call frames
    frames: Vec<CallFrame>,
    
    /// Global variables
    globals: HashMap<String, Binding>,
    
    /// Causality log
    pub causality: CausalityLog,
    
    /// Compiled function chunks (indexed by chunk_index)
    function_chunks: Vec<Rc<Chunk>>,
    
    /// Garbage collected heap
    pub heap: crate::gc::Heap,

    /// Open upvalues (pointing to stack)
    open_upvalues: Vec<crate::gc::Handle>,
}

impl VM {
    pub fn new() -> Self {
        let mut vm = Self {
            frames: Vec::with_capacity(FRAMES_MAX),
            stack: Vec::with_capacity(STACK_MAX),
            globals: HashMap::new(),
            function_chunks: Vec::new(),
            causality: CausalityLog::new(),
            heap: crate::gc::Heap::new(),
            open_upvalues: Vec::new(),
        };

        
        vm.define_natives();
        vm
    }
    
    fn define_natives(&mut self) {
        let natives = vec![
            // len(val)
            NativeFn::new(
                "len",
                Some(1),
                |vm, args| {
                    match &args[0] {
                        Value::String(s) => Ok(Value::Number(s.len() as f64)),
                        Value::Array(handle) => {
                            if let Some(arr) = vm.heap.get_array(*handle) {
                                Ok(Value::Number(arr.len() as f64))
                            } else {
                                Err("Array not found (GC error?)".to_string())
                            }
                        }
                        _ => Err(format!("len() requires string or array")),
                    }
                },
            ),
            
            // substr(s, start, end?)
            NativeFn::new(
                "substr",
                None,
                |_vm, args| {
                    if args.is_empty() || args.len() > 3 {
                        return Err("substr() takes 2 or 3 arguments".to_string());
                    }
                    let s = match &args[0] {
                        Value::String(s) => s,
                        _ => return Err("substr() requires a string as first argument".to_string()),
                    };
                    let start = match &args[1] {
                        Value::Number(n) => *n as usize,
                        _ => return Err("substr() requires a number as second argument".to_string()),
                    };
                    let end = if args.len() == 3 {
                        match &args[2] {
                            Value::Number(n) => *n as usize,
                            _ => return Err("substr() requires a number as third argument".to_string()),
                        }
                    } else {
                        s.len()
                    };
                    let end = std::cmp::min(end, s.len());
                    let start = std::cmp::min(start, end);
                    Ok(Value::String(s[start..end].to_string()))
                },
            ),
            
            // str(val)
            NativeFn::new(
                "str",
                Some(1),
                |_vm, args| Ok(Value::String(format!("{}", args[0]))),
            ),
            
            // num(val)
            NativeFn::new(
                "num",
                Some(1),
                |_vm, args| {
                    match &args[0] {
                        Value::Number(n) => Ok(Value::Number(*n)),
                        Value::String(s) => s.parse::<f64>()
                            .map(Value::Number)
                            .map_err(|_| format!("cannot convert '{}' to number", s)),
                        Value::Bool(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
                        _ => Err(format!("cannot convert to number")),
                    }
                },
            ),
            
            // type(val)
            NativeFn::new(
                "type",
                Some(1),
                |_vm, args| Ok(Value::String(args[0].type_name().to_string())),
            ),
            
            // range(n) or range(start, end)
            NativeFn::new(
                "range",
                None,
                |vm, args| {
                    let (start, end) = match args.len() {
                        1 => {
                            if let Value::Number(n) = &args[0] {
                                (0, *n as i64)
                            } else {
                                return Err("range() requires number".to_string());
                            }
                        }
                        2 => {
                            if let (Value::Number(a), Value::Number(b)) = (&args[0], &args[1]) {
                                (*a as i64, *b as i64)
                            } else {
                                return Err("range() requires numbers".to_string());
                            }
                        }
                        _ => return Err("range() takes 1 or 2 arguments".to_string()),
                    };
                    
                    let values: Vec<Value> = (start..end)
                        .map(|i| Value::Number(i as f64))
                        .collect();
                    Ok(Value::Array(vm.heap.alloc_array(values)))
                },
            ),
            
            // assert(cond, msg?)
            NativeFn::new(
                "assert",
                None,
                |_vm, args| {
                    if args.is_empty() {
                        return Err("assert() requires at least one argument".to_string());
                    }
                    if !args[0].is_truthy() {
                        let msg = args.get(1)
                            .map(|v| format!("{}", v))
                            .unwrap_or_else(|| "assertion failed".to_string());
                        return Err(msg);
                    }
                    Ok(Value::Nil)
                },
            ),
            
            // === Math functions ===
            
            // abs(n)
            NativeFn::new(
                "abs",
                Some(1),
                |_vm, args| {
                    match &args[0] {
                        Value::Number(n) => Ok(Value::Number(n.abs())),
                        _ => Err("abs() requires a number".to_string()),
                    }
                },
            ),
            
            // min(a, b)
            NativeFn::new(
                "min",
                Some(2),
                |_vm, args| {
                    match (&args[0], &args[1]) {
                        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a.min(*b))),
                        _ => Err("min() requires two numbers".to_string()),
                    }
                },
            ),
            
            // max(a, b)
            NativeFn::new(
                "max",
                Some(2),
                |_vm, args| {
                    match (&args[0], &args[1]) {
                        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a.max(*b))),
                        _ => Err("max() requires two numbers".to_string()),
                    }
                },
            ),
            
            // floor(n)
            NativeFn::new(
                "floor",
                Some(1),
                |_vm, args| {
                    match &args[0] {
                        Value::Number(n) => Ok(Value::Number(n.floor())),
                        _ => Err("floor() requires a number".to_string()),
                    }
                },
            ),
            
            // ceil(n)
            NativeFn::new(
                "ceil",
                Some(1),
                |_vm, args| {
                    match &args[0] {
                        Value::Number(n) => Ok(Value::Number(n.ceil())),
                        _ => Err("ceil() requires a number".to_string()),
                    }
                },
            ),
            
            // round(n)
            NativeFn::new(
                "round",
                Some(1),
                |_vm, args| {
                    match &args[0] {
                        Value::Number(n) => Ok(Value::Number(n.round())),
                        _ => Err("round() requires a number".to_string()),
                    }
                },
            ),
            
            // === Enhanced Causality Functions ===
            
            // causal_graph(var_name, format?) - Export causality as DOT or JSON
            NativeFn::new(
                "causal_graph",
                None,
                |vm, args| {
                    if args.is_empty() || args.len() > 2 {
                        return Err("causal_graph() takes 1 or 2 arguments".to_string());
                    }
                    let var_name = match &args[0] {
                        Value::String(s) => s.clone(),
                        _ => return Err("causal_graph() requires variable name as string".to_string()),
                    };
                    let format = if args.len() > 1 {
                        match &args[1] {
                            Value::String(s) => s.as_str(),
                            _ => return Err("causal_graph() format must be string".to_string()),
                        }
                    } else {
                        "dot"
                    };
                    
                    match format {
                        "dot" => Ok(Value::String(vm.causality.to_dot(&var_name))),
                        "json" => Ok(Value::String(vm.causality.to_json(&var_name))),
                        _ => Err(format!("Unknown format '{}'. Use 'dot' or 'json'", format)),
                    }
                },
            ),
            
            // transitions(var_name) - Get number of state transitions
            NativeFn::new(
                "transitions",
                Some(1),
                |vm, args| {
                    let var_name = match &args[0] {
                        Value::String(s) => s.clone(),
                        _ => return Err("transitions() requires variable name as string".to_string()),
                    };
                    Ok(Value::Number(vm.causality.transition_count(&var_name) as f64))
                },
            ),
            
            // snapshot() - Get current logical time
            NativeFn::new(
                "snapshot",
                Some(0),
                |vm, _args| {
                    Ok(Value::Number(vm.causality.current_time() as f64))
                },
            ),
        ];

        for native in natives {
            let name = native.name.clone();
            self.globals.insert(name, Binding {
                value: Value::NativeFunction(native),
                is_state: false,
            });
        }
    }
    
    /// Run bytecode
    pub fn run(&mut self, chunk: Chunk) -> Result<Value> {
        let chunk = Rc::new(chunk);
        let function = crate::value::Function::new(
            "<script>".to_string(),
            Vec::new(),
            Rc::clone(&chunk),
            0,
        );
        let func_handle = self.heap.alloc_function(function);
        let closure_handle = self.heap.alloc_closure(func_handle, Vec::new());
        
        // Push script closure to stack (slot 0)
        self.stack.push(Value::Closure(closure_handle));
        
        self.frames.push(CallFrame::new(
            closure_handle,
            chunk,
            0,
        ));
        
        self.execute()
    }
    
    /// Register compiled function chunks
    pub fn register_chunks(&mut self, chunks: Vec<Chunk>) {
        for chunk in chunks {
            self.function_chunks.push(Rc::new(chunk));
        }
    }
    
    pub fn collect_garbage(&mut self) {
        // 1. Mark roots
        self.mark_roots();
        
        // 2. Trace references (Blacken)
        self.heap.trace_references();
        
        // 3. Sweep
        self.heap.sweep();
        
        // Prune upvalues that weren't marked (no longer reachable)
        self.open_upvalues.retain(|&handle| self.heap.is_marked(handle));
    }
    
    fn capture_upvalue(&mut self, location: usize) -> crate::gc::Handle {
        // Check if existing open upvalue points to this location
        for &handle in &self.open_upvalues {
             if let Some(upvalue) = self.heap.get_upvalue(handle) {
                 if let crate::gc::UpvalueState::Open(slot) = *upvalue.location.borrow() {
                     if slot == location {
                         return handle;
                     }
                 }
             }
        }
        
        // Create new upvalue
        let handle = self.heap.alloc_upvalue(location);
        self.open_upvalues.push(handle);
        handle
    }
    
    fn close_upvalues(&mut self, last: usize) {
        let mut i = 0;
        while i < self.open_upvalues.len() {
            let handle = self.open_upvalues[i];
            let mut should_close = false;
            
            if let Some(upvalue) = self.heap.get_upvalue(handle) {
                // Check if open and get slot
                let slot = if let crate::gc::UpvalueState::Open(s) = *upvalue.location.borrow() {
                    Some(s)
                } else {
                    None
                };
                
                if let Some(s) = slot {
                    if s >= last {
                        let value = self.stack[s].clone();
                        *upvalue.location.borrow_mut() = crate::gc::UpvalueState::Closed(value);
                        should_close = true;
                    }
                } else {
                    // Already closed, remove from open list
                    should_close = true;
                }
            } else {
                // Invalid handle, remove
                should_close = true;
            }
            
            if should_close {
                self.open_upvalues.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }
    
    fn mark_roots(&mut self) {
        // Stack
        for value in &self.stack {
            for child in value.children() {
                self.heap.mark(child);
            }
        }
        
        // Globals
        for binding in self.globals.values() {
            for child in binding.value.children() {
                self.heap.mark(child);
            }
        }
        
        // Functions (Chunks)
        // We need to trace constants in all chunks because functions might be running
        // or reachable via call frames.
        for chunk in &self.function_chunks {
            for constant in &chunk.constants {
                for child in constant.children() {
                    self.heap.mark(child);
                }
            }
        }
    }
    
    fn execute(&mut self) -> Result<Value> {
        loop {
            if self.frames.is_empty() {
                return Ok(self.stack.pop().unwrap_or(Value::Nil));
            }
            
            let op = self.read_byte();
            let opcode = OpCode::from(op);

            // GC Check
            if self.heap.should_collect() {
                // println!("-- Triggering GC --"); // Debug
                self.collect_garbage();
            }

            
            match opcode {
                OpCode::Constant => {
                    let idx = self.read_u16();
                    let value = self.current_chunk().constants[idx as usize].clone();
                    self.push(value);
                }
                
                OpCode::Nil => self.push(Value::Nil),
                OpCode::True => self.push(Value::Bool(true)),
                OpCode::False => self.push(Value::Bool(false)),
                
                OpCode::Pop => { self.pop(); }
                
                OpCode::Dup => {
                    let val = self.peek(0).clone();
                    self.push(val);
                }
                
                OpCode::DefineGlobal => {
                    let idx = self.read_u16();
                    let name = self.get_name(idx);
                    let value = self.pop();
                    self.globals.insert(name, Binding { value, is_state: false });
                }
                
                OpCode::DefineState => {
                    let idx = self.read_u16();
                    let name = self.get_name(idx);
                    let value = self.pop();
                    self.globals.insert(name, Binding { value, is_state: true });
                }
                
                OpCode::GetGlobal => {
                    let idx = self.read_u16();
                    let name = self.get_name(idx);
                    let value = self.globals.get(&name)
                        .ok_or_else(|| SkyHetuError::new(
                            ErrorKind::UndefinedVariable(name.clone()),
                            None,
                        ))?
                        .value
                        .clone();
                    self.push(value);
                }
                
                OpCode::SetGlobal => {
                    let idx = self.read_u16();
                    let name = self.get_name(idx);
                    let value = self.peek(0).clone();
                    
                    if let Some(binding) = self.globals.get_mut(&name) {
                        if !binding.is_state {
                            return Err(SkyHetuError::new(
                                ErrorKind::ImmutableVariable(name),
                                None,
                            ));
                        }
                        binding.value = value;
                    } else {
                        return Err(SkyHetuError::new(
                            ErrorKind::UndefinedVariable(name),
                            None,
                        ));
                    }
                }
                
                OpCode::Transition => {
                    let idx = self.read_u16();
                    let name = self.get_name(idx);
                    let new_value = self.pop();
                    
                    if let Some(binding) = self.globals.get_mut(&name) {
                        if !binding.is_state {
                            return Err(SkyHetuError::new(
                                ErrorKind::ImmutableVariable(name),
                                None,
                            ));
                        }
                        
                        let old_value = binding.value.clone();
                        
                        // Record causality
                        self.causality.record_mutation(
                            &name,
                            old_value,
                            new_value.clone(),
                            None,
                        );
                        
                        binding.value = new_value;
                    } else {
                        return Err(SkyHetuError::new(
                            ErrorKind::UndefinedVariable(name),
                            None,
                        ));
                    }
                }

                OpCode::TransitionLocal => {
                    let slot = self.read_u16() as usize;
                    let frame_slot = self.current_frame().slot;
                    let stack_idx = frame_slot + slot;
                    let new_value = self.pop();
                    
                    // We don't have the name of local easily for causality log?
                    // We can reconstruct it or pass it?
                    // Passing it would require add_name call in compiler.
                    // For now, let's use "local_state_{slot}" or similar if needed,
                    // BUT CausalityLog expects names.
                    // To do this properly, the compiler should pass the name too.
                    // Or we just update value and log with generic name.
                    // Ideally, TransitionLocal should take (slot, name_idx).
                    // Bytecode: TransitionLocal(slot: u16, name: u16).
                    
                    // Let's modify opcode usage to include name index.
                    let name_idx = self.read_u16();
                    let name = self.get_name(name_idx);
                    
                    let old_value = self.stack[stack_idx].clone();
                    
                    self.causality.record_mutation(
                        &name,
                        old_value,
                        new_value.clone(),
                        None, 
                    );
                    
                    self.stack[stack_idx] = new_value;
                }
                
                OpCode::GetLocal => {
                    let slot = self.read_u16() as usize;
                    let frame_slot = self.current_frame().slot;
                    let value = self.stack[frame_slot + slot].clone();
                    self.push(value);
                }
                
                OpCode::SetLocal => {
                    let slot = self.read_u16() as usize;
                    let frame_slot = self.current_frame().slot;
                    let value = self.peek(0).clone();
                    self.stack[frame_slot + slot] = value;
                }
                
                // Arithmetic
                OpCode::Add => {
                    let b = self.pop();
                    let a = self.pop();
                    
                    match (&a, &b) {
                        (Value::Number(x), Value::Number(y)) => {
                            self.push(Value::Number(x + y));
                        }
                        (Value::String(s1), Value::String(s2)) => {
                            self.push(Value::String(format!("{}{}", s1, s2)));
                        }
                        (Value::String(s), Value::Number(n)) => {
                             self.push(Value::String(format!("{}{}", s, n)));
                        }
                        (Value::Number(n), Value::String(s)) => {
                             self.push(Value::String(format!("{}{}", n, s)));
                        }
                        _ => {
                            return Err(SkyHetuError::new(
                                ErrorKind::TypeMismatch("numbers or strings".to_string(), format!("{} and {}", a.type_name(), b.type_name())),
                                None,
                            ));
                        }
                    }
                }
                
                OpCode::Subtract => self.binary_op(|a, b| a - b, "-")?,
                OpCode::Multiply => self.binary_op(|a, b| a * b, "*")?,
                OpCode::Divide => {
                    let b = self.pop();
                    let a = self.pop();
                    match (&a, &b) {
                        (Value::Number(x), Value::Number(y)) => {
                            if *y == 0.0 {
                                return Err(SkyHetuError::new(ErrorKind::DivisionByZero, None));
                            }
                            self.push(Value::Number(x / y));
                        }
                        _ => {
                            return Err(SkyHetuError::new(
                                ErrorKind::TypeMismatch("numbers".to_string(), format!("{} and {}", a.type_name(), b.type_name())),
                                None,
                            ));
                        }
                    }
                }
                OpCode::Modulo => self.binary_op(|a, b| a % b, "%")?,
                
                OpCode::Negate => {
                    let val = self.pop();
                    match val {
                        Value::Number(n) => self.push(Value::Number(-n)),
                        _ => {
                            return Err(SkyHetuError::new(
                                ErrorKind::TypeMismatch("number".to_string(), val.type_name().to_string()),
                                None,
                            ));
                        }
                    }
                }
                
                // Comparison
                OpCode::Equal => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Value::Bool(a == b));
                }
                
                OpCode::NotEqual => {
                    let b = self.pop();
                    let a = self.pop();
                    self.push(Value::Bool(a != b));
                }
                
                OpCode::Less => self.comparison_op(|a, b| a < b)?,
                OpCode::LessEqual => self.comparison_op(|a, b| a <= b)?,
                OpCode::Greater => self.comparison_op(|a, b| a > b)?,
                OpCode::GreaterEqual => self.comparison_op(|a, b| a >= b)?,
                
                OpCode::Not => {
                    let val = self.pop();
                    self.push(Value::Bool(!val.is_truthy()));
                }
                
                // Control flow
                OpCode::Jump => {
                    let offset = self.read_u16() as usize;
                    let current_ip = self.current_frame().ip;
                    self.current_frame_mut().ip = current_ip + offset;
                }
                
                OpCode::JumpIfFalse => {
                    let offset = self.read_u16() as usize;
                    if !self.peek(0).is_truthy() {
                        let current_ip = self.current_frame().ip;
                        self.current_frame_mut().ip = current_ip + offset;
                    }
                }
                
                OpCode::JumpIfTrue => {
                    let offset = self.read_u16() as usize;
                    if self.peek(0).is_truthy() {
                        let current_ip = self.current_frame().ip;
                        self.current_frame_mut().ip = current_ip + offset;
                    }
                }
                
                OpCode::Loop => {
                    let offset = self.read_u16() as usize;
                    let current_ip = self.current_frame().ip;
                    self.current_frame_mut().ip = current_ip - offset;
                }
                
                // Functions
                OpCode::Call => {
                    let arg_count = self.read_byte() as usize;
                    let callee = self.peek(arg_count).clone();
                    self.call_value(callee, arg_count)?;
                }
                
                OpCode::Return => {
                    let result = self.pop();
                    let frame = self.frames.pop().unwrap();
                    
                    // Close upvalues for the frame being popped
                    self.close_upvalues(frame.slot);
                    
                    if self.frames.is_empty() {
                        self.push(result);
                        return Ok(self.pop());
                    }
                    
                    // Pop arguments and function
                    self.stack.truncate(frame.slot);
                    self.push(result);
                }
                
                OpCode::Closure => {
                    let idx = self.read_u16();
                    let func_const = self.current_chunk().constants[idx as usize].clone();
                    
                    if let Value::Function(func_handle) = func_const {
                        let func = self.heap.get_function(func_handle).unwrap(); // Should exist
                        let upvalue_count = func.upvalue_count;
                        
                        let mut upvalues = Vec::with_capacity(upvalue_count);
                        
                        for _ in 0..upvalue_count {
                            let is_local = self.read_byte() != 0;
                            let index = self.read_byte() as usize;
                            
                            if is_local {
                                let frame_slot = self.current_frame().slot;
                                let location = frame_slot + index;
                                let upvalue = self.capture_upvalue(location);
                                upvalues.push(upvalue);
                            } else {
                                // Capture from enclosing closure
                                let current_closure_handle = self.current_frame().closure;
                                let current_closure = self.heap.get_closure(current_closure_handle).expect("Closure missing");
                                let upvalue = current_closure.upvalues[index];
                                upvalues.push(upvalue);
                            }
                        }
                        
                        let closure_handle = self.heap.alloc_closure(func_handle, upvalues);
                        self.push(Value::Closure(closure_handle));
                        
                    } else {
                        return Err(SkyHetuError::new(ErrorKind::RuntimeError("Closure operand must be a function".to_string()), None));
                    }
                }
                
                // Built-ins
                OpCode::Print => {
                    let count = self.read_byte() as usize;
                    let mut output = Vec::new();
                    for _ in 0..count {
                        output.push(format!("{}", self.pop()));
                    }
                    output.reverse();
                    println!("{}", output.join(" "));
                    self.push(Value::Nil);
                }
                
                OpCode::Why => {
                    let idx = self.read_u16();
                    let name = self.get_name(idx);
                    let result = self.causality.why(&name);
                    self.push(Value::String(result));
                }
                
                OpCode::Time => {
                    let time = self.causality.current_time() as f64;
                    self.push(Value::Number(time));
                }
                
                OpCode::Array => {
                    let count = self.read_byte() as usize;
                    let mut elements = Vec::new();
                    for _ in 0..count {
                        elements.push(self.pop());
                    }
                    elements.reverse();
                    let handle = self.heap.alloc_array(elements);
                    self.push(Value::Array(handle));
                }
                
                OpCode::Index => {
                    let index = self.pop();
                    let array = self.pop();
                    
                    match (&array, &index) {
                        (Value::Array(handle), Value::Number(i)) => {
                            if let Some(arr) = self.heap.get_array(*handle) {
                                let idx = *i as usize;
                                let val = arr.get(idx).cloned().unwrap_or(Value::Nil);
                                self.push(val);
                            } else {
                                // Array not found
                                self.push(Value::Nil);
                            }
                        }
                        (Value::String(s), Value::Number(i)) => {
                            let idx = *i as usize;
                            let val = s.chars().nth(idx)
                                .map(|c| Value::String(c.to_string()))
                                .unwrap_or(Value::Nil);
                            self.push(val);
                        }
                        _ => {
                            return Err(SkyHetuError::new(
                                ErrorKind::TypeMismatch("array or string".to_string(), array.type_name().to_string()),
                                None,
                            ));
                        }
                    }
                }
                
                OpCode::Break | OpCode::Continue => {
                    // These should be compiled to jumps
                    unreachable!("Break/Continue should be compiled to jumps");
                }
                
                OpCode::Halt => {
                    return Ok(self.stack.pop().unwrap_or(Value::Nil));
                }
                
                OpCode::GetUpvalue => {
                    let idx = self.read_u16();
                    let closure_handle = self.current_frame().closure;
                    let closure = self.heap.get_closure(closure_handle).expect("Closure missing");
                    let upvalue_handle = closure.upvalues[idx as usize];
                    
                    let value = if let Some(upvalue) = self.heap.get_upvalue(upvalue_handle) {
                        match &*upvalue.location.borrow() {
                            crate::gc::UpvalueState::Open(slot) => self.stack[*slot].clone(),
                            crate::gc::UpvalueState::Closed(val) => val.clone(),
                        }
                    } else {
                        Value::Nil
                    };
                    self.push(value);
                }

                OpCode::SetUpvalue => {
                    let idx = self.read_u16();
                    let closure_handle = self.current_frame().closure;
                    let closure = self.heap.get_closure(closure_handle).expect("Closure missing");
                    let upvalue_handle = closure.upvalues[idx as usize];
                    let value = self.peek(0).clone();
                    
                    if let Some(upvalue) = self.heap.get_upvalue(upvalue_handle) {
                        let mut location = upvalue.location.borrow_mut();
                        match *location {
                            crate::gc::UpvalueState::Open(slot) => {
                                self.stack[slot] = value;
                            }
                            crate::gc::UpvalueState::Closed(ref mut val) => {
                                *val = value;
                            }
                        }
                    }
                }
                
                OpCode::TransitionUpvalue => {
                    let slot = self.read_u16() as usize;
                    let name_idx = self.read_u16();
                    let name = self.get_name(name_idx);
                    let new_value = self.pop();
                    
                    let closure_handle = self.current_frame().closure;
                    let closure = self.heap.get_closure(closure_handle).expect("Closure missing");
                    let upvalue_handle = closure.upvalues[slot];
                    
                    let old_value = if let Some(upvalue) = self.heap.get_upvalue(upvalue_handle) {
                         match &*upvalue.location.borrow() {
                            crate::gc::UpvalueState::Open(s) => self.stack[*s].clone(),
                            crate::gc::UpvalueState::Closed(val) => val.clone(),
                        }
                    } else { Value::Nil };
                    
                    self.causality.record_mutation(
                        &name,
                        old_value,
                        new_value.clone(),
                        None,
                    );
                    
                    if let Some(upvalue) = self.heap.get_upvalue(upvalue_handle) {
                        let mut location = upvalue.location.borrow_mut();
                        match *location {
                            crate::gc::UpvalueState::Open(s) => {
                                self.stack[s] = new_value;
                            }
                            crate::gc::UpvalueState::Closed(ref mut val) => {
                                *val = new_value;
                            }
                        }
                    }
                }
                
                OpCode::CloseUpvalue => {
                    self.close_upvalues(self.stack.len() - 1);
                    self.pop();
                }
                
                // --- Classes & Instances ---
                
                OpCode::Class => {
                    let idx = self.read_u16();
                    let name = self.get_name(idx);
                    let handle = self.heap.alloc_class(name);
                    self.push(Value::Class(handle));
                }
                
                OpCode::Method => {
                    let idx = self.read_u16();
                    let name = self.get_name(idx);
                    let method_val = self.peek(0).clone();
                    let class_val = self.peek(1).clone();
                    
                    if let Value::Class(class_handle) = class_val {
                        if let Value::Closure(method_handle) = method_val {
                            if let Some(class) = self.heap.get_class_mut(class_handle) {
                                class.methods.insert(name, method_handle);
                            }
                        } else {
                             return Err(SkyHetuError::new(ErrorKind::RuntimeError("Method must be a closure".to_string()), None));
                        }
                    } else {
                        return Err(SkyHetuError::new(ErrorKind::RuntimeError("Cannot define method on non-class".to_string()), None));
                    }
                    self.pop(); // Pop method closure
                }
                
                OpCode::GetProperty => {
                    let idx = self.read_u16();
                    let name = self.get_name(idx);
                    let receiver = self.peek(0).clone();
                    
                    if let Value::Instance(handle) = receiver {
                        // 1. Try Fields
                        let field_val = {
                             let instance = self.heap.get_instance(handle).unwrap();
                             instance.fields.borrow().get(&name).cloned()
                        };
                        
                        if let Some(val) = field_val {
                            self.pop(); // Instance
                            self.push(val);
                        } else {
                            // 2. Try Methods
                            let method_handle = {
                                let instance = self.heap.get_instance(handle).unwrap();
                                let class_handle = instance.class;
                                let class = self.heap.get_class(class_handle).unwrap();
                                class.methods.get(&name).cloned()
                            };
                            
                            if let Some(handle) = method_handle {
                                let bound = self.heap.alloc_bound_method(receiver, handle);
                                self.pop(); // Instance
                                self.push(Value::BoundMethod(bound));
                            } else {
                                return Err(SkyHetuError::new(ErrorKind::UndefinedProperty(name), None));
                            }
                        }
                    } else {
                         return Err(SkyHetuError::new(ErrorKind::RuntimeError("Only instances have properties.".to_string()), None));
                    }
                }
                
                OpCode::SetProperty => {
                    let idx = self.read_u16();
                    let name = self.get_name(idx);
                    let value = self.pop();
                    let receiver = self.peek(0).clone();
                    
                    if let Value::Instance(handle) = receiver {
                        {
                            let instance = self.heap.get_instance(handle).unwrap();
                            instance.fields.borrow_mut().insert(name, value.clone());
                        } // Drop instance borrow
                        
                        self.pop(); // Pop Instance
                        self.push(value); // Push Value (result)
                    } else {
                         return Err(SkyHetuError::new(ErrorKind::RuntimeError("Only instances have properties.".to_string()), None));
                    }
                }
            }
        }
    }
    
    /// Call a value
    fn call_value(&mut self, callee: Value, arg_count: usize) -> Result<()> {
        match callee {
            Value::Function(func_handle) => {
                // Wrap raw function in closure
                let closure_handle = self.heap.alloc_closure(func_handle, Vec::new());
                self.call_function(closure_handle, arg_count)
            }
            Value::Closure(handle) => {
                 self.call_function(handle, arg_count)
            }
            Value::NativeFunction(native) => {
                let args_start = self.stack.len() - arg_count;
                let args = &self.stack[args_start..];
                
                // Check arity
                if let Some(arity) = native.arity {
                    if arg_count != arity {
                        return Err(SkyHetuError::new(
                            ErrorKind::WrongArity(arity, arg_count),
                            None,
                        ));
                    }
                }
                
                // Clone args to satisfy borrow checker when calling native func which needs &mut self
                let args_vec = args.to_vec();
                
                // Call native function
                let result = (native.func)(self, &args_vec).map_err(|msg| SkyHetuError::new(
                    ErrorKind::RuntimeError(msg),
                    None,
                ))?;
                
                // Pop args + function
                self.stack.truncate(args_start - 1);
                self.push(result);
                Ok(())
            }
            Value::Class(handle) => {
                 let instance_handle = self.heap.alloc_instance(handle);
                 let instance_val = Value::Instance(instance_handle);
                 
                 // Look for 'init' method
                 let init_handle = {
                     let class = self.heap.get_class(handle).unwrap();
                     class.methods.get("init").cloned()
                 };
                 
                 if let Some(handle) = init_handle {
                     // Replace Class with Instance on stack (at stack.len() - 1 - arg_count)
                     let idx = self.stack.len() - 1 - arg_count;
                     self.stack[idx] = instance_val;
                     
                     // Call init closure
                     // Note: methods in class.methods ARE closures (Handle to Closure)
                     // So we just call it directly
                     self.call_function(handle, arg_count)
                 } else if arg_count != 0 {
                      return Err(SkyHetuError::new(ErrorKind::WrongArity(0, arg_count), None));
                 } else {
                     // No init, valid if 0 args.
                     let _idx = self.stack.len() - 1; // Class is here
                     self.pop(); // Pop Class
                     self.push(instance_val);
                     Ok(())
                 }
            }
            Value::BoundMethod(handle) => {
                let bound = self.heap.get_bound_method(handle).unwrap().clone();
                // Set 'this' (receiver) at stack slot 0 of call (stack.len - 1 - arg_count)
                let idx = self.stack.len() - 1 - arg_count;
                self.stack[idx] = bound.receiver;
                
                self.call_function(bound.method, arg_count)
            }
            _ => Err(SkyHetuError::new(
                ErrorKind::TypeMismatch("function".to_string(), callee.type_name().to_string()),
                None,
            )),
        }
    }
    
    /// Call a user-defined function
    /// Call a closure
    fn call_function(&mut self, closure_handle: crate::gc::Handle, arg_count: usize) -> Result<()> {
        // Get function from closure
        let func_handle = if let Some(closure) = self.heap.get_closure(closure_handle) {
            closure.function
        } else {
             return Err(SkyHetuError::new(ErrorKind::RuntimeError("Called value is not a closure".to_string()), None));
        };

        let func = self.heap.get_function(func_handle)
            .ok_or_else(|| SkyHetuError::new(ErrorKind::RuntimeError("Function not found".to_string()), None))?;
            
        if arg_count != func.params.len() {
            return Err(SkyHetuError::new(
                ErrorKind::WrongArity(func.params.len(), arg_count),
                None,
            ));
        }
        
        if self.frames.len() >= FRAMES_MAX {
            return Err(SkyHetuError::new(ErrorKind::StackOverflow, None));
        }
        
        let chunk = func.chunk.clone();
        
        self.frames.push(CallFrame {
            closure: closure_handle,
            chunk,
            ip: 0,
            slot: self.stack.len() - arg_count - 1,
        });
        
        Ok(())
    }

    
    fn binary_op<F>(&mut self, op: F, op_name: &str) -> Result<()>
    where
        F: Fn(f64, f64) -> f64,
    {
        let b = self.pop();
        let a = self.pop();
        
        match (&a, &b) {
            (Value::Number(x), Value::Number(y)) => {
                self.push(Value::Number(op(*x, *y)));
                Ok(())
            }
            (Value::String(s1), Value::String(s2)) if op_name == "+" => {
                self.push(Value::String(format!("{}{}", s1, s2)));
                Ok(())
            }
            (Value::String(s), Value::Number(n)) if op_name == "*" => {
                self.push(Value::String(s.repeat(*n as usize)));
                Ok(())
            }
            _ => Err(SkyHetuError::new(
                ErrorKind::TypeMismatch(
                    "numbers".to_string(),
                    format!("{} and {}", a.type_name(), b.type_name()),
                ),
                None,
            )),
        }
    }
    
    fn comparison_op<F>(&mut self, op: F) -> Result<()>
    where
        F: Fn(f64, f64) -> bool,
    {
        let b = self.pop();
        let a = self.pop();
        
        match (&a, &b) {
            (Value::Number(x), Value::Number(y)) => {
                self.push(Value::Bool(op(*x, *y)));
                Ok(())
            }
            _ => Err(SkyHetuError::new(
                ErrorKind::TypeMismatch(
                    "numbers".to_string(),
                    format!("{} and {}", a.type_name(), b.type_name()),
                ),
                None,
            )),
        }
    }
    
    // ==================== Helpers ====================
    
    fn push(&mut self, value: Value) {
        if self.stack.len() >= STACK_MAX {
            panic!("Stack overflow");
        }
        self.stack.push(value);
    }
    
    fn pop(&mut self) -> Value {
        self.stack.pop().expect("Stack underflow")
    }
    
    fn peek(&self, distance: usize) -> &Value {
        &self.stack[self.stack.len() - 1 - distance]
    }
    
    fn read_byte(&mut self) -> u8 {
        let frame = self.frames.last_mut().unwrap();
        let byte = frame.chunk.code[frame.ip];
        frame.ip += 1;
        byte
    }
    
    fn read_u16(&mut self) -> u16 {
        let frame = self.frames.last_mut().unwrap();
        let value = frame.chunk.read_u16(frame.ip);
        frame.ip += 2;
        value
    }
    
    fn current_chunk(&self) -> &Chunk {
        &self.frames.last().unwrap().chunk
    }
    
    fn current_frame(&self) -> &CallFrame {
        self.frames.last().unwrap()
    }
    
    fn current_frame_mut(&mut self) -> &mut CallFrame {
        self.frames.last_mut().unwrap()
    }
    
    fn get_name(&self, idx: u16) -> String {
        self.current_chunk().names[idx as usize].clone()
    }
    
    pub fn why(&self, variable: &str) -> String {
        self.causality.why(variable)
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    use crate::parser::Parser;
    use crate::compiler::Compiler;
    
    fn run_vm(source: &str) -> Value {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let mut vm = VM::new();
        let mut compiler = Compiler::new();
        let (chunk, _) = compiler.compile(&program, &mut vm.heap).unwrap();
        
        vm.run(chunk).unwrap()
    }
    
    #[test]
    fn test_vm_arithmetic() {
        // Use state to capture results
        assert_eq!(run_vm("state r = 1 + 2\nr"), Value::Number(3.0));
        assert_eq!(run_vm("state r = 10 - 3\nr"), Value::Number(7.0));
        assert_eq!(run_vm("state r = 4 * 5\nr"), Value::Number(20.0));
        assert_eq!(run_vm("state r = 20 / 4\nr"), Value::Number(5.0));
    }
    
    #[test]
    fn test_vm_comparison() {
        assert_eq!(run_vm("state r = 1 < 2\nr"), Value::Bool(true));
        assert_eq!(run_vm("state r = 5 > 3\nr"), Value::Bool(true));
        assert_eq!(run_vm("state r = 2 == 2\nr"), Value::Bool(true));
        assert_eq!(run_vm("state r = 1 != 2\nr"), Value::Bool(true));
    }
    
    #[test]
    fn test_vm_variables() {
        assert_eq!(run_vm("let x = 42\nstate r = x\nr"), Value::Number(42.0));
    }
    
    #[test]
    fn test_vm_state_transition() {
        let result = run_vm(r#"
            state counter = 0
            counter -> counter + 1
            counter -> counter + 1
            counter
        "#);
        assert_eq!(result, Value::Number(2.0));
    }
    
    #[test]
    fn test_vm_if() {
        let result = run_vm(r#"
            let x = 10
            state result = 0
            if x > 5 {
                result -> 1
            }
            result
        "#);
        assert_eq!(result, Value::Number(1.0));
    }
    
    #[test]
    fn test_vm_while() {
        let result = run_vm(r#"
            state sum = 0
            state i = 1
            while i <= 5 {
                sum -> sum + i
                i -> i + 1
            }
            sum
        "#);
        assert_eq!(result, Value::Number(15.0));
    }
    
    #[test]
    fn test_vm_causality() {
        let source = r#"
            state x = 0
            x -> x + 10
            x -> x + 5
            x
        "#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = crate::parser::Parser::new(tokens);
        let program = parser.parse().unwrap();
        
        let mut vm = VM::new();
        let mut compiler = crate::compiler::Compiler::new();
        let (chunk, _) = compiler.compile(&program, &mut vm.heap).unwrap();
        
        let result = vm.run(chunk).unwrap();
        
        // Check that causality was recorded
        let history = vm.causality.history("x");
        assert_eq!(history.len(), 2);
    }
}

