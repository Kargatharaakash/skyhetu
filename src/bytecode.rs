//! Bytecode instructions for SkyHetu VM
//!
//! A stack-based virtual machine with causality tracking.

use std::fmt;

/// Opcodes for the VM
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OpCode {
    // Constants and literals
    Constant,       // Push constant from pool
    Nil,            // Push nil
    True,           // Push true
    False,          // Push false
    
    // Stack manipulation
    Pop,            // Pop top of stack
    Dup,            // Duplicate top of stack
    
    // Variables
    DefineGlobal,   // Define global variable (constant index)
    GetGlobal,      // Get global variable
    SetGlobal,      // Set global variable
    DefineState,    // Define mutable state
    GetLocal,       // Get local variable (stack offset)
    SetLocal,       // Set local variable
    TransitionLocal, // Transition a local state variable

    
    // State transitions (causality tracked)
    Transition,     // State transition: var -> value
    
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Negate,         // Unary -
    
    // Comparison
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    
    // Logical
    Not,            // Unary !
    
    // Control flow
    Jump,           // Unconditional jump
    JumpIfFalse,    // Jump if top of stack is falsy
    JumpIfTrue,     // Jump if top of stack is truthy
    Loop,           // Jump backwards
    
    // Functions
    Call,           // Call function (arg count)
    Return,         // Return from function
    Closure,        // Create closure
    
    GetUpvalue,     // Get upvalue (index)
    SetUpvalue,     // Set upvalue (index)
    TransitionUpvalue, // Transition upvalue (index, name)
    CloseUpvalue,   // Close upvalue (hoist)
    
    // Built-ins
    Print,          // Print (arg count)
    Why,            // Query causality
    Time,           // Get logical time
    
    // Loops
    Break,          // Break from loop
    Continue,       // Continue loop
    
    // Arrays
    Array,          // Create array (element count)
    Index,          // Array indexing
    
    // Classes and Instances
    Class,          // Create class (name index)
    Method,         // Define method (name index)
    GetProperty,    // Get property (name index)
    SetProperty,    // Set property (name index)

    // Misc
    Halt,           // Stop execution
}

impl From<u8> for OpCode {
    fn from(byte: u8) -> Self {
        // Safety: we control all writes to bytecode
        unsafe { std::mem::transmute(byte) }
    }
}

impl From<OpCode> for u8 {
    fn from(op: OpCode) -> Self {
        op as u8
    }
}

/// A chunk of bytecode with associated data
#[derive(Debug, Clone)]
pub struct Chunk {
    /// The bytecode instructions
    pub code: Vec<u8>,
    
    /// Constant pool
    pub constants: Vec<crate::value::Value>,
    
    /// Line numbers for each instruction (for error reporting)
    pub lines: Vec<usize>,
    
    /// Variable names (for debugging and causality)
    pub names: Vec<String>,
}

impl Chunk {
    pub fn new() -> Self {
        Self {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
            names: Vec::new(),
        }
    }
    
    /// Write an opcode to the chunk
    pub fn write(&mut self, op: OpCode, line: usize) {
        self.code.push(op as u8);
        self.lines.push(line);
    }
    
    /// Write a raw byte (operand)
    pub fn write_byte(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }
    
    /// Write a 16-bit operand
    pub fn write_u16(&mut self, value: u16, line: usize) {
        self.write_byte((value >> 8) as u8, line);
        self.write_byte(value as u8, line);
    }
    
    /// Add a constant and return its index
    pub fn add_constant(&mut self, value: crate::value::Value) -> u16 {
        self.constants.push(value);
        (self.constants.len() - 1) as u16
    }
    
    /// Add a name and return its index
    pub fn add_name(&mut self, name: String) -> u16 {
        // Check if name already exists
        if let Some(idx) = self.names.iter().position(|n| n == &name) {
            return idx as u16;
        }
        self.names.push(name);
        (self.names.len() - 1) as u16
    }
    
    /// Read a 16-bit value at offset
    pub fn read_u16(&self, offset: usize) -> u16 {
        ((self.code[offset] as u16) << 8) | (self.code[offset + 1] as u16)
    }
    
    /// Get current code length (for jump patching)
    pub fn len(&self) -> usize {
        self.code.len()
    }
    
