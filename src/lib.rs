//! SkyHetu - A causality-first programming language
//!
//! SkyHetu makes state, time, and causality explicit by default.

pub mod token;
pub mod lexer;
pub mod parser;
pub mod ast;
pub mod value;
pub mod environment;
// pub mod interpreter;
pub mod causality;
pub mod gc;
pub mod error;
pub mod bytecode;
pub mod compiler;
pub mod vm;

pub use error::{Result, SkyHetuError};
// pub use interpreter::Interpreter;
pub use lexer::Lexer;
pub use parser::Parser;
pub use value::Value;

/// Convenience function to run SkyHetu code
pub fn run(source: &str) -> Result<Value> {
    let mut lexer = Lexer::new(source);
    let tokens = lexer.tokenize()?;
    let mut parser = Parser::new(tokens);
    let program = parser.parse()?;
    
    // Create VM first to access Heap
    let mut vm = vm::VM::new();
    
    // Compile to bytecode
    let mut compiler = compiler::Compiler::new();
    let (chunk, chunks) = compiler.compile(&program, &mut vm.heap)?;
    
    // Run on VM
    vm.register_chunks(chunks);
    vm.run(chunk)
}

/// Version of the SkyHetu language
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
