//! Parser for SkyHetu language
//!
//! Converts tokens into an Abstract Syntax Tree.

use crate::ast::{BinaryOp, Expr, LogicalOp, Program, Stmt, UnaryOp};
use crate::error::{ErrorKind, Result, SkyHetuError};
use crate::token::{Span, Token, TokenKind};

/// The parser state
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    /// Create a new parser from tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }
    
    /// Parse the tokens into a program
    pub fn parse(&mut self) -> Result<Program> {
        let mut statements = Vec::new();
        
        while !self.is_at_end() {
            self.skip_newlines();
            if !self.is_at_end() {
                statements.push(self.declaration()?);
            }
        }
        
        Ok(Program::new(statements))
    }
    
    // ==================== Declarations ====================
    
    fn declaration(&mut self) -> Result<Stmt> {
        if self.check(&TokenKind::Let) {
            self.let_declaration()
        } else if self.check(&TokenKind::State) {
            self.state_declaration()
        } else if self.check(&TokenKind::Fn) {
            self.function_declaration()
        } else if self.check(&TokenKind::Class) {
            self.class_declaration()
        } else if self.check(&TokenKind::Import) {
            self.import_declaration()
        } else if self.check(&TokenKind::Export) {
            self.export_declaration()
        } else {
            self.statement()
        }
    }
    
    fn let_declaration(&mut self) -> Result<Stmt> {
        let span = self.advance().span; // consume 'let'
        
        let name = self.expect_ident("expected variable name")?;
        
        self.expect(&TokenKind::Equal, "expected '=' after variable name")?;
        
        let value = self.expression()?;
        self.skip_newlines();
        
        Ok(Stmt::Let { name, value, span })
    }
    
    fn state_declaration(&mut self) -> Result<Stmt> {
        let span = self.advance().span; // consume 'state'
        
        let name = self.expect_ident("expected state variable name")?;
        
        self.expect(&TokenKind::Equal, "expected '=' after state name")?;
        
        let value = self.expression()?;
        self.skip_newlines();
        
        Ok(Stmt::State { name, value, span })
    }
    
    fn function_declaration(&mut self) -> Result<Stmt> {
        let span = self.advance().span; // consume 'fn'
        
        let name = self.expect_ident("expected function name")?;
        
        self.expect(&TokenKind::LeftParen, "expected '(' after function name")?;
        
        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                params.push(self.expect_ident("expected parameter name")?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        
        self.expect(&TokenKind::RightParen, "expected ')' after parameters")?;
        
        self.skip_newlines();
        self.expect(&TokenKind::LeftBrace, "expected '{' before function body")?;
        
        let body = self.block_statements()?;
        
        Ok(Stmt::Function { name, params, body, span })
    }
    
    fn class_declaration(&mut self) -> Result<Stmt> {
        let span = self.advance().span; // consume 'class'
        let name = self.expect_ident("expected class name")?;
        
        self.skip_newlines();
        self.expect(&TokenKind::LeftBrace, "expected '{' before class body")?;
        self.skip_newlines();
        
        // Parse methods (no 'fn' keyword, just name(params) { body })
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            methods.push(self.method_declaration()?);
            self.skip_newlines();
        }
        
        self.expect(&TokenKind::RightBrace, "expected '}' after class body")?;
        
        Ok(Stmt::Class { name, methods, span })
    }
    
    fn method_declaration(&mut self) -> Result<Stmt> {
        let span = self.peek().span;
        let name = self.expect_ident("expected method name")?;
        
        self.expect(&TokenKind::LeftParen, "expected '(' after method name")?;
        
        let mut params = Vec::new();
        if !self.check(&TokenKind::RightParen) {
            loop {
                params.push(self.expect_ident("expected parameter name")?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        
        self.expect(&TokenKind::RightParen, "expected ')' after parameters")?;
        
        self.skip_newlines();
        self.expect(&TokenKind::LeftBrace, "expected '{' before method body")?;
        
        let body = self.block_statements()?;
        
        Ok(Stmt::Function { name, params, body, span })
    }
    
    /// Parse import declaration: import { a, b } from "path"
    fn import_declaration(&mut self) -> Result<Stmt> {
        let span = self.advance().span; // consume 'import'
        
        self.expect(&TokenKind::LeftBrace, "expected '{' after import")?;
        
        let mut names = Vec::new();
        if !self.check(&TokenKind::RightBrace) {
            loop {
                names.push(self.expect_ident("expected import name")?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        
        self.expect(&TokenKind::RightBrace, "expected '}' after import names")?;
        self.expect(&TokenKind::From, "expected 'from' after import names")?;
        
        let path = match &self.peek().kind {
            TokenKind::String(s) => {
                let p = s.clone();
                self.advance();
                p
            }
            _ => return Err(SkyHetuError::new(
                ErrorKind::UnexpectedToken("expected module path string".to_string()),
                Some(self.peek().span),
            )),
        };
        
        self.skip_newlines();
        Ok(Stmt::Import { names, path, span })
    }
    
    /// Parse export declaration: export fn foo() { } or export let x = 1
    fn export_declaration(&mut self) -> Result<Stmt> {
        let span = self.advance().span; // consume 'export'
        
        // Parse the exportable statement
        let stmt = if self.check(&TokenKind::Fn) {
            self.function_declaration()?
        } else if self.check(&TokenKind::Let) {
            self.let_declaration()?
        } else if self.check(&TokenKind::State) {
            self.state_declaration()?
        } else if self.check(&TokenKind::Class) {
            self.class_declaration()?
        } else {
            return Err(SkyHetuError::new(
                ErrorKind::UnexpectedToken("expected fn, let, state, or class after export".to_string()),
                Some(self.peek().span),
            ));
        };
        
        Ok(Stmt::Export { stmt: Box::new(stmt), span })
    }
    
    // ==================== Statements ====================
    
    fn statement(&mut self) -> Result<Stmt> {
        if self.check(&TokenKind::If) {
            self.if_statement()
        } else if self.check(&TokenKind::While) {
            self.while_statement()
        } else if self.check(&TokenKind::For) {
            self.for_statement()
        } else if self.check(&TokenKind::Return) {
            self.return_statement()
        } else if self.check(&TokenKind::Break) {
            self.break_statement()
        } else if self.check(&TokenKind::Continue) {
            self.continue_statement()
        } else if self.check(&TokenKind::LeftBrace) {
            let span = self.peek().span;
            self.advance();
            let stmts = self.block_statements()?;
            Ok(Stmt::Block { stmts, span })
        } else {
            self.expression_or_transition()
        }
    }
    
    fn expression_or_transition(&mut self) -> Result<Stmt> {
        // Check for transition: ident -> expr
        if let TokenKind::Ident(name) = &self.peek().kind {
            let name = name.clone();
            let span = self.peek().span;
            
            // Look ahead for arrow
            if self.peek_next().map(|t| &t.kind) == Some(&TokenKind::Arrow) {
                self.advance(); // consume ident
                self.advance(); // consume arrow
                
                let value = self.expression()?;
                self.skip_newlines();
                
                return Ok(Stmt::Transition { name, value, span });
            }
        }
        
        let expr = self.expression()?;
        self.skip_newlines();
        Ok(Stmt::Expr { expr })
    }
    
    fn if_statement(&mut self) -> Result<Stmt> {
        let span = self.advance().span; // consume 'if'
        
        let condition = self.expression()?;
        
        self.skip_newlines();
        self.expect(&TokenKind::LeftBrace, "expected '{' after if condition")?;
        
        let then_stmts = self.block_statements()?;
        let then_branch = Box::new(Stmt::Block { stmts: then_stmts, span });
        
        self.skip_newlines();
        
        let else_branch = if self.match_token(&TokenKind::Else) {
            self.skip_newlines();
            if self.check(&TokenKind::If) {
                Some(Box::new(self.if_statement()?))
            } else {
                self.expect(&TokenKind::LeftBrace, "expected '{' after else")?;
                let else_stmts = self.block_statements()?;
                Some(Box::new(Stmt::Block { stmts: else_stmts, span }))
            }
        } else {
            None
        };
        
        Ok(Stmt::If { condition, then_branch, else_branch, span })
    }
    
    fn while_statement(&mut self) -> Result<Stmt> {
        let span = self.advance().span; // consume 'while'
        
        let condition = self.expression()?;
        
        self.skip_newlines();
        self.expect(&TokenKind::LeftBrace, "expected '{' after while condition")?;
        
        let body_stmts = self.block_statements()?;
        let body = Box::new(Stmt::Block { stmts: body_stmts, span });
        
        Ok(Stmt::While { condition, body, span })
    }
    
    fn for_statement(&mut self) -> Result<Stmt> {
        let span = self.advance().span; // consume 'for'
        
        let var = self.expect_ident("expected variable name in for loop")?;
        
        // Expect 'in' keyword (we'll use an identifier check)
        if !matches!(self.peek().kind, TokenKind::Ident(ref s) if s == "in") {
            return Err(SkyHetuError::new(
                ErrorKind::ExpectedToken("in".to_string(), format!("{}", self.peek().kind)),
                Some(self.peek().span),
            ));
        }
        self.advance();
        
        let iterable = self.expression()?;
        
        self.skip_newlines();
        self.expect(&TokenKind::LeftBrace, "expected '{' after for condition")?;
        
        let body_stmts = self.block_statements()?;
        let body = Box::new(Stmt::Block { stmts: body_stmts, span });
        
        Ok(Stmt::For { var, iterable, body, span })
    }
    
    fn return_statement(&mut self) -> Result<Stmt> {
        let span = self.advance().span; // consume 'return'
        
        let value = if self.check(&TokenKind::Newline) || self.check(&TokenKind::RightBrace) || self.is_at_end() {
            None
        } else {
            Some(self.expression()?)
        };
        
        self.skip_newlines();
        Ok(Stmt::Return { value, span })
    }
    
    fn break_statement(&mut self) -> Result<Stmt> {
        let span = self.advance().span;
        self.skip_newlines();
        Ok(Stmt::Break { span })
    }
    
    fn continue_statement(&mut self) -> Result<Stmt> {
        let span = self.advance().span;
        self.skip_newlines();
        Ok(Stmt::Continue { span })
    }
    
    fn block_statements(&mut self) -> Result<Vec<Stmt>> {
        let mut stmts = Vec::new();
        
        self.skip_newlines();
        
        while !self.check(&TokenKind::RightBrace) && !self.is_at_end() {
            stmts.push(self.declaration()?);
            self.skip_newlines();
        }
        
        self.expect(&TokenKind::RightBrace, "expected '}' after block")?;
        
        Ok(stmts)
    }
    
    // ==================== Expressions ====================
    
    fn expression(&mut self) -> Result<Expr> {
        self.assignment()
    }
    
    fn assignment(&mut self) -> Result<Expr> {
        let expr = self.or_expr()?;
        
        if self.match_token(&TokenKind::Equal) {
            let equals = self.previous().clone();
            let value = self.assignment()?;
            
            if let Expr::Get { object, name: prop_name, span } = expr {
                return Ok(Expr::Set {
                    object,
                    name: prop_name,
                    value: Box::new(value),
                    span,
                });
            }
            
            return Err(SkyHetuError::new(
                ErrorKind::InvalidAssignment,
                Some(equals.span),
            ));
        }
        
        Ok(expr)
    }
    
    fn or_expr(&mut self) -> Result<Expr> {
        let mut left = self.and_expr()?;
        
        while self.match_token(&TokenKind::Or) {
            let right = self.and_expr()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );
            left = Expr::Logical {
                left: Box::new(left),
                op: LogicalOp::Or,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(left)
    }
    
    fn and_expr(&mut self) -> Result<Expr> {
        let mut left = self.equality()?;
        
        while self.match_token(&TokenKind::And) {
            let right = self.equality()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );
            left = Expr::Logical {
                left: Box::new(left),
                op: LogicalOp::And,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(left)
    }
    
    fn equality(&mut self) -> Result<Expr> {
        let mut left = self.comparison()?;
        
        loop {
            let op = if self.match_token(&TokenKind::EqualEqual) {
                BinaryOp::Eq
            } else if self.match_token(&TokenKind::BangEqual) {
                BinaryOp::Ne
            } else {
                break;
            };
            
            let right = self.comparison()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(left)
    }
    
    fn comparison(&mut self) -> Result<Expr> {
        let mut left = self.term()?;
        
        loop {
            let op = if self.match_token(&TokenKind::Less) {
                BinaryOp::Lt
            } else if self.match_token(&TokenKind::LessEqual) {
                BinaryOp::Le
            } else if self.match_token(&TokenKind::Greater) {
                BinaryOp::Gt
            } else if self.match_token(&TokenKind::GreaterEqual) {
                BinaryOp::Ge
            } else {
                break;
            };
            
            let right = self.term()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(left)
    }
    
    fn term(&mut self) -> Result<Expr> {
        let mut left = self.factor()?;
        
        loop {
            let op = if self.match_token(&TokenKind::Plus) {
                BinaryOp::Add
            } else if self.match_token(&TokenKind::Minus) {
                BinaryOp::Sub
            } else {
                break;
            };
            
            let right = self.factor()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(left)
    }
    
    fn factor(&mut self) -> Result<Expr> {
        let mut left = self.unary()?;
        
        loop {
            let op = if self.match_token(&TokenKind::Star) {
                BinaryOp::Mul
            } else if self.match_token(&TokenKind::Slash) {
                BinaryOp::Div
            } else if self.match_token(&TokenKind::Percent) {
                BinaryOp::Mod
            } else {
                break;
            };
            
            let right = self.unary()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().column,
            );
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        
        Ok(left)
    }
    
    fn unary(&mut self) -> Result<Expr> {
        if self.match_token(&TokenKind::Minus) {
            let span = self.previous().span;
            let operand = self.unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Neg,
                operand: Box::new(operand),
                span,
            });
        }
        
        if self.match_token(&TokenKind::Bang) {
            let span = self.previous().span;
            let operand = self.unary()?;
            return Ok(Expr::Unary {
                op: UnaryOp::Not,
                operand: Box::new(operand),
                span,
            });
        }
        
        self.call()
    }
    
    fn call(&mut self) -> Result<Expr> {
        let mut expr = self.primary()?;
        
        loop {
            if self.match_token(&TokenKind::LeftParen) {
                expr = self.finish_call(expr)?;
            } else if self.match_token(&TokenKind::Dot) {
                let name = self.expect_ident("expected property name after '.'")?;
                let dot_span = self.previous().span; 
                let expr_span = expr.span();
                expr = Expr::Get { 
                    object: Box::new(expr), 
                    name, 
                    span: Span::new(expr_span.start, dot_span.end, expr_span.line, expr_span.column) 
                };
            } else {
                break;
            }
        }
        
        Ok(expr)
    }
    
    fn finish_call(&mut self, callee: Expr) -> Result<Expr> {
        let mut args = Vec::new();
        
        if !self.check(&TokenKind::RightParen) {
            loop {
                args.push(self.expression()?);
                if !self.match_token(&TokenKind::Comma) {
                    break;
                }
            }
        }
        
        let end_span = self.peek().span;
        self.expect(&TokenKind::RightParen, "expected ')' after arguments")?;
        
        let span = Span::new(
            callee.span().start,
            end_span.end,
            callee.span().line,
            callee.span().column,
        );
        
        Ok(Expr::Call {
            callee: Box::new(callee),
            args,
            span,
        })
    }
    
    fn primary(&mut self) -> Result<Expr> {
        let token = self.peek().clone();
        
        match &token.kind {
            TokenKind::Number(n) => {
                let value = *n;
                self.advance();
                Ok(Expr::Number { value, span: token.span })
            }
            TokenKind::String(s) => {
                let value = s.clone();
                self.advance();
                Ok(Expr::String { value, span: token.span })
            }
            TokenKind::True => {
                self.advance();
                Ok(Expr::Bool { value: true, span: token.span })
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::Bool { value: false, span: token.span })
            }
            TokenKind::Nil => {
                self.advance();
                Ok(Expr::Nil { span: token.span })
            }
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expr::Ident { name, span: token.span })
            }
            TokenKind::LeftParen => {
                let start_span = token.span;
                self.advance();
                let expr = self.expression()?;
                self.expect(&TokenKind::RightParen, "expected ')' after expression")?;
                Ok(Expr::Grouping {
                    expr: Box::new(expr),
                    span: start_span,
                })
            }
            _ => Err(SkyHetuError::new(
                ErrorKind::ExpectedExpression,
                Some(token.span),
            )),
        }
    }
    
    // ==================== Helpers ====================
    
    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }
    
    fn peek_next(&self) -> Option<&Token> {
        self.tokens.get(self.current + 1)
    }
    
    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
    
    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }
    
    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }
    
    fn check(&self, kind: &TokenKind) -> bool {
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }
    
    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }
    
    fn expect(&mut self, kind: &TokenKind, message: &str) -> Result<&Token> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            Err(SkyHetuError::new(
                ErrorKind::ExpectedToken(message.to_string(), format!("{}", self.peek().kind)),
                Some(self.peek().span),
            ))
        }
    }
    
    fn expect_ident(&mut self, message: &str) -> Result<String> {
        if let TokenKind::Ident(name) = &self.peek().kind {
            let name = name.clone();
            self.advance();
            Ok(name)
        } else {
            Err(SkyHetuError::new(
                ErrorKind::ExpectedToken(message.to_string(), format!("{}", self.peek().kind)),
                Some(self.peek().span),
            ))
        }
    }
    
    fn skip_newlines(&mut self) {
        while self.match_token(&TokenKind::Newline) {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    
    fn parse(source: &str) -> Program {
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        parser.parse().unwrap()
    }
    
    #[test]
    fn test_let_statement() {
        let program = parse("let x = 42");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Let { name, .. } => assert_eq!(name, "x"),
            _ => panic!("expected let statement"),
        }
    }
    
    #[test]
    fn test_state_statement() {
        let program = parse("state counter = 0");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::State { name, .. } => assert_eq!(name, "counter"),
            _ => panic!("expected state statement"),
        }
    }
    
    #[test]
    fn test_transition() {
        let program = parse("counter -> counter + 1");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Transition { name, .. } => assert_eq!(name, "counter"),
            _ => panic!("expected transition statement"),
        }
    }
    
    #[test]
    fn test_function() {
        let program = parse("fn add(a, b) { return a + b }");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::Function { name, params, .. } => {
                assert_eq!(name, "add");
                assert_eq!(params, &["a", "b"]);
            }
            _ => panic!("expected function"),
        }
    }
    
    #[test]
    fn test_if_else() {
        let program = parse("if x > 0 { print(x) } else { print(0) }");
        assert_eq!(program.statements.len(), 1);
        match &program.statements[0] {
            Stmt::If { else_branch, .. } => {
                assert!(else_branch.is_some());
            }
            _ => panic!("expected if statement"),
        }
    }
    
    #[test]
    fn test_binary_expr() {
        let program = parse("1 + 2 * 3");
        assert_eq!(program.statements.len(), 1);
    }
}
