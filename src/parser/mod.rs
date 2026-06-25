pub mod event;
pub mod map;
pub mod parser;
pub mod program;
pub mod token;
pub mod unit;

pub use parser::{ParseError, Parser};
pub use token::{SourceLoc, Token, TokenKind};

use crate::ast::Program;
use crate::lexer::Lexer;

#[allow(dead_code)]
pub fn parse(src: &str) -> Result<Program, ParseError> {
    let tokens = Lexer::new(src).tokenize()?;
    parse_tokens(tokens)
}

pub fn parse_tokens(tokens: Vec<Token>) -> Result<Program, ParseError> {
    let mut parser = Parser::new(tokens)?;
    program::parse_program(&mut parser)
}
