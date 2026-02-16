use crate::{
    ast::{EventDecl, EventField, EventType, PrimitiveType},
    parser::{ParseError, Parser, TokenKind},
};

pub fn parse_event(parser: &mut Parser) -> Result<EventDecl, ParseError> {
    // event
    parser.expect(TokenKind::KeywordEvent)?;

    // name
    let name_tok = parser.expect(TokenKind::Identifier)?;
    let name = name_tok.lexeme.clone();
    let loc = name_tok.loc;

    // {
    parser.expect(TokenKind::LBrace)?;

    let mut fields = Vec::new();

    while !parser.check(TokenKind::RBrace) {
        fields.push(parse_event_field(parser)?);
    }

    // }
    parser.expect(TokenKind::RBrace)?;

    Ok(EventDecl { name, fields, loc })
}

fn parse_event_field(parser: &mut Parser) -> Result<EventField, ParseError> {
    let name_tok = parser.expect(TokenKind::Identifier)?;
    let name = name_tok.lexeme.clone();
    let loc = name_tok.loc;

    parser.expect(TokenKind::Colon)?;

    let ty = parse_event_type(parser)?;

    parser.expect(TokenKind::Semicolon)?;

    Ok(EventField { name, ty, loc })
}

fn parse_event_type(parser: &mut Parser) -> Result<EventType, ParseError> {
    let loc = parser.current_loc();

    // Primitive types
    let primitive = match parser.current_kind() {
        TokenKind::TypeU32 => {
            parser.advance()?;
            Some(PrimitiveType::U32)
        }
        TokenKind::TypeU64 => {
            parser.advance()?;
            Some(PrimitiveType::U64)
        }
        TokenKind::TypeI32 => {
            parser.advance()?;
            Some(PrimitiveType::I32)
        }
        TokenKind::TypeI64 => {
            parser.advance()?;
            Some(PrimitiveType::I64)
        }
        _ => None,
    };

    if let Some(p) = primitive {
        // Array form: u32[16]
        if parser.r#match(TokenKind::LBracket) {
            let len_tok = parser.expect(TokenKind::Number)?;

            let len = len_tok.int_value
                .ok_or((len_tok.loc, "Invalid array length".to_string()))?
                as u32;

            parser.expect(TokenKind::RBracket)?;

            return Ok(EventType::Array { elem: p, len });
        }

        return Ok(match p {
            PrimitiveType::U32 => EventType::U32,
            PrimitiveType::U64 => EventType::U64,
            PrimitiveType::I32 => EventType::I32,
            PrimitiveType::I64 => EventType::I64,
        });
    }

    // bytes[256]
    if parser.r#match(TokenKind::KeywordBytes) {
        parser.expect(TokenKind::LBracket)?;

        let len_tok = parser.expect(TokenKind::Number)?;
        let len = len_tok.int_value
            .ok_or((len_tok.loc, "Invalid bytes length".to_string()))?
            as u32;

        parser.expect(TokenKind::RBracket)?;

        return Ok(EventType::Bytes(len));
    }

    Err((loc, "Invalid event field type".to_string()))
}
