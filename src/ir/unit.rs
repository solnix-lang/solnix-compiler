use super::{Instruction, VarId};
use crate::ast::unit::ProgramKind;
use crate::ast::{Expr, ExprKind, Stmt, StmtKind, Unit};
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
    next_block_id: u32,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<Instruction>,
    pub terminator: Terminator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

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
    map_ptr_vars: std::collections::HashSet<VarId>,
}

impl UnitIr {
    pub fn lower(
        unit: &Unit,
        events: &std::collections::HashMap<String, u32>,
        event_decls: &std::collections::HashMap<String, crate::ast::EventDecl>,
    ) -> Result<Self, LoweringError> {
        let program_kind = unit.kind;

        let mut ir = Self {
            name: unit.name.clone(),
            sections: unit.sections.clone(),
            license: unit.license.clone().unwrap_or_else(|| "GPL".to_string()),
            blocks: Vec::new(),
            next_var_id: 0,
            program_type: program_kind,
            next_block_id: 0,
        };

        let mut ctx = LowerCtx {
            vars: std::collections::HashMap::new(),
            map_ptr_vars: std::collections::HashSet::new(),
        };

        let entry_id = ir.alloc_block_id();
        let mut current_block = BasicBlock {
            id: entry_id,
            instructions: Vec::new(),
            terminator: Terminator::Return(Operand::Immediate(0)),
        };

        for stmt in &unit.body {
            lower_statement(
                stmt,
                &mut ctx,
                &mut ir,
                &mut current_block,
                events,
                event_decls,
            )?;
        }

        ir.blocks.push(current_block);
        Ok(ir)
    }

    fn alloc_var(&mut self, _var_type: crate::ast::Type) -> VarId {
        let id = VarId(self.next_var_id);
        self.next_var_id += 1;
        id
    }

    fn alloc_block_id(&mut self) -> BlockId {
        let id = BlockId(self.next_block_id);
        self.next_block_id += 1;
        id
    }
}

// ------------------------
// Shared ctx helper lowering
// ------------------------

fn lower_ctx_helper(
    method: CtxMethod,
    ir: &mut UnitIr,
    block: &mut BasicBlock,
) -> Result<Operand, LoweringError> {
    let (helper_id, result_type) = match method {
        CtxMethod::GetPidTgid => (14, crate::ast::Type::U64),
        CtxMethod::GetUidGid => (15, crate::ast::Type::U64),
        CtxMethod::GetCurrentComm => (16, crate::ast::Type::U64),
        CtxMethod::GetCurrentTask => (35, crate::ast::Type::U64),
        CtxMethod::GetKtimeNs => (5, crate::ast::Type::U64),

        // Not helpers - these are handled separately
        CtxMethod::LoadU8
        | CtxMethod::LoadU16
        | CtxMethod::LoadU32
        | CtxMethod::LoadU64
        | CtxMethod::LoadI8
        | CtxMethod::LoadI16
        | CtxMethod::LoadI32
        | CtxMethod::LoadI64
        | CtxMethod::LoadBytes
        | CtxMethod::ProbeReadUserStr
        | CtxMethod::ProbeReadKernelStr => {
            return Err(LoweringError::UnitLowering(format!(
                "Internal error: {:?} is not a simple helper call",
                method
            )));
        }
    };

    let result = ir.alloc_var(result_type);
    block.instructions.push(Instruction {
        result,
        opcode: Opcode::HelperCall { id: helper_id },
        operands: vec![],
        result_type,
    });

    Ok(Operand::Var(result))
}

// ------------------------
// Statements
// ------------------------

