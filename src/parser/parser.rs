use super::{Token, TokenKind};

pub type ParseError = (crate::parser::SourceLoc, String);

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Result<Self, ParseError> {
        if tokens.is_empty() {
            return Err((
                super::SourceLoc::new(0, 0, 0),
                "Parser received an empty token stream".to_string(),
            ));
        }

        if tokens.last().map(|token| token.kind) != Some(TokenKind::Eof) {
            return Err((
                tokens
                    .last()
                    .map(|token| token.loc)
                    .unwrap_or_else(|| super::SourceLoc::new(0, 0, 0)),
                "Token stream must end with EOF".to_string(),
            ));
        }

        Ok(Self { tokens, current: 0 })
    }

    pub fn advance(&mut self) -> Result<(), ParseError> {
        if !self.check(TokenKind::Eof) {
            self.current += 1;
        }
        Ok(())
    }

    pub fn check(&self, kind: TokenKind) -> bool {
        self.current().kind == kind
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
            let tok = self.current().clone();
            self.advance()?;
            Ok(tok)
        } else {
            Err((
                self.current().loc,
                format!("Expected {}, found {}", kind, self.current().kind),
            ))
        }
    }

    pub fn error(&self, message: impl Into<String>) -> ParseError {
        (self.current().loc, message.into())
    }

    pub fn current(&self) -> &Token {
        &self.tokens[self.current]
    }

    pub fn current_loc(&self) -> crate::parser::SourceLoc {
        self.current().loc
    }

    pub fn current_kind(&self) -> TokenKind {
        self.current().kind
    }

    pub fn _current_lexeme(&self) -> &str {
        &self.current().lexeme
    }
}
