//! Abstract Syntax Tree definitions for SkyHetu
//!
//! Represents the structure of programs after parsing.

use crate::token::Span;

/// A unique identifier for AST nodes (used for causality tracking)
pub type NodeId = usize;

/// Expression nodes
#[derive(Debug, Clone)]
pub enum Expr {
    /// Number literal: 42, 3.14
    Number { value: f64, span: Span },
    
    /// String literal: "hello"
    String { value: String, span: Span },
    
    /// Boolean literal: true, false
    Bool { value: bool, span: Span },
    
    /// Nil literal
    Nil { span: Span },
    
    /// Variable reference: foo
    Ident { name: String, span: Span },
    
    /// Binary operation: a + b, x * y
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    
    /// Unary operation: -x, !y
    Unary {
        op: UnaryOp,
        operand: Box<Expr>,
        span: Span,
    },
    
    /// Function call: foo(a, b)
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    
    /// Grouping: (expr)
    Grouping { expr: Box<Expr>, span: Span },
    
    /// Logical and/or: a and b, x or y
    Logical {
        left: Box<Expr>,
        op: LogicalOp,
        right: Box<Expr>,
        span: Span,
    },
    
    /// Lambda/anonymous function: |a, b| a + b
    Lambda {
        params: Vec<String>,
        body: Box<Expr>,
        span: Span,
    },

    /// Property access: obj.prop
    Get {
        object: Box<Expr>,
        name: String,
        span: Span,
    },

    /// Property assignment: obj.prop = value
    Set {
        object: Box<Expr>,
        name: String,
        value: Box<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Number { span, .. } => *span,
            Expr::String { span, .. } => *span,
            Expr::Bool { span, .. } => *span,
            Expr::Nil { span } => *span,
            Expr::Ident { span, .. } => *span,
            Expr::Binary { span, .. } => *span,
            Expr::Unary { span, .. } => *span,
            Expr::Call { span, .. } => *span,
            Expr::Grouping { span, .. } => *span,
            Expr::Logical { span, .. } => *span,
            Expr::Lambda { span, .. } => *span,
            Expr::Get { span, .. } => *span,
            Expr::Set { span, .. } => *span,
        }
    }
}

/// Binary operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BinaryOp {
    Add,      // +
    Sub,      // -
    Mul,      // *
    Div,      // /
    Mod,      // %
    Eq,       // ==
    Ne,       // !=
    Lt,       // <
    Le,       // <=
    Gt,       // >
    Ge,       // >=
}

impl std::fmt::Display for BinaryOp {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BinaryOp::Add => write!(f, "+"),
            BinaryOp::Sub => write!(f, "-"),
            BinaryOp::Mul => write!(f, "*"),
            BinaryOp::Div => write!(f, "/"),
            BinaryOp::Mod => write!(f, "%"),
            BinaryOp::Eq => write!(f, "=="),
            BinaryOp::Ne => write!(f, "!="),
            BinaryOp::Lt => write!(f, "<"),
            BinaryOp::Le => write!(f, "<="),
            BinaryOp::Gt => write!(f, ">"),
            BinaryOp::Ge => write!(f, ">="),
        }
    }
}

/// Unary operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UnaryOp {
    Neg,  // -
    Not,  // !
}

/// Logical operators
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LogicalOp {
    And,
    Or,
}

/// Statement nodes
#[derive(Debug, Clone)]
pub enum Stmt {
    /// Expression statement
    Expr { expr: Expr },
    
    /// Immutable binding: let x = expr
    Let {
        name: String,
        value: Expr,
        span: Span,
    },
    
    /// Mutable state: state x = expr
    State {
        name: String,
        value: Expr,
        span: Span,
    },
    
    /// State transition: x -> expr
    Transition {
        name: String,
        value: Expr,
        span: Span,
    },
    
    /// Block: { stmt* }
    Block { stmts: Vec<Stmt>, span: Span },
    
    /// If statement: if cond { } else { }
    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
        span: Span,
    },
    
    /// While loop: while cond { }
    While {
        condition: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    
    /// For loop: for x in iter { }
    For {
        var: String,
        iterable: Expr,
        body: Box<Stmt>,
        span: Span,
    },
    
    /// Function definition: fn name(params) { }
    Function {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
        span: Span,
    },
    
    /// Return statement: return expr
    Return { value: Option<Expr>, span: Span },
    
    /// Break statement
    Break { span: Span },
    
    /// Continue statement  
    Continue { span: Span },
    
    /// Class definition
    Class {
        name: String,
        methods: Vec<Stmt>,
        span: Span,
    },
    
    /// Import declaration: import { a, b } from "module"
    Import {
        names: Vec<String>,
        path: String,
        span: Span,
    },
    
    /// Export declaration: export fn foo() { } or export let x = 1
    Export {
        stmt: Box<Stmt>,
        span: Span,
    },
}

/// A complete program
#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

impl Program {
    pub fn new(statements: Vec<Stmt>) -> Self {
        Self { statements }
    }
}