fn lower_statement(
    stmt: &Stmt,
    ctx: &mut LowerCtx,
    ir: &mut UnitIr,
    block: &mut BasicBlock,
    events: &std::collections::HashMap<String, u32>,
    event_decls: &std::collections::HashMap<String, crate::ast::EventDecl>,
) -> Result<(), LoweringError> {
    match &stmt.kind {
        StmtKind::VarDecl(var_decl) => {
            let ty: crate::ast::Type = vartype_to_type(&var_decl.var_type)?;
            let var_id = ir.alloc_var(ty.clone());
            let value = lower_expr(&var_decl.value, ctx, ir, block, events, event_decls)?;

            ctx.vars.insert(var_decl.name.clone(), var_id);

            // SSA-style "move"
            block.instructions.push(Instruction {
                result: var_id,
                opcode: Opcode::Binary { op: BinaryOp::Add },
                operands: vec![value, Operand::Immediate(0)],
                result_type: ty,
            });
        }

        StmtKind::Return(expr) => {
            let ret_value = lower_expr(expr, ctx, ir, block, events, event_decls)?;
            block.terminator = Terminator::Return(ret_value);
        }

        StmtKind::HeapVarDecl(heap_decl) => {
            match &heap_decl.init.kind {
                ExprKind::MethodCall(call) => {
                    let receiver_name = if let ExprKind::Variable(name) = &call.receiver.kind {
                        name.clone()
                    } else {
                        return Err(LoweringError::UnitLowering(
                            "heap initializer receiver must be identifier".to_string(),
                        ));
                    };

                    match call.method.as_str() {
                        // ----------------------------
                        // map.lookup(key)
                        // ----------------------------
                        "lookup" => {
                            if call.arg.len() != 1 {
                                return Err(LoweringError::UnitLowering(format!(
                                    "{}.lookup expects 1 argument",
                                    receiver_name
                                )));
                            }

                            let key =
                                lower_expr(&call.arg[0], ctx, ir, block, events, event_decls)?;
                            let result = ir.alloc_var(crate::ast::Type::U64);

                            block.instructions.push(Instruction {
                                result,
                                opcode: Opcode::CallMap {
                                    map_name: receiver_name.clone(),
                                },
                                operands: vec![key],
                                result_type: crate::ast::Type::U64,
                            });

                            ctx.vars.insert(heap_decl.name.clone(), result);
                            ctx.map_ptr_vars.insert(result);
                        }

                        // ----------------------------
                        // map.reserve(event_type)
                        // ----------------------------
                        "reserve" => {
                            if call.arg.len() != 1 {
                                return Err(LoweringError::UnitLowering(format!(
                                    "{}.reserve expects event type name",
                                    receiver_name
                                )));
                            }

                            let event_name = if let ExprKind::Variable(name) = &call.arg[0].kind {
                                name.clone()
                            } else {
                                return Err(LoweringError::UnitLowering(
                                    "reserve requires event type name".to_string(),
                                ));
                            };

                            let size = *events.get(&event_name).ok_or_else(|| {
                                LoweringError::UnitLowering(format!(
                                    "Unknown event type: {}",
                                    event_name
                                ))
                            })?;

                            let result = ir.alloc_var(crate::ast::Type::U64);

                            block.instructions.push(Instruction {
                                result,
                                opcode: Opcode::RingBufReserve {
                                    map_name: receiver_name.clone(),
                                    size,
                                },
                                operands: vec![],
                                result_type: crate::ast::Type::U64,
                            });

                            ctx.vars.insert(heap_decl.name.clone(), result);
                            ctx.map_ptr_vars.insert(result);
                        }

                        _ => {
                            return Err(LoweringError::UnitLowering(format!(
                                "heap var must be initialized with map.lookup or map.reserve"
                            )));
                        }
                    }
                }

                _ => {
                    return Err(LoweringError::UnitLowering(
                        "heap var must be initialized with map.lookup or map.reserve".to_string(),
                    ));
                }
            }
        }

        StmtKind::Assignment(assign) => {
            let value = lower_expr(&assign.value, ctx, ir, block, events, event_decls)?;

            match &assign.target.kind {
                ExprKind::Dereference(ptr_expr) => {
                    let ptr = lower_expr(ptr_expr, ctx, ir, block, events, event_decls)?;

                    let needs_null_check =
                        matches!(ptr, Operand::Var(v) if ctx.map_ptr_vars.contains(&v));

                    if needs_null_check {
                        let check = ir.alloc_var(crate::ast::Type::U64);
                        block.instructions.push(Instruction {
                            result: check,
                            opcode: Opcode::NullCheck,
                            operands: vec![ptr.clone()],
                            result_type: crate::ast::Type::U64,
                        });
                    }

                    let final_value = match assign.op {
                        crate::ast::AssignmentOp::Assign => value,

                        crate::ast::AssignmentOp::AddAssign
                        | crate::ast::AssignmentOp::SubAssign
                        | crate::ast::AssignmentOp::MulAssign
                        | crate::ast::AssignmentOp::DivAssign
                        | crate::ast::AssignmentOp::ModAssign => {
                            // load current
                            let load_result = ir.alloc_var(crate::ast::Type::U64);
                            block.instructions.push(Instruction {
                                result: load_result,
                                opcode: Opcode::LoadKey,
                                operands: vec![ptr.clone()],
                                result_type: crate::ast::Type::U64,
                            });

                            let op = match assign.op {
                                crate::ast::AssignmentOp::AddAssign => BinaryOp::Add,
                                crate::ast::AssignmentOp::SubAssign => BinaryOp::Sub,
                                crate::ast::AssignmentOp::MulAssign => BinaryOp::Mul,
                                crate::ast::AssignmentOp::DivAssign => BinaryOp::Div,
                                crate::ast::AssignmentOp::ModAssign => BinaryOp::Mod,
                                _ => unreachable!(),
                            };

                            let calc = ir.alloc_var(crate::ast::Type::U64);
                            block.instructions.push(Instruction {
                                result: calc,
                                opcode: Opcode::Binary { op },
                                operands: vec![Operand::Var(load_result), value],
                                result_type: crate::ast::Type::U64,
                            });

                            Operand::Var(calc)
                        }
                    };

                    // store back
                    let store_result = ir.alloc_var(crate::ast::Type::U64);
                    block.instructions.push(Instruction {
                        result: store_result,
                        opcode: Opcode::Store { size: 8 },
                        operands: vec![ptr, final_value],
                        result_type: crate::ast::Type::U64,
                    });
                }

                ExprKind::Variable(var_name) => {
                    let old_id = ctx.vars.get(var_name).copied().ok_or_else(|| {
                        LoweringError::UnitLowering(format!("Undefined variable: {var_name}"))
                    })?;

                    let new_id = match assign.op {
                        crate::ast::AssignmentOp::Assign => {
                            let mov = ir.alloc_var(crate::ast::Type::U64);
                            block.instructions.push(Instruction {
                                result: mov,
                                opcode: Opcode::Binary { op: BinaryOp::Add },
                                operands: vec![value, Operand::Immediate(0)],
                                result_type: crate::ast::Type::U64,
                            });
                            mov
                        }
                        crate::ast::AssignmentOp::AddAssign
                        | crate::ast::AssignmentOp::SubAssign
                        | crate::ast::AssignmentOp::MulAssign
                        | crate::ast::AssignmentOp::DivAssign
                        | crate::ast::AssignmentOp::ModAssign => {
                            let op = match assign.op {
                                crate::ast::AssignmentOp::AddAssign => BinaryOp::Add,
                                crate::ast::AssignmentOp::SubAssign => BinaryOp::Sub,
                                crate::ast::AssignmentOp::MulAssign => BinaryOp::Mul,
                                crate::ast::AssignmentOp::DivAssign => BinaryOp::Div,
                                crate::ast::AssignmentOp::ModAssign => BinaryOp::Mod,
                                _ => unreachable!(),
                            };

                            let out = ir.alloc_var(crate::ast::Type::U64);
                            block.instructions.push(Instruction {
                                result: out,
                                opcode: Opcode::Binary { op },
                                operands: vec![Operand::Var(old_id), value],
                                result_type: crate::ast::Type::U64,
                            });
                            out
                        }
                    };

                    ctx.vars.insert(var_name.clone(), new_id);
                }

                ExprKind::FieldAccess { base, field } => {
                    // Handle field assignment like evt.pid = value
                    // First, get the base pointer/variable
                    let base_operand = lower_expr(base, ctx, ir, block, events, event_decls)?;

                    // Get the variable ID (should be a pointer from reserve)
                    let base_var = if let Operand::Var(v) = base_operand {
                        v
                    } else {
                        return Err(LoweringError::UnitLowering(
                            "Field access requires a pointer variable".to_string(),
                        ));
                    };

                    // Look up field offset + size across all events
                    let mut field_offset: Option<u32> = None;
                    let mut field_size: Option<u32> = None;
                    for (_, event_decl) in event_decls {
                        if let Some(offset) =
                            crate::sema::event::compute_field_offset(event_decl, field)
                        {
                            field_offset = Some(offset);
                            field_size = crate::sema::event::compute_field_size(event_decl, field);
                            break;
                        }
                    }

                    let offset = field_offset.ok_or_else(|| {
                        LoweringError::UnitLowering(format!("Unknown field: {}", field))
                    })?;
                    let size = field_size.ok_or_else(|| {
                        LoweringError::UnitLowering(format!("Unknown field size: {}", field))
                    })?;

                    // Compute pointer to the field: base + offset
                    let field_ptr_var = if offset == 0 {
                        base_var
                    } else {
                        let ptr = ir.alloc_var(crate::ast::Type::U64);
                        block.instructions.push(Instruction {
                            result: ptr,
                            opcode: Opcode::Binary { op: BinaryOp::Add },
                            operands: vec![
                                Operand::Var(base_var),
                                Operand::Immediate(offset as i64),
                            ],
                            result_type: crate::ast::Type::U64,
                        });
                        ptr
                    };

                    // support simple field assignments
                    if !matches!(assign.op, crate::ast::AssignmentOp::Assign) {
                        return Err(LoweringError::UnitLowering(
                            "Compound assignment operators not supported on struct fields"
                                .to_string(),
                        ));
                    }

                    // Create a store instruction for the field (correct offset + size)
                    let result = ir.alloc_var(crate::ast::Type::U64);
                    block.instructions.push(Instruction {
                        result,
                        opcode: Opcode::Store { size: (size as u8) },
                        operands: vec![Operand::Var(field_ptr_var), value.clone()],
                        result_type: crate::ast::Type::U64,
                    });
                }

                _ => {
                    return Err(LoweringError::UnitLowering(
                        "Invalid assignment target".to_string(),
                    ));
                }
            }
        }

        StmtKind::IfGuard(if_guard) => {
            let guard_var = match &if_guard.condition.kind {
                ExprKind::Variable(name) => ctx.vars.get(name).copied().ok_or_else(|| {
                    LoweringError::UnitLowering(format!("Undefined variable in guard: {name}"))
                })?,
                _ => {
                    return Err(LoweringError::UnitLowering(
                        "Guard must be a variable".to_string(),
                    ));
                }
            };

            let cond = ir.alloc_var(crate::ast::Type::U64);
            block.instructions.push(Instruction {
                result: cond,
                opcode: Opcode::NullCheck,
                operands: vec![Operand::Var(guard_var)],
                result_type: crate::ast::Type::U64,
            });

            let then_id = ir.alloc_block_id();
            let else_id = ir.alloc_block_id();
            let merge_id = ir.alloc_block_id();

            block.terminator = Terminator::Branch {
                condition: Operand::Var(cond),
                true_block: then_id,
                false_block: else_id,
            };
            let finished = std::mem::replace(
                block,
                BasicBlock {
                    id: merge_id,
                    instructions: Vec::new(),
                    terminator: Terminator::Return(Operand::Immediate(0)),
                },
            );
            ir.blocks.push(finished);

            // THEN block
            let mut tb = BasicBlock {
                id: then_id,
                instructions: Vec::new(),
                terminator: Terminator::Jump(merge_id),
            };
            for s in &if_guard.then_body {
                lower_statement(s, ctx, ir, &mut tb, events, event_decls)?;
            }
            if !matches!(
                tb.terminator,
                Terminator::Return(_) | Terminator::Branch { .. }
            ) {
                tb.terminator = Terminator::Jump(merge_id);
            }
            let mut eb = BasicBlock {
                id: else_id,
                instructions: Vec::new(),
                terminator: Terminator::Jump(merge_id),
            };
            if let Some(else_body) = &if_guard.else_body {
                for s in else_body {
                    lower_statement(s, ctx, ir, &mut eb, events, event_decls)?;
                }
            }
            if !matches!(
                eb.terminator,
                Terminator::Return(_) | Terminator::Branch { .. }
            ) {
                eb.terminator = Terminator::Jump(merge_id);
            }

            ir.blocks.push(tb);
            ir.blocks.push(eb);
        }

        StmtKind::ExprStmt(expr) => {
            let _ = lower_expr(expr, ctx, ir, block, events, event_decls)?;
        }
    }
    Ok(())
}

