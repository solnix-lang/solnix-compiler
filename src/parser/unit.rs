use super::{ParseError, Parser};
use crate::{
    ast::{
        Assignment, AssignmentOp, Expr, ExprKind, HeapVarDecl, IfGuard, MethodCall, Stmt, StmtKind,
        Unit, VarDecl, VarType,
    },
    parser::{map::expect_token, TokenKind},
};
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

    let kind = sections
        .first()
        .map(|s| crate::ast::unit::ProgramKind::from_section(s))
        .unwrap_or(crate::ast::unit::ProgramKind::Unknown);

    Ok(Unit {
        name: name_tok.lexeme,
        loc: unit_loc,
        sections,
        kind,
        license,
        events: Vec::new(), 
        body,
    })
}

fn parse_stmt(parser: &mut Parser, body: &mut Vec<Stmt>) -> Result<(), ParseError> {
    // reg x = expr;
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

    // imm x = expr;
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

    // heap x = expr;   (ex: heap v = map.lookup(k);)
    if parser.r#match(TokenKind::KeywordHeap) {
        let var_loc = parser.current_loc();
        let var_name_tok = parser.expect(TokenKind::Identifier)?;
        expect_token(parser, TokenKind::Equals)?;

        let init = parse_expr(parser)?;
        expect_token(parser, TokenKind::Semicolon)?;

        body.push(Stmt {
            kind: StmtKind::HeapVarDecl(HeapVarDecl {
                name: var_name_tok.lexeme,
                init,
            }),
            loc: var_loc,
        });
        return Ok(());
    }

    // return expr;
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

    // if guard(x) { ... } [else { ... }]
    if parser.r#match(TokenKind::KeywordIf) {
        let if_loc = parser.current_loc();
        expect_token(parser, TokenKind::KeywordGuard)?;
        expect_token(parser, TokenKind::LParen)?;
        let var_tok = parser.expect(TokenKind::Identifier)?;
        expect_token(parser, TokenKind::RParen)?;
        expect_token(parser, TokenKind::LBrace)?;

        // THEN body
        let mut then_body = Vec::new();
        while !parser.r#match(TokenKind::RBrace) {
            parse_stmt(parser, &mut then_body)?;
        }

        // ELSE body (optional)
        let else_body = if parser.r#match(TokenKind::KeywordElse) {
            expect_token(parser, TokenKind::LBrace)?;
            let mut eb = Vec::new();
            while !parser.r#match(TokenKind::RBrace) {
                parse_stmt(parser, &mut eb)?;
            }
            Some(eb)
        } else {
            None
        };

        body.push(Stmt {
            kind: StmtKind::IfGuard(IfGuard {
                condition: Expr {
                    kind: ExprKind::Variable(var_tok.lexeme.clone()),
                    loc: var_tok.loc,
                },
                then_body,
                else_body,
            }),
            loc: if_loc,
        });

        return Ok(());
    }

    // assignment OR expr-statement
    let target = parse_expr(parser)?;
    let target_loc = target.loc;

    // target = expr;
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
    }

    // target += expr;
    if parser.r#match(TokenKind::PlusEquals) {
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

    // NEW: expression statement: expr;
    // Example: map.update(k, v);  map.delete(k);
    expect_token(parser, TokenKind::Semicolon)?;
    body.push(Stmt {
        kind: StmtKind::ExprStmt(Box::new(target)),
        loc: target_loc,
    });
    Ok(())
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
    let mut expr;

    // number
    if parser.check(TokenKind::Number) {
        let num_tok = parser.expect(TokenKind::Number)?;
        let value = num_tok
            .int_value
            .ok_or_else(|| parser.error("Invalid number literal"))?;

        expr = Expr {
            kind: ExprKind::Number(value),
            loc: num_tok.loc,
        };
    }
    // (expr)
    else if parser.r#match(TokenKind::LParen) {
        expr = parse_expr(parser)?;
        expect_token(parser, TokenKind::RParen)?;
    }
    // identifier
    else {
        let ident_tok = parser.expect(TokenKind::Identifier)?;
        expr = Expr {
            kind: ExprKind::Variable(ident_tok.lexeme),
            loc: ident_tok.loc,
        };
    }

    // POSTFIX LOOP (supports chaining)
    loop {
        // field access OR method call
        if parser.r#match(TokenKind::Dot) {
            let field_tok = parser.expect(TokenKind::Identifier)?;

            // method call
            if parser.r#match(TokenKind::LParen) {
                let mut args = Vec::new();
                if !parser.check(TokenKind::RParen) {
                    loop {
                        args.push(parse_expr(parser)?);
                        if parser.r#match(TokenKind::Comma) {
                            continue;
                        }
                        break;
                    }
                }
                expect_token(parser, TokenKind::RParen)?;

                expr = Expr {
                    kind: ExprKind::MethodCall(MethodCall {
                        receiver: match expr.kind {
                            ExprKind::Variable(ref name) => Box::new(Expr {
                                kind: ExprKind::Variable(name.clone()),
                                loc: expr.loc,
                            }),
                            _ => panic!("Unsupported method receiver"),
                        },
                        method: field_tok.lexeme,
                        arg: args,
                    }),
                    loc: expr.loc,
                };
            }
            // field access
            else {
                expr = Expr {
                    kind: ExprKind::FieldAccess {
                        base: Box::new(expr),
                        field: field_tok.lexeme,
                    },
                    loc: field_tok.loc,
                };
            }

            continue;
        }

        break;
    }

    Ok(expr)
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
