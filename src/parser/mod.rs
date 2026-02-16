pub mod token;
pub mod parser;
pub mod program;
pub mod map;
pub mod unit;
pub mod event;

pub use token::{Token, TokenKind, SourceLoc};
pub use parser::{Parser, ParseError};

use crate::ast::Program;

pub fn parse(src: &str) -> Result<Program, ParseError> {
    let mut parser = Parser::new(src)?;
    program::parse_program(&mut parser)
}