fn lower_expr(
    expr: &Expr,
    ctx: &mut LowerCtx,
    ir: &mut UnitIr,
    block: &mut BasicBlock,
    events: &std::collections::HashMap<String, u32>,
    event_decls: &std::collections::HashMap<String, crate::ast::EventDecl>,
) -> Result<Operand, LoweringError> {
    match &expr.kind {
        ExprKind::Variable(name) => {
            let v = ctx.vars.get(name).copied().ok_or_else(|| {
                LoweringError::UnitLowering(format!("Undefined variable: {name}"))
            })?;
            Ok(Operand::Var(v))
        }

        ExprKind::Number(n) => Ok(Operand::Immediate(*n)),

        ExprKind::HeapLookup(hl) => {
            let key = lower_expr(&hl.key_expr, ctx, ir, block, events, event_decls)?;
            let result = ir.alloc_var(crate::ast::Type::U64);

            block.instructions.push(Instruction {
                result,
                opcode: Opcode::CallMap {
                    map_name: hl.map_name.clone(),
                },
                operands: vec![key],
                result_type: crate::ast::Type::U64,
            });

            ctx.map_ptr_vars.insert(result);
            Ok(Operand::Var(result))
        }

        ExprKind::MethodCall(call) => {
            let receiver_name = if let ExprKind::Variable(name) = &call.receiver.kind {
                name.clone()
            } else {
                return Err(LoweringError::UnitLowering(
                    "Method receiver must be identifier".to_string(),
                ));
            };

            if receiver_name == "ctx" {
                let method = CtxMethod::from_str(&call.method).ok_or_else(|| {
                    LoweringError::UnitLowering(format!("Unknown ctx method: {}", call.method))
                })?;

                if !ir.program_type.allows_ctx_method(method) {
                    return Err(LoweringError::UnitLowering(format!(
                        "ctx method {:?} not allowed in {:?}",
                        method, ir.program_type
                    )));
                }
                return match method {
                    // ctx.load_*(offset)
                    CtxMethod::LoadU8
                    | CtxMethod::LoadU16
                    | CtxMethod::LoadU32
                    | CtxMethod::LoadU64
                    | CtxMethod::LoadI8
                    | CtxMethod::LoadI16
                    | CtxMethod::LoadI32
                    | CtxMethod::LoadI64 => {
                        if call.arg.len() != 1 {
                            return Err(LoweringError::UnitLowering(format!(
                                "ctx method {} expects 1 argument, got {}",
                                call.method,
                                call.arg.len()
                            )));
                        }

                        let offset_expr =
                            lower_expr(&call.arg[0], ctx, ir, block, events, event_decls)?;
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

                    // ctx.probe_read_user_str(dest, size, src)
                    CtxMethod::ProbeReadUserStr => {
                        if call.arg.len() != 3 {
                            return Err(LoweringError::UnitLowering(format!(
                                "ctx.probe_read_user_str expects 3 arguments, got {}",
                                call.arg.len()
                            )));
                        }

                        let dest = lower_expr(&call.arg[0], ctx, ir, block, events, event_decls)?;
                        let size_expr =
                            lower_expr(&call.arg[1], ctx, ir, block, events, event_decls)?;
                        let src = lower_expr(&call.arg[2], ctx, ir, block, events, event_decls)?;

                        let size = match size_expr {
                            Operand::Immediate(n) => n as u32,
                            _ => {
                                return Err(LoweringError::UnitLowering(
                                    "probe_read_user_str size must be immediate".to_string(),
                                ))
                            }
                        };

                        let result = ir.alloc_var(crate::ast::Type::U64);
                        block.instructions.push(Instruction {
                            result,
                            opcode: Opcode::HelperCall { id: 202 },
                            operands: vec![dest, Operand::Immediate(size as i64), src],
                            result_type: crate::ast::Type::U64,
                        });

                        Ok(Operand::Var(result))
                    }

                    // ctx.probe_read_kernel_str(dest, size, src)
                    CtxMethod::ProbeReadKernelStr => {
                        if call.arg.len() != 3 {
                            return Err(LoweringError::UnitLowering(format!(
                                "ctx.probe_read_kernel_str expects 3 arguments, got {}",
                                call.arg.len()
                            )));
                        }

                        let dest = lower_expr(&call.arg[0], ctx, ir, block, events, event_decls)?;
                        let size_expr =
                            lower_expr(&call.arg[1], ctx, ir, block, events, event_decls)?;
                        let src = lower_expr(&call.arg[2], ctx, ir, block, events, event_decls)?;

                        let size = match size_expr {
                            Operand::Immediate(n) => n as u32,
                            _ => {
                                return Err(LoweringError::UnitLowering(
                                    "probe_read_kernel_str size must be immediate".to_string(),
                                ))
                            }
                        };

                        let result = ir.alloc_var(crate::ast::Type::U64);
                        block.instructions.push(Instruction {
                            result,
                            opcode: Opcode::HelperCall { id: 204 },
                            operands: vec![dest, Operand::Immediate(size as i64), src],
                            result_type: crate::ast::Type::U64,
                        });

                        Ok(Operand::Var(result))
                    }

                    CtxMethod::LoadBytes => {
                        if call.arg.len() != 3 {
                            return Err(LoweringError::UnitLowering(
                                "ctx.load_bytes expects (offset, dest, size)".to_string(),
                            ));
                        }

                        let offset_expr =
                            lower_expr(&call.arg[0], ctx, ir, block, events, event_decls)?;
                        let dest = lower_expr(&call.arg[1], ctx, ir, block, events, event_decls)?;
                        let size_expr =
                            lower_expr(&call.arg[2], ctx, ir, block, events, event_decls)?;

                        let offset = match offset_expr {
                            Operand::Immediate(n) => n as i32,
                            _ => {
                                return Err(LoweringError::UnitLowering(
                                    "offset must be immediate".to_string(),
                                ))
                            }
                        };

                        let size = match size_expr {
                            Operand::Immediate(n) => n as u32,
                            _ => {
                                return Err(LoweringError::UnitLowering(
                                    "size must be immediate".to_string(),
                                ))
                            }
                        };

                        block.instructions.push(Instruction {
                            result: ir.alloc_var(crate::ast::Type::U64),
                            opcode: Opcode::CopyCtxToMem { offset, size },
                            operands: vec![dest.clone()],
                            result_type: crate::ast::Type::U64,
                        });

                        Ok(dest)
                    }
                    // ctx helper methods (0 args)
                    _ => {
                        if !call.arg.is_empty() {
                            return Err(LoweringError::UnitLowering(format!(
                                "ctx method {} expects 0 arguments, got {}",
                                call.method,
                                call.arg.len()
                            )));
                        }
                        lower_ctx_helper(method, ir, block)
                    }
                };
            }

            // ----------------------------
            // map.<method>()
            // ----------------------------
            match call.method.as_str() {
                "lookup" => {
                    if call.arg.len() != 1 {
                        return Err(LoweringError::UnitLowering(format!(
                            "{}.lookup expects 1 argument, got {}",
                            receiver_name,
                            call.arg.len()
                        )));
                    }

                    let key = lower_expr(&call.arg[0], ctx, ir, block, events, event_decls)?;
                    let result = ir.alloc_var(crate::ast::Type::U64);

                    block.instructions.push(Instruction {
                        result,
                        opcode: Opcode::CallMap {
                            map_name: receiver_name.clone(),
                        },
                        operands: vec![key],
                        result_type: crate::ast::Type::U64,
                    });

                    ctx.map_ptr_vars.insert(result);

                    Ok(Operand::Var(result))
                }

                "update" => {
                    if call.arg.len() != 2 {
                        return Err(LoweringError::UnitLowering(format!(
                            "{}.update expects 2 arguments, got {}",
                            receiver_name,
                            call.arg.len()
                        )));
                    }

                    let key = lower_expr(&call.arg[0], ctx, ir, block, events, event_decls)?;
                    let value = lower_expr(&call.arg[1], ctx, ir, block, events, event_decls)?;

                    let result = ir.alloc_var(crate::ast::Type::U64);

                    block.instructions.push(Instruction {
                        result,
                        opcode: Opcode::UpdateMap {
                            map_name: receiver_name.clone(),
                        },
                        operands: vec![key, value],
                        result_type: crate::ast::Type::U64,
                    });

                    Ok(Operand::Var(result))
                }

                "reserve" => {
                    if call.arg.len() != 1 {
                        return Err(LoweringError::UnitLowering(format!(
                            "{}.reserve expects 1 argument (event type)",
                            receiver_name
                        )));
                    }

                    let event_name = if let ExprKind::Variable(name) = &call.arg[0].kind {
                        name.clone()
                    } else {
                        return Err(LoweringError::UnitLowering(
                            "reserve requires event type name".to_string(),
                        ));
                    };

                    let size = *events.get(&event_name).ok_or_else(|| {
                        LoweringError::UnitLowering(format!("Unknown event type: {}", event_name))
                    })?;

                    let result = ir.alloc_var(crate::ast::Type::U64);

                    block.instructions.push(Instruction {
                        result,
                        opcode: Opcode::RingBufReserve {
                            map_name: receiver_name.clone(),
                            size: size,
                        },
                        operands: vec![],
                        result_type: crate::ast::Type::U64,
                    });

                    ctx.map_ptr_vars.insert(result); // mark as nullable pointer

                    Ok(Operand::Var(result))
                }

                "submit" => {
                    if call.arg.len() != 1 {
                        return Err(LoweringError::UnitLowering(format!(
                            "{}.submit expects 1 argument",
                            receiver_name
                        )));
                    }

                    let ptr = lower_expr(&call.arg[0], ctx, ir, block, events, event_decls)?;

                    let result = ir.alloc_var(crate::ast::Type::U64);

                    block.instructions.push(Instruction {
                        result,
                        opcode: Opcode::RingBufSubmit {
                            map_name: receiver_name.clone(),
                        },
                        operands: vec![ptr],
                        result_type: crate::ast::Type::U64,
                    });

                    Ok(Operand::Var(result))
                }
                _ => Err(LoweringError::UnitLowering(format!(
                    "Unknown method: {}.{}",
                    receiver_name, call.method
                ))),
            }
        }

        ExprKind::Dereference(ptr_expr) => {
            let ptr = lower_expr(ptr_expr, ctx, ir, block, events, event_decls)?;
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
            let left = lower_expr(&bin.left, ctx, ir, block, events, event_decls)?;
            let right = lower_expr(&bin.right, ctx, ir, block, events, event_decls)?;

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
        ExprKind::Call(call) => {
            let method = CtxMethod::from_str(&call.name).ok_or_else(|| {
                LoweringError::UnitLowering(format!("Unknown builtin function: {}", call.name))
            })?;

            if !ir.program_type.allows_ctx_method(method) {
                return Err(LoweringError::UnitLowering(format!(
                    "ctx method {:?} not allowed in {:?}",
                    method, ir.program_type
                )));
            }

            if !call.args.is_empty() {
                return Err(LoweringError::UnitLowering(format!(
                    "{} expects 0 arguments, got {}",
                    call.name,
                    call.args.len()
                )));
            }

            match method {
                CtxMethod::GetPidTgid
                | CtxMethod::GetUidGid
                | CtxMethod::GetCurrentComm
                | CtxMethod::GetCurrentTask
                | CtxMethod::GetKtimeNs => lower_ctx_helper(method, ir, block),

                _ => Err(LoweringError::UnitLowering(format!(
                    "Use ctx.{}(offset) for context loads",
                    call.name
                ))),
            }
        }

        ExprKind::FieldAccess { base, field } => {
            // Handle field access like evt.filename (returns pointer to the field)
            let base_operand = lower_expr(base, ctx, ir, block, events, event_decls)?;

            // Get the variable ID (should be a pointer from reserve)
            let base_var = if let Operand::Var(v) = base_operand {
                v
            } else {
                return Err(LoweringError::UnitLowering(
                    "Field access requires a pointer variable".to_string(),
                ));
            };

            // Look up field offset across all events
            let mut field_offset: Option<u32> = None;
            for (_, event_decl) in event_decls {
                if let Some(offset) = crate::sema::event::compute_field_offset(event_decl, field) {
                    field_offset = Some(offset);
                    break;
                }
            }

            let offset = field_offset
                .ok_or_else(|| LoweringError::UnitLowering(format!("Unknown field: {}", field)))?;

            if offset == 0 {
                Ok(Operand::Var(base_var))
            } else {
                let result = ir.alloc_var(crate::ast::Type::U64);
                block.instructions.push(Instruction {
                    result,
                    opcode: Opcode::Binary { op: BinaryOp::Add },
                    operands: vec![Operand::Var(base_var), Operand::Immediate(offset as i64)],
                    result_type: crate::ast::Type::U64,
                });
                Ok(Operand::Var(result))
            }
        }

        other => Err(LoweringError::UnitLowering(format!(
            "InvalidOperand: unsupported expr kind: {other:?}"
        ))),
    }
}

fn vartype_to_type(vt: &crate::ast::VarType) -> Result<crate::ast::Type, LoweringError> {
    use crate::ast::{Type, VarType};

    match vt {
        VarType::Reg => Ok(Type::U64),
        VarType::Imm => Ok(Type::U64),
    }
}
