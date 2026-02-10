use super::Parser;
use super::ParseError;
use crate::ast::Program;
use crate::parser::TokenKind;
use crate::parser::map::parse_map;
use crate::parser::unit::parse_unit;

pub fn parse_program(parser: &mut Parser) -> Result<Program, ParseError> {
    let mut maps = Vec::new();
    let mut units = Vec::new();

    while !parser.check(TokenKind::Eof) {
        if parser.r#match(TokenKind::KeywordMap) {
            let map = parse_map(parser)?;
            maps.push(map);
        } else if parser.check(TokenKind::KeywordUnit) {
            let unit = parse_unit(parser)?;
            units.push(unit);
        } else {
            return Err(parser.error("Expected 'map' or 'unit'"));
        }
    }

    Ok(Program { maps, units })
}