use super::{Parser, ParseError};
use crate::{ast::{
    Assignment, AssignmentOp,  Expr, ExprKind, HeapLookup, HeapVarDecl, IfGuard, MethodCall, Stmt, StmtKind, Unit, VarDecl, VarType
}, parser::{TokenKind, map::expect_token}};
use std::boxed::Box;

pub fn parse_unit(parser: &mut Parser) -> Result<Unit, ParseError> {
    let unit_loc = parser.current_loc();
    expect_token(parser, TokenKind::KeywordUnit)?;

    let name_tok = parser.expect(TokenKind::Identifier)?;
    expect_token(parser, TokenKind::LBrace)?;

    let mut sections = Vec::new();
    let mut body = Vec::new();
    let mut license: Option<String> = None;

    while !parser.check(TokenKind::RBrace) {
        if parser.r#match(TokenKind::KeywordSection) {
            expect_token(parser, TokenKind::Colon)?;
            let s = parser.expect(TokenKind::StringLiteral)?;
            
            let txt = s.lexeme.trim_matches('"').to_string();
            sections.push(txt);
            expect_token(parser, TokenKind::Semicolon)?;
            continue;
        }

        if parser.r#match(TokenKind::KeywordLicense) {
            expect_token(parser, TokenKind::Colon)?;
            let s = parser.expect(TokenKind::StringLiteral)?;
            
            let txt = s.lexeme.trim_matches('"').to_string();
            license = Some(txt);
            expect_token(parser, TokenKind::Semicolon)?;
            continue;
        }

        parse_stmt(parser, &mut body)?;
    }

    expect_token(parser, TokenKind::RBrace)?;

    Ok(Unit {
        name: name_tok.lexeme,
        loc: unit_loc,
        sections,
        license,
        body,
    })
}