    /// Patch a jump instruction at offset
    pub fn patch_jump(&mut self, offset: usize) {
        let jump = self.code.len() - offset - 2;
        if jump > u16::MAX as usize {
            panic!("Jump too large");
        }
        self.code[offset] = (jump >> 8) as u8;
        self.code[offset + 1] = jump as u8;
    }
    
    /// Disassemble for debugging
    pub fn disassemble(&self, name: &str) -> String {
        let mut result = format!("== {} ==\n", name);
        let mut offset = 0;
        
        while offset < self.code.len() {
            let (s, new_offset) = self.disassemble_instruction(offset);
            result.push_str(&s);
            result.push('\n');
            offset = new_offset;
        }
        
        result
    }
    
    fn disassemble_instruction(&self, offset: usize) -> (String, usize) {
        let op = OpCode::from(self.code[offset]);
        let line = self.lines.get(offset).copied().unwrap_or(0);
        
        let (instr, new_offset) = match op {
            OpCode::Constant => {
                let idx = self.read_u16(offset + 1);
                let val = &self.constants[idx as usize];
                (format!("CONSTANT {:04} '{}'", idx, val), offset + 3)
            }
            OpCode::DefineGlobal | OpCode::GetGlobal | OpCode::SetGlobal | 
            OpCode::DefineState | OpCode::Transition |
            OpCode::Class | OpCode::Method | OpCode::GetProperty | OpCode::SetProperty => {
                let idx = self.read_u16(offset + 1);
                let name = &self.names[idx as usize];
                (format!("{:?} {:04} '{}'", op, idx, name), offset + 3)
            }
            OpCode::GetLocal | OpCode::SetLocal => {
                let slot = self.read_u16(offset + 1);
                (format!("{:?} {:04}", op, slot), offset + 3)
            }
            OpCode::TransitionLocal => {
                let slot = self.read_u16(offset + 1);
                let name_idx = self.read_u16(offset + 3);
                let name = &self.names[name_idx as usize];
                (format!("{:?} slot:{} name:'{}'", op, slot, name), offset + 5)
            }

            OpCode::Jump | OpCode::JumpIfFalse | OpCode::JumpIfTrue => {
                let jump = self.read_u16(offset + 1);
                (format!("{:?} -> {:04}", op, offset + 3 + jump as usize), offset + 3)
            }
            OpCode::Loop => {
                let jump = self.read_u16(offset + 1);
                (format!("{:?} -> {:04}", op, offset + 3 - jump as usize), offset + 3)
            }
            OpCode::Call | OpCode::Print | OpCode::Array => {
                let count = self.code[offset + 1];
                (format!("{:?} ({})", op, count), offset + 2)
            }
            OpCode::Closure => {
                let idx = self.read_u16(offset + 1);
                (format!("CLOSURE {:04}", idx), offset + 3)
            }
            OpCode::GetUpvalue | OpCode::SetUpvalue => {
                let slot = self.read_u16(offset + 1);
                (format!("{:?} {:04}", op, slot), offset + 3)
            }
            OpCode::TransitionUpvalue => {
                let slot = self.read_u16(offset + 1);
                let name_idx = self.read_u16(offset + 3);
                let name = &self.names[name_idx as usize];
                (format!("{:?} idx:{} name:'{}'", op, slot, name), offset + 5)
            }
            OpCode::CloseUpvalue => {
                (format!("{:?}", op), offset + 1)
            }
            _ => (format!("{:?}", op), offset + 1),
        };
        
        (format!("{:04} {:4} {}", offset, line, instr), new_offset)
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Chunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.disassemble("chunk"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Value;
    
    #[test]
    fn test_chunk_write() {
        let mut chunk = Chunk::new();
        chunk.write(OpCode::Constant, 1);
        let idx = chunk.add_constant(Value::Number(42.0));
        chunk.write_u16(idx, 1);
        chunk.write(OpCode::Return, 1);
        
        assert_eq!(chunk.code.len(), 4);
        assert_eq!(chunk.constants.len(), 1);
    }
    
    #[test]
    fn test_disassemble() {
        let mut chunk = Chunk::new();
        chunk.write(OpCode::Constant, 1);
        let idx = chunk.add_constant(Value::Number(1.5));
        chunk.write_u16(idx, 1);
        chunk.write(OpCode::Return, 2);
        
        let disasm = chunk.disassemble("test");
        assert!(disasm.contains("CONSTANT"));
        assert!(disasm.contains("1.5"));
    }
}