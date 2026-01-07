//! Lexer for SkyHetu language
//!
//! Converts source code into a stream of tokens.

use crate::error::{ErrorKind, Result, SkyHetuError};
use crate::token::{lookup_keyword, Span, Token, TokenKind};

/// The lexer state
pub struct Lexer<'a> {
    source: &'a str,
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    current_pos: usize,
    line: usize,
    column: usize,
    line_start: usize,
}

impl<'a> Lexer<'a> {
    /// Create a new lexer from source code
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            chars: source.char_indices().peekable(),
            current_pos: 0,
            line: 1,
            column: 1,
            line_start: 0,
        }
    }
    
    /// Tokenize the entire source
    pub fn tokenize(&mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        
        while let Some(token) = self.next_token()? {
            tokens.push(token);
        }
        
        // Add EOF token
        tokens.push(Token::new(
            TokenKind::Eof,
            Span::new(self.current_pos, self.current_pos, self.line, self.column),
            String::new(),
        ));
        
        Ok(tokens)
    }
    
    /// Get the next token
    fn next_token(&mut self) -> Result<Option<Token>> {
        self.skip_whitespace_and_comments();
        
        let Some(&(start_pos, ch)) = self.chars.peek() else {
            return Ok(None);
        };
        
        let start_line = self.line;
        let start_column = self.column;
        
        let kind = match ch {
            // Single character tokens
            '(' => { self.advance(); TokenKind::LeftParen }
            ')' => { self.advance(); TokenKind::RightParen }
            '{' => { self.advance(); TokenKind::LeftBrace }
            '}' => { self.advance(); TokenKind::RightBrace }
            '[' => { self.advance(); TokenKind::LeftBracket }
            ']' => { self.advance(); TokenKind::RightBracket }
            ',' => { self.advance(); TokenKind::Comma }
            ';' => { self.advance(); TokenKind::Semicolon }
            ':' => { self.advance(); TokenKind::Colon }
            '.' => { self.advance(); TokenKind::Dot }
            '+' => { self.advance(); TokenKind::Plus }
            '*' => { self.advance(); TokenKind::Star }
            '%' => { self.advance(); TokenKind::Percent }
            
            // Potentially two-character tokens
            '-' => {
                self.advance();
                if self.peek_char() == Some('>') {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }
            '/' => { self.advance(); TokenKind::Slash }
            '=' => {
                self.advance();
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::EqualEqual
                } else if self.peek_char() == Some('>') {
                    self.advance();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Equal
                }
            }
            '!' => {
                self.advance();
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::BangEqual
                } else {
                    TokenKind::Bang
                }
            }
            '<' => {
                self.advance();
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }
            '>' => {
                self.advance();
                if self.peek_char() == Some('=') {
                    self.advance();
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }
            
            // Newlines (significant for statement separation)
            '\n' => {
                self.advance();
                self.line += 1;
                self.column = 1;
                self.line_start = self.current_pos;
                TokenKind::Newline
            }
            
            // String literals
            '"' => self.scan_string()?,
            
            // Number literals
            c if c.is_ascii_digit() => self.scan_number()?,
            
            // Identifiers and keywords
            c if c.is_alphabetic() || c == '_' => self.scan_identifier()?,
            
            // Unknown character
            _ => {
                self.advance();
                return Err(SkyHetuError::new(
                    ErrorKind::UnexpectedCharacter(ch),
                    Some(Span::new(start_pos, self.current_pos, start_line, start_column)),
                ));
            }
        };
        
        let lexeme = self.source[start_pos..self.current_pos].to_string();
        
        Ok(Some(Token::new(
            kind,
            Span::new(start_pos, self.current_pos, start_line, start_column),
            lexeme,
        )))
    }
    
    /// Advance and return the current character
    fn advance(&mut self) -> Option<char> {
        if let Some((pos, ch)) = self.chars.next() {
            self.current_pos = pos + ch.len_utf8();
            self.column += 1;
            Some(ch)
        } else {
            None
        }
    }
    
    /// Peek at the next character without advancing
    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|&(_, ch)| ch)
    }
    
    /// Skip whitespace (except newlines) and comments
    fn skip_whitespace_and_comments(&mut self) {
        while let Some(&(_, ch)) = self.chars.peek() {
            match ch {
                // Regular whitespace (not newline)
                ' ' | '\t' | '\r' => {
                    self.advance();
                }
                
                // Comments
                '/' if self.source[self.current_pos..].starts_with("//") => {
                    // Skip to end of line
                    while let Some(&(_, c)) = self.chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                
                _ => break,
            }
        }
    }
    
    /// Scan a string literal
    fn scan_string(&mut self) -> Result<TokenKind> {
        let start_line = self.line;
        let start_column = self.column;
        let start_pos = self.current_pos;
        
        // Consume opening quote
        self.advance();
        
        let mut value = String::new();
        
        loop {
            match self.peek_char() {
                Some('"') => {
                    self.advance();
                    return Ok(TokenKind::String(value));
                }
                Some('\\') => {
                    self.advance();
                    match self.peek_char() {
                        Some('n') => { self.advance(); value.push('\n'); }
                        Some('t') => { self.advance(); value.push('\t'); }
                        Some('r') => { self.advance(); value.push('\r'); }
                        Some('\\') => { self.advance(); value.push('\\'); }
                        Some('"') => { self.advance(); value.push('"'); }
                        Some(c) => { self.advance(); value.push(c); }
                        None => break,
                    }
                }
                Some('\n') => {
                    value.push('\n');
                    self.advance();
                    self.line += 1;
                    self.column = 1;
                    self.line_start = self.current_pos;
                }
                Some(c) => {
                    value.push(c);
                    self.advance();
                }
                None => break,
            }
        }
        
        Err(SkyHetuError::new(
            ErrorKind::UnterminatedString,
            Some(Span::new(start_pos, self.current_pos, start_line, start_column)),
        ))
    }
    
    /// Scan a number literal
    fn scan_number(&mut self) -> Result<TokenKind> {
        let start = self.current_pos;
        
        // Consume digits
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                self.advance();
            } else {
                break;
            }
        }
        
        // Check for decimal point
        if self.peek_char() == Some('.') {
            // Look ahead to see if it's followed by a digit
            let remaining = &self.source[self.current_pos..];
            if remaining.len() > 1 && remaining.chars().nth(1).map_or(false, |c| c.is_ascii_digit()) {
                self.advance(); // Consume the dot
                
                // Consume decimal digits
                while let Some(c) = self.peek_char() {
                    if c.is_ascii_digit() {
                        self.advance();
                    } else {
                        break;
                    }
                }
            }
        }
        
        let text = &self.source[start..self.current_pos];
        match text.parse::<f64>() {
            Ok(value) => Ok(TokenKind::Number(value)),
            Err(_) => Err(SkyHetuError::new(
                ErrorKind::InvalidNumber(text.to_string()),
                Some(Span::new(start, self.current_pos, self.line, self.column)),
            )),
        }
    }
    
    /// Scan an identifier or keyword
    fn scan_identifier(&mut self) -> Result<TokenKind> {
        let start = self.current_pos;
        
        while let Some(c) = self.peek_char() {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }
        
        let text = &self.source[start..self.current_pos];
        
        // Check if it's a keyword
        if let Some(keyword) = lookup_keyword(text) {
            Ok(keyword)
        } else {
            Ok(TokenKind::Ident(text.to_string()))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn tokenize(source: &str) -> Vec<TokenKind> {
        let mut lexer = Lexer::new(source);
        lexer.tokenize()
            .unwrap()
            .into_iter()
            .map(|t| t.kind)
            .filter(|k| !matches!(k, TokenKind::Newline | TokenKind::Eof))
            .collect()
    }
    
    #[test]
    fn test_keywords() {
        let tokens = tokenize("let state fn return if else while");
        assert_eq!(tokens, vec![
            TokenKind::Let,
            TokenKind::State,
            TokenKind::Fn,
            TokenKind::Return,
            TokenKind::If,
            TokenKind::Else,
            TokenKind::While,
        ]);
    }
    
    #[test]
    fn test_operators() {
        let tokens = tokenize("+ - * / = == != < <= > >= ->");
        assert_eq!(tokens, vec![
            TokenKind::Plus,
            TokenKind::Minus,
            TokenKind::Star,
            TokenKind::Slash,
            TokenKind::Equal,
            TokenKind::EqualEqual,
            TokenKind::BangEqual,
            TokenKind::Less,
            TokenKind::LessEqual,
            TokenKind::Greater,
            TokenKind::GreaterEqual,
            TokenKind::Arrow,
        ]);
    }
    
    #[test]
    fn test_numbers() {
        let tokens = tokenize("42 3.14 0 100.0");
        assert_eq!(tokens, vec![
            TokenKind::Number(42.0),
            TokenKind::Number(3.14),
            TokenKind::Number(0.0),
            TokenKind::Number(100.0),
        ]);
    }
    
    #[test]
    fn test_strings() {
        let tokens = tokenize(r#""hello" "world""#);
        assert_eq!(tokens, vec![
            TokenKind::String("hello".to_string()),
            TokenKind::String("world".to_string()),
        ]);
    }
    
    #[test]
    fn test_identifiers() {
        let tokens = tokenize("foo bar_baz x1 _private");
        assert_eq!(tokens, vec![
            TokenKind::Ident("foo".to_string()),
            TokenKind::Ident("bar_baz".to_string()),
            TokenKind::Ident("x1".to_string()),
            TokenKind::Ident("_private".to_string()),
        ]);
    }
    
    #[test]
    fn test_arrow() {
        let tokens = tokenize("counter -> counter + 1");
        assert_eq!(tokens, vec![
            TokenKind::Ident("counter".to_string()),
            TokenKind::Arrow,
            TokenKind::Ident("counter".to_string()),
            TokenKind::Plus,
            TokenKind::Number(1.0),
        ]);
    }
}
