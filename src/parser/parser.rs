#![allow(unused_assignments)]

use super::{Token, TokenKind};
use crate::lexer::Lexer;
use anyhow::Result;

pub type ParseError = (crate::parser::SourceLoc, String);

pub struct Parser<'src> {
    _src: &'src str,
    lexer: Lexer<'src>,
    current: Token,
}

impl<'src> Parser<'src> {
    pub fn new(src: &'src str) -> Result<Self, ParseError> {
        let mut lexer = Lexer::new(src);
        let current = lexer.next_token().map_err(|_e| {
            (super::SourceLoc::new(0, 0, 0), "Failed to lex first token".to_string())
        })?;

        Ok(Self {
            _src: src,
            lexer,
            current,
        })
    }

    pub fn advance(&mut self) -> Result<(), ParseError> {
        self.current = self
            .lexer
            .next_token()
            .map_err(|_e| (self.current.loc, "Lexer error".to_string()))?;
        Ok(())
    }

    pub fn check(&self, kind: TokenKind) -> bool {
        self.current.kind == kind
    }

    pub fn r#match(&mut self, kind: TokenKind) -> bool {
        if self.check(kind) {
            let _ = self.advance();
            true
        } else {
            false
        }
    }

    pub fn expect(&mut self, kind: TokenKind) -> Result<Token, ParseError> {
        if self.check(kind) {
            let tok = self.current.clone();
            self.advance()?;
            Ok(tok)
        } else {
            Err((
                self.current.loc,
                format!("Expected {}, found {}", kind, self.current.kind),
            ))
        }
    }

    pub fn error(&self, message: impl Into<String>) -> ParseError {
        (self.current.loc, message.into())
    }

    pub fn current(&self) -> &Token {
        &self.current
    }

    pub fn current_loc(&self) -> crate::parser::SourceLoc {
        self.current.loc
    }

    pub fn current_kind(&self) -> TokenKind {
        self.current.kind
    }

    pub fn _current_lexeme(&self) -> &str {
        &self.current.lexeme
    }
    
}
