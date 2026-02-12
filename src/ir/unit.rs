use super::{Instruction, VarId};
use crate::ast::{Expr, ExprKind, Stmt, StmtKind, Unit};
use crate::ast::unit::ProgramKind;
use crate::ir::ctx::CtxMethod;
use crate::ir::{BinaryOp, LoweringError, Opcode, Operand};

#[derive(Debug, Clone)]
pub struct UnitIr {
    pub name: String,
    pub sections: Vec<String>,
    pub license: String,
    pub blocks: Vec<BasicBlock>,
    pub next_var_id: u32,
    pub program_type: ProgramKind,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Terminator {
    Return(Operand),
    Jump(BlockId),
    Branch {
        condition: Operand,
        true_block: BlockId,
        false_block: BlockId,
    },
}

struct LowerCtx {
    vars: std::collections::HashMap<String, VarId>,
    // Track which variables are map pointers (from CallMap operations)
    map_ptr_vars: std::collections::HashSet<VarId>,
}

impl UnitIr {
    pub fn lower(unit: &Unit) -> Result<Self, LoweringError> {
        let program_kind = unit.kind;

        let mut ir = Self {
            name: unit.name.clone(),
            sections: unit.sections.clone(),
            license: unit.license.clone().unwrap_or_else(|| "GPL".to_string()),
            blocks: Vec::new(),
            next_var_id: 0,
            program_type: program_kind,
        };

        let mut ctx = LowerCtx {
            vars: std::collections::HashMap::new(),
            map_ptr_vars: std::collections::HashSet::new(),
        };

        let mut current_block = BasicBlock {
            id: BlockId(0),
            instructions: Vec::new(),
            terminator: Terminator::Return(Operand::Immediate(0)),
        };

        for stmt in &unit.body {
            lower_statement(stmt, &mut ctx, &mut ir, &mut current_block)?;
        }

        ir.blocks.push(current_block);
        Ok(ir)
    }

