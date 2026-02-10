use super::{Parser, ParseError};
use crate::{ast::{MapDecl, MapType, Type}, parser::TokenKind};

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
            let n = parser.expect(TokenKind::Number)?;
            
            let max = n.int_value.ok_or_else(|| {
                parser.error("Expected integer value for max_entries")
            })?;
            
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
    
    if map_type.is_none() {
        return Err(parser.error("Map missing required field: type"));
    }
    if key_type.is_none() {
        return Err(parser.error("Map missing required field: key"));
    }
    if value_type.is_none() {
        return Err(parser.error("Map missing required field: value"));
    }
    if max_entries.is_none() {
        return Err(parser.error("Map missing required field: max"));
    }

    Ok(MapDecl {
        name: map_name_tok.lexeme,
        map_type: map_type.unwrap(),
        key_type: key_type.unwrap(),
        value_type: value_type.unwrap(),
        max_entries: max_entries.unwrap(),
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