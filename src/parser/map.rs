use super::{ParseError, Parser};
use crate::{
    ast::{MapDecl, MapType, Type},
    parser::TokenKind,
};

pub fn parse_map(parser: &mut Parser) -> Result<MapDecl, ParseError> {
    let map_loc = parser.current_loc();

    let map_name_tok = parser.expect(TokenKind::Identifier)?;
    expect_token(parser, TokenKind::LBrace)?;

    let mut map_type: Option<MapType> = None;
    let mut key_type: Option<Type> = None;
    let mut value_type: Option<Type> = None;
    let mut max_entries: Option<u32> = None;

    while !parser.check(TokenKind::RBrace) {
        if parser.r#match(TokenKind::KeywordType) {
            expect_token(parser, TokenKind::Colon)?;
            expect_token(parser, TokenKind::Dot)?;
            let t_tok = parser.expect(TokenKind::Identifier)?;

            map_type = Some(match t_tok.lexeme.as_str() {
                "hash" => MapType::Hash,
                "array" => MapType::Array,
                "ringbuf" => MapType::Ringbuf,
                "lru_hash" => MapType::LruHash,
                "prog_array" => MapType::ProgArray,
                _ => {
                    return Err(parser.error(format!(
                        "Unknown map type: {} (valid: hash, array, ringbuf, lru_hash, prog_array)",
                        t_tok.lexeme
                    )));
                }
            });

            expect_token(parser, TokenKind::Semicolon)?;
            continue;
        }
        if parser.r#match(TokenKind::KeywordKey) {
            expect_token(parser, TokenKind::Colon)?;
            key_type = Some(parse_type(parser)?);
            expect_token(parser, TokenKind::Semicolon)?;
            continue;
        }
        if parser.r#match(TokenKind::KeywordValue) {
            expect_token(parser, TokenKind::Colon)?;
            value_type = Some(parse_type(parser)?);
            expect_token(parser, TokenKind::Semicolon)?;
            continue;
        }
        if parser.r#match(TokenKind::KeywordMax) {
            expect_token(parser, TokenKind::Colon)?;
            let max = parse_const_expr(parser)?;

            if max < 0 {
                return Err(parser.error("max_entries must be >= 0"));
            }

            max_entries = Some(max as u32);

            expect_token(parser, TokenKind::Semicolon)?;
            continue;
        }

        return Err(parser.error(format!(
            "Unexpected token inside map: {} (expected one of: type, key, value, max)",
            parser.current_kind()
        )));
    }

    expect_token(parser, TokenKind::RBrace)?;

    let map_type = map_type.unwrap();

    match map_type {
        MapType::Ringbuf => {
            if max_entries.is_none() {
                return Err(parser.error("Ringbuf map requires: max"));
            }
        }

        MapType::Hash | MapType::Array | MapType::LruHash | MapType::ProgArray => {
            if key_type.is_none() {
                return Err(parser.error("Map missing required field: key"));
            }
            if value_type.is_none() {
                return Err(parser.error("Map missing required field: value"));
            }
            if max_entries.is_none() {
                return Err(parser.error("Map missing required field: max"));
            }
        }
    }

    Ok(MapDecl {
    name: map_name_tok.lexeme,
    map_type,
    key_type,
    value_type,
    max_entries,
    loc: map_loc,
})

}

pub fn parse_type(parser: &mut Parser) -> Result<Type, ParseError> {
    let t = parser.current().clone();
    parser.advance()?;

    match t.kind {
        TokenKind::TypeU32 => Ok(Type::U32),
        TokenKind::TypeU64 => Ok(Type::U64),
        TokenKind::TypeI32 => Ok(Type::I32),
        TokenKind::TypeI64 => Ok(Type::I64),
        _ => Err(parser.error("Expected type (u32, u64, i32, i64)")),
    }
}

pub fn expect_token(parser: &mut Parser, kind: TokenKind) -> Result<(), ParseError> {
    parser.expect(kind)?;
    Ok(())
}

fn parse_const_expr(parser: &mut Parser) -> Result<i64, ParseError> {
    parse_shift(parser)
}

fn parse_shift(parser: &mut Parser) -> Result<i64, ParseError> {
    let mut left = parse_primary(parser)?;

    loop {
        if parser.check(TokenKind::Shl) {
            parser.advance()?;
            let right = parse_primary(parser)?;
            left = left << right;
        } else if parser.check(TokenKind::Shr) {
            parser.advance()?;
            let right = parse_primary(parser)?;
            left = left >> right;
        } else {
            break;
        }
    }

    Ok(left)
}

fn parse_primary(parser: &mut Parser) -> Result<i64, ParseError> {
    let n = parser.expect(TokenKind::Number)?;
    n.int_value
        .ok_or_else(|| parser.error("Expected integer literal"))
}
