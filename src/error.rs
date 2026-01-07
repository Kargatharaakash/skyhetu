//! Error types for SkyHetu language
//!
//! Provides structured error handling with source locations.

use crate::token::Span;
use std::fmt;

/// Error kinds in SkyHetu
#[derive(Debug, Clone)]
pub enum ErrorKind {
    // Lexer errors
    UnexpectedCharacter(char),
    UnterminatedString,
    InvalidNumber(String),
    
    // Parser errors
    UnexpectedToken(String),
    ExpectedToken(String, String),
    ExpectedExpression,
    ExpectedStatement,
    InvalidAssignmentTarget,
    InvalidAssignment,
    
    // Runtime errors
    UndefinedVariable(String),
    UndefinedProperty(String),
    TypeMismatch(String, String),
    DivisionByZero,
    NotCallable,
    WrongArity(usize, usize),
    ImmutableVariable(String),
    BreakOutsideLoop,
    ContinueOutsideLoop,
    ReturnOutsideFunction,
    StackOverflow,
    
    // Causality errors
    NoStateHistory(String),
    
    // Generic runtime error
    RuntimeError(String),
    
    // Module errors
    ModuleNotFound(String),
}

impl fmt::Display for ErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ErrorKind::UnexpectedCharacter(c) => write!(f, "unexpected character '{}'", c),
            ErrorKind::UnterminatedString => write!(f, "unterminated string"),
            ErrorKind::InvalidNumber(s) => write!(f, "invalid number '{}'", s),
            ErrorKind::UnexpectedToken(t) => write!(f, "unexpected token '{}'", t),
            ErrorKind::ExpectedToken(expected, got) => {
                write!(f, "expected '{}', got '{}'", expected, got)
            }
            ErrorKind::ExpectedExpression => write!(f, "expected expression"),
            ErrorKind::ExpectedStatement => write!(f, "expected statement"),
            ErrorKind::InvalidAssignmentTarget => write!(f, "invalid assignment target"),
            ErrorKind::InvalidAssignment => write!(f, "invalid assignment"),
            ErrorKind::UndefinedVariable(name) => write!(f, "undefined variable '{}'", name),
            ErrorKind::UndefinedProperty(name) => write!(f, "undefined property '{}'", name),
            ErrorKind::TypeMismatch(expected, got) => {
                write!(f, "type mismatch: expected {}, got {}", expected, got)
            }
            ErrorKind::DivisionByZero => write!(f, "division by zero"),
            ErrorKind::NotCallable => write!(f, "value is not callable"),
            ErrorKind::WrongArity(expected, got) => {
                write!(f, "expected {} arguments, got {}", expected, got)
            }
            ErrorKind::ImmutableVariable(name) => {
                write!(f, "cannot mutate immutable variable '{}'", name)
            }
            ErrorKind::BreakOutsideLoop => write!(f, "break outside of loop"),
            ErrorKind::ContinueOutsideLoop => write!(f, "continue outside of loop"),
            ErrorKind::ReturnOutsideFunction => write!(f, "return outside of function"),
            ErrorKind::StackOverflow => write!(f, "stack overflow"),
            ErrorKind::NoStateHistory(name) => {
                write!(f, "no state history for '{}'", name)
            }
            ErrorKind::RuntimeError(msg) => write!(f, "{}", msg),
            ErrorKind::ModuleNotFound(msg) => write!(f, "module not found: {}", msg),
        }
    }
}

/// A SkyHetu error with location information
#[derive(Debug, Clone)]
pub struct SkyHetuError {
    pub kind: ErrorKind,
    pub span: Option<Span>,
    pub source_line: Option<String>,
}

impl SkyHetuError {
    pub fn new(kind: ErrorKind, span: Option<Span>) -> Self {
        Self {
            kind,
            span,
            source_line: None,
        }
    }
    
    pub fn with_source(mut self, source: &str) -> Self {
        if let Some(span) = &self.span {
            let lines: Vec<&str> = source.lines().collect();
            if span.line > 0 && span.line <= lines.len() {
                self.source_line = Some(lines[span.line - 1].to_string());
            }
        }
        self
    }
}

impl fmt::Display for SkyHetuError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(span) = &self.span {
            write!(f, "[line {}:{}] Error: {}", span.line, span.column, self.kind)?;
            
            if let Some(ref line) = self.source_line {
                write!(f, "\n  | {}", line)?;
                write!(f, "\n  | {}^", " ".repeat(span.column.saturating_sub(1)))?;
            }
        } else {
            write!(f, "Error: {}", self.kind)?;
        }
        Ok(())
    }
}

impl std::error::Error for SkyHetuError {}

/// Result type for SkyHetu operations
pub type Result<T> = std::result::Result<T, SkyHetuError>;