fn parse_stmt(parser: &mut Parser, body: &mut Vec<Stmt>) -> Result<(), ParseError> {
    if parser.r#match(TokenKind::KeywordReg) {
        let var_loc = parser.current_loc();
        let var_name_tok = parser.expect(TokenKind::Identifier)?;
        expect_token(parser, TokenKind::Equals)?;
        let value_expr = parse_expr(parser)?;
        expect_token(parser, TokenKind::Semicolon)?;

        body.push(Stmt {
            kind: StmtKind::VarDecl(VarDecl {
                name: var_name_tok.lexeme,
                var_type: VarType::Reg,
                value: Box::new(value_expr),
            }),
            loc: var_loc,
        });
        return Ok(());
    }

    if parser.r#match(TokenKind::KeywordImm) {
        let var_loc = parser.current_loc();
        let var_name_tok = parser.expect(TokenKind::Identifier)?;
        expect_token(parser, TokenKind::Equals)?;
        let value_expr = parse_expr(parser)?;
        expect_token(parser, TokenKind::Semicolon)?;

        body.push(Stmt {
            kind: StmtKind::VarDecl(VarDecl {
                name: var_name_tok.lexeme,
                var_type: VarType::Imm,
                value: Box::new(value_expr),
            }),
            loc: var_loc,
        });
        return Ok(());
    }

    if parser.r#match(TokenKind::KeywordHeap) {
        let var_loc = parser.current_loc();
        let var_name_tok = parser.expect(TokenKind::Identifier)?;
        expect_token(parser, TokenKind::Equals)?;

        let map_name_tok = parser.expect(TokenKind::Identifier)?;
        expect_token(parser, TokenKind::Dot)?;
        let lookup_tok = parser.expect(TokenKind::Identifier)?;
        
        if lookup_tok.lexeme != "lookup" {
            return Err(parser.error(format!("Expected 'lookup' method, found: {}", lookup_tok.lexeme)));
        }

        expect_token(parser, TokenKind::LParen)?;
        let key_expr = parse_expr(parser)?;
        expect_token(parser, TokenKind::RParen)?;
        expect_token(parser, TokenKind::Semicolon)?;

        body.push(Stmt {
            kind: StmtKind::HeapVarDecl(HeapVarDecl {
                name: var_name_tok.lexeme,
                lookup: HeapLookup {
                    map_name: map_name_tok.lexeme,
                    key_expr: Box::new(key_expr),
                },
            }),
            loc: var_loc,
        });
        return Ok(());
    }

    if parser.r#match(TokenKind::KeywordReturn) {
        let return_loc = parser.current_loc();
        let v = parse_expr(parser)?;
        expect_token(parser, TokenKind::Semicolon)?;

        body.push(Stmt {
            kind: StmtKind::Return(Box::new(v)),
            loc: return_loc,
        });
        return Ok(());
    }

    if parser.r#match(TokenKind::KeywordIf) {
        let if_loc = parser.current_loc();
        expect_token(parser, TokenKind::KeywordGuard)?;
        expect_token(parser, TokenKind::LParen)?;
        let var_tok = parser.expect(TokenKind::Identifier)?;
        expect_token(parser, TokenKind::RParen)?;
        expect_token(parser, TokenKind::LBrace)?;

        let mut guard_body = Vec::new();
        while !parser.r#match(TokenKind::RBrace) {
            parse_stmt(parser, &mut guard_body)?;
        }

        body.push(Stmt {
            kind: StmtKind::IfGuard(IfGuard {
                condition: Expr {
                    kind: ExprKind::Variable(var_tok.lexeme.clone()),
                    loc: var_tok.loc,
                },
                body: guard_body,
            }),
            loc: if_loc,
        });
        return Ok(());
    }

    let target = parse_expr(parser)?;
    let target_loc = target.loc;
    if parser.r#match(TokenKind::Equals) {
        let value = parse_expr(parser)?;
        expect_token(parser, TokenKind::Semicolon)?;
        
        body.push(Stmt {
            kind: StmtKind::Assignment(Assignment {
                target: Box::new(target),
                op: AssignmentOp::Assign,
                value: Box::new(value),
            }),
            loc: target_loc,
        });
        return Ok(());
    } else if parser.r#match(TokenKind::PlusEquals) {
        let value = parse_expr(parser)?;
        expect_token(parser, TokenKind::Semicolon)?;
        
        body.push(Stmt {
            kind: StmtKind::Assignment(Assignment {
                target: Box::new(target),
                op: AssignmentOp::AddAssign,
                value: Box::new(value),
            }),
            loc: target_loc,
        });
        return Ok(());
    }

    Err(parser.error("Unexpected statement: expected reg, imm, heap, return, if, or expression"))
}

pub fn parse_expr(parser: &mut Parser) -> Result<Expr, ParseError> {
    parse_add(parser)
}

fn parse_add(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut expr = parse_shift(parser)?;

    loop {
        if parser.r#match(TokenKind::Plus) {
            let rhs = parse_mul(parser)?;
            let loc = expr.loc;
            expr = Expr {
                kind: ExprKind::Binary(crate::ast::BinaryExpr {
                    op: crate::ast::BinOp::Add,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                }),
                loc,
            };
            continue;
        }

        if parser.r#match(TokenKind::Minus) {
            let rhs = parse_mul(parser)?;
            let loc = expr.loc;
            expr = Expr {
                kind: ExprKind::Binary(crate::ast::BinaryExpr {
                    op: crate::ast::BinOp::Sub,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                }),
                loc,
            };
            continue;
        }

        break;
    }

    Ok(expr)
}

