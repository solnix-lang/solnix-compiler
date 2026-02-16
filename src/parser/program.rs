use super::ParseError;
use super::Parser;
use crate::ast::Program;
use crate::parser::event::parse_event;
use crate::parser::map::parse_map;
use crate::parser::unit::parse_unit;
use crate::parser::TokenKind;

pub fn parse_program(parser: &mut Parser) -> Result<Program, ParseError> {
    let mut maps = Vec::new();
    let mut units = Vec::new();
    let mut events = Vec::new();

    while !parser.check(TokenKind::Eof) {
        if parser.r#match(TokenKind::KeywordMap) {
            let map = parse_map(parser)?;
            maps.push(map);
        } else if parser.check(TokenKind::KeywordUnit) {
            let unit = parse_unit(parser)?;
            units.push(unit);
        } else if parser.check(TokenKind::KeywordEvent) {
            let event = parse_event(parser)?;
            events.push(event);
        } else {
            return Err(parser.error("Expected 'map', 'unit', or 'event'"));
        }
    }

    Ok(Program { maps, units, events })
}