    fn alloc_var(&mut self, _var_type: crate::ast::Type) -> VarId {
        let id = VarId(self.next_var_id);
        self.next_var_id += 1;
        id
    }
}

fn lower_statement(
    stmt: &Stmt,
    ctx: &mut LowerCtx,
    ir: &mut UnitIr,
    block: &mut BasicBlock,
) -> Result<(), LoweringError> {
    match &stmt.kind {
        StmtKind::VarDecl(var_decl) => {
            let ty: crate::ast::Type = vartype_to_type(&var_decl.var_type)?;

            let var_id = ir.alloc_var(ty.clone());
            let value = lower_expr(&var_decl.value, ctx, ir, block)?;

            ctx.vars.insert(var_decl.name.clone(), var_id);

            block.instructions.push(Instruction {
                result: var_id,
                opcode: Opcode::Binary { op: BinaryOp::Add },
                operands: vec![value, Operand::Immediate(0)],
                result_type: ty,
            });
        }

        StmtKind::Return(expr) => {
            let ret_value = lower_expr(expr, ctx, ir, block)?;
            block.terminator = Terminator::Return(ret_value);
        }

        StmtKind::HeapVarDecl(heap_decl) => {
            let key = lower_expr(&heap_decl.lookup.key_expr, ctx, ir, block)?;
            let result = ir.alloc_var(crate::ast::Type::U64);

            block.instructions.push(Instruction {
                result,
                opcode: Opcode::CallMap {
                    map_name: heap_decl.lookup.map_name.clone(),
                },
                operands: vec![key],
                result_type: crate::ast::Type::U64,
            });

            ctx.vars.insert(heap_decl.name.clone(), result);
            // Mark this variable as coming from a map lookup
            ctx.map_ptr_vars.insert(result);
        }

        StmtKind::Assignment(assign) => {
            let value = lower_expr(&assign.value, ctx, ir, block)?;

            match &assign.target.kind {
                ExprKind::Dereference(ptr_expr) => {
                    let ptr = lower_expr(ptr_expr, ctx, ir, block)?;

                    // If the pointer is a map pointer, we need to insert a null check
                    let needs_null_check = if let Operand::Var(ptr_var) = ptr {
                        ctx.map_ptr_vars.contains(&ptr_var)
                    } else {
                        false
                    };

                    if needs_null_check {
                        // Emit a NullCheck instruction
                        let null_check_result = ir.alloc_var(crate::ast::Type::U64);
                        block.instructions.push(Instruction {
                            result: null_check_result,
                            opcode: Opcode::NullCheck,
                            operands: vec![ptr.clone()],
                            result_type: crate::ast::Type::U64,
                        });
                    }

                    // For +=, we need to load the current value, add, then store
                    let final_value = if assign.op == crate::ast::AssignmentOp::AddAssign {
                        // Load current value
                        let load_result = ir.alloc_var(crate::ast::Type::U64);
                        block.instructions.push(Instruction {
                            result: load_result,
                            opcode: Opcode::LoadKey,
                            operands: vec![ptr.clone()],
                            result_type: crate::ast::Type::U64,
                        });

                        // Add the new value to it
                        let add_result = ir.alloc_var(crate::ast::Type::U64);
                        block.instructions.push(Instruction {
                            result: add_result,
                            opcode: Opcode::Binary { op: BinaryOp::Add },
                            operands: vec![Operand::Var(load_result), value],
                            result_type: crate::ast::Type::U64,
                        });

                        Operand::Var(add_result)
                    } else {
                        value
                    };

                    let _result = ir.alloc_var(crate::ast::Type::U64);

                    block.instructions.push(Instruction {
                        result: _result,
                        opcode: Opcode::Store { size: 8 },
                        operands: vec![ptr, final_value],
                        result_type: crate::ast::Type::U64,
                    });
                }
                ExprKind::Variable(var_name) => {
                    if ctx.vars.contains_key(var_name) {
                        let var_id = ctx.vars.get(var_name).copied().unwrap();

                        let final_value = if assign.op == crate::ast::AssignmentOp::AddAssign {
                            let add_result = ir.alloc_var(crate::ast::Type::U64);
                            block.instructions.push(Instruction {
                                result: add_result,
                                opcode: Opcode::Binary { op: BinaryOp::Add },
                                operands: vec![Operand::Var(var_id), value],
                                result_type: crate::ast::Type::U64,
                            });
                            add_result
                        } else {
                            let result = ir.alloc_var(crate::ast::Type::U64);
                            block.instructions.push(Instruction {
                                result,
                                opcode: Opcode::Binary { op: BinaryOp::Add },
                                operands: vec![value, Operand::Immediate(0)],
                                result_type: crate::ast::Type::U64,
                            });
                            result
                        };

                        ctx.vars.insert(var_name.clone(), final_value);
                    } else {
                        return Err(LoweringError::UnitLowering(format!(
                            "Undefined variable: {var_name}"
                        )));
                    }
                }
                _ => {
                    return Err(LoweringError::UnitLowering(
                        "Invalid assignment target".to_string(),
                    ));
                }
            }
        }

        StmtKind::IfGuard(if_guard) => {
            let guard_expr = &if_guard.condition;
            let guard_var = match &guard_expr.kind {
                ExprKind::Variable(name) => ctx.vars.get(name).copied().ok_or_else(|| {
                    LoweringError::UnitLowering(format!("Undefined variable in guard: {name}"))
                })?,
                _ => {
                    return Err(LoweringError::UnitLowering(
                        "Guard must be a variable".to_string(),
                    ));
                }
            };

            let null_check_result = ir.alloc_var(crate::ast::Type::U64);
            block.instructions.push(Instruction {
                result: null_check_result,
                opcode: Opcode::NullCheck,
                operands: vec![Operand::Var(guard_var)],
                result_type: crate::ast::Type::U64,
            });

            let true_block_id = BlockId(ir.blocks.len() as u32);
            let false_block_id = BlockId(ir.blocks.len() as u32 + 1);
            let merge_block_id = BlockId(ir.blocks.len() as u32 + 2);

            block.terminator = Terminator::Branch {
                condition: Operand::Var(null_check_result),
                true_block: true_block_id,
                false_block: false_block_id,
            };
            let mut true_block = BasicBlock {
                id: true_block_id,
                instructions: Vec::new(),
                terminator: Terminator::Jump(merge_block_id),
            };

            for stmt in &if_guard.body {
                lower_statement(stmt, ctx, ir, &mut true_block)?;
            }

            let false_block = BasicBlock {
                id: false_block_id,
                instructions: Vec::new(),
                terminator: Terminator::Jump(merge_block_id),
            };

            let merge_block = BasicBlock {
                id: merge_block_id,
                instructions: Vec::new(),
                terminator: Terminator::Return(Operand::Immediate(0)),
            };

            ir.blocks.push(true_block);
            ir.blocks.push(false_block);
            ir.blocks.push(merge_block.clone());

            *block = merge_block;
        }
    }
    Ok(())
}

fn lower_expr(
    expr: &Expr,
    ctx: &mut LowerCtx,
    ir: &mut UnitIr,
    block: &mut BasicBlock,
) -> Result<Operand, LoweringError> {
    match &expr.kind {
        ExprKind::Variable(name) => {
            let v = ctx.vars.get(name).copied().ok_or_else(|| {
                LoweringError::UnitLowering(format!("Undefined variable: {name}"))
            })?;
            Ok(Operand::Var(v))
        }

        ExprKind::Number(n) => Ok(Operand::Immediate(*n)),

        ExprKind::MethodCall(call) => {
            if call.receiver == "ctx" {
                let method = CtxMethod::from_str(&call.method).ok_or_else(|| {
                    LoweringError::UnitLowering(format!("Unknown ctx method: {}", call.method))
                })?;

                if !ir.program_type.allows_ctx_method(method) {
                    return Err(LoweringError::UnitLowering(format!(
                        "ctx method {:?} not allowed in {:?}",
                        method, ir.program_type
                    )));
                }

                match method {
                    CtxMethod::LoadU8
                    | CtxMethod::LoadU16
                    | CtxMethod::LoadU32
                    | CtxMethod::LoadU64
                    | CtxMethod::LoadI8
                    | CtxMethod::LoadI16
                    | CtxMethod::LoadI32
                    | CtxMethod::LoadI64 => {
                        let arg = call.arg.as_ref().ok_or_else(|| {
                            LoweringError::UnitLowering(format!(
                                "ctx method {} requires an argument",
                                call.method
                            ))
                        })?;
                        let offset_expr = lower_expr(arg, ctx, ir, block)?;
                        let offset = match offset_expr {
                            Operand::Immediate(n) => n as i32,
                            _ => {
                                return Err(LoweringError::UnitLowering(
                                    "Context load offset must be immediate".to_string(),
                                ))
                            }
                        };

                        let (size, result_type) = match method {
                            CtxMethod::LoadU8 => (1, crate::ast::Type::U32),
                            CtxMethod::LoadU16 => (2, crate::ast::Type::U32),
                            CtxMethod::LoadU32 => (4, crate::ast::Type::U32),
                            CtxMethod::LoadU64 => (8, crate::ast::Type::U64),
                            CtxMethod::LoadI8 => (1, crate::ast::Type::I32),
                            CtxMethod::LoadI16 => (2, crate::ast::Type::I32),
                            CtxMethod::LoadI32 => (4, crate::ast::Type::I32),
                            CtxMethod::LoadI64 => (8, crate::ast::Type::I64),
                            _ => unreachable!(),
                        };

                        let result = ir.alloc_var(result_type);

                        let opcode = if offset >= 0 {
                            Opcode::LoadPacket { offset, size }
                        } else {
                            Opcode::LoadCtx { offset, size }
                        };

                        block.instructions.push(Instruction {
                            result,
                            opcode,
                            operands: vec![],
                            result_type,
                        });

                        Ok(Operand::Var(result))
                    }

                    CtxMethod::GetUidGid => {
                        let result = ir.alloc_var(crate::ast::Type::U32);

                        block.instructions.push(Instruction {
                            result,
                            opcode: Opcode::HelperCall { id: 102 }, // example helper id
                            operands: vec![],
                            result_type: crate::ast::Type::U32,
                        });

                        Ok(Operand::Var(result))
                    }

                    CtxMethod::GetPidTgid => {
                        let result = ir.alloc_var(crate::ast::Type::U64);

                        block.instructions.push(Instruction {
                            result,
                            opcode: Opcode::HelperCall { id: 14 }, // bpf_get_current_pid_tgid
                            operands: vec![],
                            result_type: crate::ast::Type::U64,
                        });

                        Ok(Operand::Var(result))
                    }

                    CtxMethod::GetCurrentComm => {
                        let result = ir.alloc_var(crate::ast::Type::U64);

                        block.instructions.push(Instruction {
                            result,
                            opcode: Opcode::HelperCall { id: 16 }, // bpf_get_current_comm
                            operands: vec![],
                            result_type: crate::ast::Type::U64,
                        });

                        Ok(Operand::Var(result))
                    }

                    CtxMethod::GetCurrentTask => {
                        let result = ir.alloc_var(crate::ast::Type::U64);

                        block.instructions.push(Instruction {
                            result,
                            opcode: Opcode::HelperCall { id: 35 }, // bpf_get_current_task
                            operands: vec![],
                            result_type: crate::ast::Type::U64,
                        });

                        Ok(Operand::Var(result))
                    }

                    CtxMethod::GetKtimeNs => {
                        let result = ir.alloc_var(crate::ast::Type::U64);

                        block.instructions.push(Instruction {
                            result,
                            opcode: Opcode::HelperCall { id: 5 }, // bpf_ktime_get_ns
                            operands: vec![],
                            result_type: crate::ast::Type::U64,
                        });

                        Ok(Operand::Var(result))
                    }
                }
            } else {
                Err(LoweringError::UnitLowering(format!(
                    "Unknown method: {}.{}",
                    call.receiver, call.method
                )))
            }
        }

        ExprKind::Dereference(ptr_expr) => {
            let ptr = lower_expr(ptr_expr, ctx, ir, block)?;
            let result = ir.alloc_var(crate::ast::Type::U64);

            block.instructions.push(Instruction {
                result,
                opcode: Opcode::LoadKey,
                operands: vec![ptr],
                result_type: crate::ast::Type::U64,
            });

            Ok(Operand::Var(result))
        }

        ExprKind::Binary(bin) => {
            let left = lower_expr(&bin.left, ctx, ir, block)?;
            let right = lower_expr(&bin.right, ctx, ir, block)?;

            let result = ir.alloc_var(crate::ast::Type::U64);

            let op = match bin.op {
                crate::ast::BinOp::Add => BinaryOp::Add,
                crate::ast::BinOp::Sub => BinaryOp::Sub,
                crate::ast::BinOp::Mul => BinaryOp::Mul,
                crate::ast::BinOp::Div => BinaryOp::Div,
                crate::ast::BinOp::Mod => BinaryOp::Mod,
                crate::ast::BinOp::Shl => BinaryOp::Shl,
                crate::ast::BinOp::Shr => BinaryOp::Shr,
            };

            block.instructions.push(Instruction {
                result,
                opcode: Opcode::Binary { op },
                operands: vec![left, right],
                result_type: crate::ast::Type::U64,
            });

            Ok(Operand::Var(result))
        }

        _ => Err(LoweringError::InvalidOperand),
    }
}

fn vartype_to_type(vt: &crate::ast::VarType) -> Result<crate::ast::Type, LoweringError> {
    use crate::ast::{Type, VarType};

    match vt {
        VarType::Reg => Ok(Type::U64),
        VarType::Imm => Ok(Type::U64),
    }
}