// * / %
fn parse_mul(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut expr = parse_unary(parser)?;

    loop {
        if parser.r#match(TokenKind::Star) {
            let rhs = parse_unary(parser)?;
            let loc = expr.loc;
            expr = Expr {
                kind: ExprKind::Binary(crate::ast::BinaryExpr {
                    op: crate::ast::BinOp::Mul,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                }),
                loc,
            };
            continue;
        }

        if parser.r#match(TokenKind::Slash) {
            let rhs = parse_unary(parser)?;
            let loc = expr.loc;
            expr = Expr {
                kind: ExprKind::Binary(crate::ast::BinaryExpr {
                    op: crate::ast::BinOp::Div,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                }),
                loc,
            };
            continue;
        }

        if parser.r#match(TokenKind::Percent) {
            let rhs = parse_unary(parser)?;
            let loc = expr.loc;
            expr = Expr {
                kind: ExprKind::Binary(crate::ast::BinaryExpr {
                    op: crate::ast::BinOp::Mod,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                }),
                loc,
            };
            continue;
        }

        break;
    }

    Ok(expr)
}

// unary: *expr (dereference)
fn parse_unary(parser: &mut Parser) -> Result<Expr, ParseError> {
    if parser.r#match(TokenKind::Star) {
        let inner = parse_unary(parser)?;
        let inner_loc = inner.loc;
        return Ok(Expr {
            kind: ExprKind::Dereference(Box::new(inner)),
            loc: inner_loc,
        });
    }

    parse_primary(parser)
}

fn parse_primary(parser: &mut Parser) -> Result<Expr, ParseError> {
    // number literal
    if parser.check(TokenKind::Number) {
        let num_tok = parser.expect(TokenKind::Number)?;
        let value = num_tok
            .int_value
            .ok_or_else(|| parser.error("Invalid number literal"))?;

        return Ok(Expr {
            kind: ExprKind::Number(value),
            loc: num_tok.loc,
        });
    }

    // parenthesized expression: (expr)
    if parser.r#match(TokenKind::LParen) {
        let e = parse_expr(parser)?;
        expect_token(parser, TokenKind::RParen)?;
        return Ok(e);
    }

    // identifier (variable, method call, or function call)
    let ident_tok = parser.expect(TokenKind::Identifier)?;

    // method call: receiver.method(arg)
    if parser.r#match(TokenKind::Dot) {
        let method_tok = parser.expect(TokenKind::Identifier)?;
        expect_token(parser, TokenKind::LParen)?;
        let arg = parse_expr(parser)?;
        expect_token(parser, TokenKind::RParen)?;

        return Ok(Expr {
            kind: ExprKind::MethodCall(MethodCall {
                receiver: ident_tok.lexeme,
                method: method_tok.lexeme,
                arg: Box::new(arg),
            }),
            loc: ident_tok.loc,
        });
    }
    
    if parser.r#match(TokenKind::LParen) {
        let mut args = Vec::new();

        // parse arguments if any
        if !parser.check(TokenKind::RParen) {
            loop {
                args.push(parse_expr(parser)?);

                if parser.r#match(TokenKind::Comma) {
                    continue;
                } else {
                    break;
                }
            }
        }

        expect_token(parser, TokenKind::RParen)?;
        return Ok(Expr {
            kind: ExprKind::Call(crate::ast::CallExpr {
                name: ident_tok.lexeme,
                args,
            }),
            loc: ident_tok.loc,
        });
    }
    
    Ok(Expr {
        kind: ExprKind::Variable(ident_tok.lexeme),
        loc: ident_tok.loc,
    })
}

fn parse_shift(parser: &mut Parser) -> Result<Expr, ParseError> {
    let mut expr = parse_mul(parser)?; 

    loop {
        if parser.r#match(TokenKind::Shl) {
            let rhs = parse_mul(parser)?;
            let loc = expr.loc;
            expr = Expr {
                kind: ExprKind::Binary(crate::ast::BinaryExpr {
                    op: crate::ast::BinOp::Shl,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                }),
                loc,
            };
            continue;
        }

        if parser.r#match(TokenKind::Shr) {
            let rhs = parse_mul(parser)?;
            let loc = expr.loc;
            expr = Expr {
                kind: ExprKind::Binary(crate::ast::BinaryExpr {
                    op: crate::ast::BinOp::Shr,
                    left: Box::new(expr),
                    right: Box::new(rhs),
                }),
                loc,
            };
            continue;
        }

        break;
    }

    Ok(expr)
}