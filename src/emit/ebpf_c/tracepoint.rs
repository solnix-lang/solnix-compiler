use std::collections::{HashMap, HashSet};
use std::fmt::Write;

use crate::ir::{BinaryOp, Opcode, Operand, UnitIr, VarId};

pub fn emit_tracepoint(out: &mut String, unit: &UnitIr, sec: &str) -> Result<(), String> {
    emit_license(out, &unit.license)?;

    writeln!(out, "SEC(\"{}\")", sec).map_err(err)?;
    writeln!(out, "int {}(void *ctx) {{", unit.name).map_err(err)?;
    writeln!(out, "    (void)ctx;").map_err(err)?;
    writeln!(out).map_err(err)?;
    
    let mut used_vars: HashSet<u32> = HashSet::new();

    for block in &unit.blocks {
        for inst in &block.instructions {
            for op in &inst.operands {
                if let Operand::Var(VarId(id)) = op {
                    used_vars.insert(*id);
                }
            }
        }

        match &block.terminator {
            crate::ir::unit::Terminator::Return(op) => {
                if let Operand::Var(VarId(id)) = op {
                    used_vars.insert(*id);
                }
            }
            crate::ir::unit::Terminator::Jump(_) => {}
            crate::ir::unit::Terminator::Branch { condition, .. } => {
                if let Operand::Var(VarId(id)) = condition {
                    used_vars.insert(*id);
                }
            }
        }
    }

    
    let mut branch_cond_vars: HashSet<u32> = HashSet::new();
    for block in &unit.blocks {
        if let crate::ir::unit::Terminator::Branch { condition, .. } = &block.terminator {
            if let Operand::Var(VarId(id)) = condition {
                branch_cond_vars.insert(*id);
            }
        }
    }
    
    let mut var_types: HashMap<u32, crate::ast::Type> = HashMap::new();
    for block in &unit.blocks {
        for inst in &block.instructions {
            var_types.entry(inst.result.0).or_insert(inst.result_type);
        }
    }
    
    let mut pointer_vars: HashSet<u32> = HashSet::new();
    for block in &unit.blocks {
        for inst in &block.instructions {
            if matches!(inst.opcode, Opcode::CallMap { .. }) && used_vars.contains(&inst.result.0) {
                pointer_vars.insert(inst.result.0);
            }
        }
    }
    
    let mut vars_sorted: Vec<u32> = used_vars.iter().copied().collect();
    vars_sorted.sort_unstable();
    
    let mut temp_counter: u32 = 0;
    let mut temps: Vec<(String, String)> = Vec::new();

    fn make_temp_u64(
        temp_counter: &mut u32,
        temps: &mut Vec<(String, String)>,
        init: &str,
    ) -> String {
        let name = format!("__tmp{}", *temp_counter);
        *temp_counter += 1;
        temps.push((name.clone(), init.to_string()));
        name
    }
    
    let mut imm_addr_cache: HashMap<(u32, String), String> = HashMap::new();

    let mut inst_index: u32 = 0;
    for block in &unit.blocks {
        for inst in &block.instructions {
            match &inst.opcode {
                Opcode::CallMap { .. } => {
                    if let Some(key_op) = inst.operands.get(0) {
                        if matches!(key_op, Operand::Immediate(_)) {
                            let key_expr = format_operand(key_op);
                            let tmp = make_temp_u64(&mut temp_counter, &mut temps, &key_expr);
                            imm_addr_cache.insert((inst_index, format!("k:{}", key_expr)), tmp);
                        }
                    }
                }
                Opcode::UpdateMap { .. } => {
                    if inst.operands.len() >= 2 {
                        let key_op = &inst.operands[0];
                        let val_op = &inst.operands[1];

                        if matches!(key_op, Operand::Immediate(_)) {
                            let key_expr = format_operand(key_op);
                            let tmp = make_temp_u64(&mut temp_counter, &mut temps, &key_expr);
                            imm_addr_cache.insert((inst_index, format!("k:{}", key_expr)), tmp);
                        }
                        if matches!(val_op, Operand::Immediate(_)) {
                            let val_expr = format_operand(val_op);
                            let tmp = make_temp_u64(&mut temp_counter, &mut temps, &val_expr);
                            imm_addr_cache.insert((inst_index, format!("v:{}", val_expr)), tmp);
                        }
                    }
                }
                _ => {}
            }

            inst_index += 1;
        }
    }
    
    for &id in vars_sorted.iter() {
        let ty = var_types.get(&id).copied().unwrap_or(crate::ast::Type::U64);
        let c_type = match ty {
            crate::ast::Type::U64 => "__u64",
            crate::ast::Type::U32 => "__u32",
            crate::ast::Type::I64 => "__s64",
            crate::ast::Type::I32 => "__s32",
        };

        if pointer_vars.contains(&id) {
            writeln!(out, "    {} *v{} = 0;", c_type, id).map_err(err)?;
        } else {
            writeln!(out, "    {} v{} = 0;", c_type, id).map_err(err)?;
        }
    }

    for (name, init) in &temps {
        writeln!(out, "    __u64 {} = {};", name, init).map_err(err)?;
    }

    if !vars_sorted.is_empty() || !temps.is_empty() {
        writeln!(out).map_err(err)?;
    }
    
    writeln!(out, "    goto __block_{};", unit.blocks[0].id.0).map_err(err)?;
    writeln!(out).map_err(err)?;

    let mut block_ids = HashSet::<u32>::new();
    for b in &unit.blocks {
        block_ids.insert(b.id.0);
    }
    
    inst_index = 0;
    for block in &unit.blocks {
        writeln!(out, "__block_{}:", block.id.0).map_err(err)?;

        for inst in &block.instructions {
            let res_used = used_vars.contains(&inst.result.0);
            let res_name = if res_used {
                Some(format!("v{}", inst.result.0))
            } else {
                None
            };

            let is_side_effect = matches!(
                inst.opcode,
                Opcode::Store { .. } | Opcode::UpdateMap { .. } | Opcode::NullCheck
            );
            
            if !is_side_effect && !res_used {
                inst_index += 1;
                continue;
            }

            match &inst.opcode {
                Opcode::Binary { op } => {
                    if inst.operands.len() >= 2 {
                        let left = format_operand(&inst.operands[0]);
                        let right = format_operand(&inst.operands[1]);
                        let op_str = match op {
                            BinaryOp::Add => "+",
                            BinaryOp::Sub => "-",
                            BinaryOp::Mul => "*",
                            BinaryOp::Div => "/",
                            BinaryOp::Mod => "%",
                            BinaryOp::Shl => "<<",
                            BinaryOp::Shr => ">>",
                        };
                        let res = res_name.unwrap();
                        writeln!(out, "    {} = {} {} {};", res, left, op_str, right)
                            .map_err(err)?;
                    }
                }

                Opcode::LoadKey => {
                    if let Some(operand) = inst.operands.get(0) {
                        let ptr = format_operand(operand);
                        let res = res_name.unwrap();
                        writeln!(out, "    {} = *{};", res, ptr).map_err(err)?;
                    }
                }

                Opcode::Store { .. } => {
                    if inst.operands.len() >= 2 {
                        let ptr = format_operand(&inst.operands[0]);
                        let val = format_operand(&inst.operands[1]);
                        writeln!(out, "    *{} = {};", ptr, val).map_err(err)?;
                    }
                }

                Opcode::LoadCtx { offset, size } => {
                    let res = res_name.unwrap();
                    writeln!(
                        out,
                        "    {} = *(__u{} *)((__u8 *)ctx + ({}));",
                        res,
                        size * 8,
                        offset
                    )
                    .map_err(err)?;
                }

                Opcode::LoadPacket { offset, size } => {
                    let res = res_name.unwrap();
                    writeln!(
                        out,
                        "    {} = *(__u{} *)((__u8 *)ctx + ({}));",
                        res,
                        size * 8,
                        offset
                    )
                    .map_err(err)?;
                }

                Opcode::HelperCall { id } => {
                    let call_expr = match *id {
                        5 => "bpf_ktime_get_ns()",
                        14 => "bpf_get_current_pid_tgid()",
                        15 => "bpf_get_current_uid_gid()",
                        35 => "bpf_get_current_task()",
                        _ => "0",
                    };
                    let res = res_name.unwrap();
                    writeln!(out, "    {} = {};", res, call_expr).map_err(err)?;
                }

                Opcode::CallMap { map_name } => {
                    if let Some(key_op) = inst.operands.get(0) {
                        let key_expr = format_operand(key_op);
                        let key_addr = match key_op {
                            Operand::Var(_) => format!("&{}", key_expr),
                            Operand::Immediate(_) => {
                                let tmp = imm_addr_cache
                                    .get(&(inst_index, format!("k:{}", key_expr)))
                                    .cloned()
                                    .ok_or_else(|| {
                                        "internal error: missing temp for immediate key".to_string()
                                    })?;
                                format!("&{}", tmp)
                            }
                        };

                        let res = res_name.unwrap();
                        writeln!(
                            out,
                            "    {} = bpf_map_lookup_elem(&{}, {});",
                            res, map_name, key_addr
                        )
                        .map_err(err)?;
                    }
                }

                Opcode::UpdateMap { map_name } => {
                    if inst.operands.len() >= 2 {
                        let key_op = &inst.operands[0];
                        let val_op = &inst.operands[1];

                        let key_expr = format_operand(key_op);
                        let val_expr = format_operand(val_op);

                        let key_addr = match key_op {
                            Operand::Var(_) => format!("&{}", key_expr),
                            Operand::Immediate(_) => {
                                let tmp = imm_addr_cache
                                    .get(&(inst_index, format!("k:{}", key_expr)))
                                    .cloned()
                                    .ok_or_else(|| {
                                        "internal error: missing temp for immediate key".to_string()
                                    })?;
                                format!("&{}", tmp)
                            }
                        };

                        let val_addr = match val_op {
                            Operand::Var(_) => format!("&{}", val_expr),
                            Operand::Immediate(_) => {
                                let tmp = imm_addr_cache
                                    .get(&(inst_index, format!("v:{}", val_expr)))
                                    .cloned()
                                    .ok_or_else(|| {
                                        "internal error: missing temp for immediate value"
                                            .to_string()
                                    })?;
                                format!("&{}", tmp)
                            }
                        };

                        if let Some(res) = res_name {
                            writeln!(
                                out,
                                "    {} = bpf_map_update_elem(&{}, {}, {}, 0);",
                                res, map_name, key_addr, val_addr
                            )
                            .map_err(err)?;
                        } else {
                            writeln!(
                                out,
                                "    (void)bpf_map_update_elem(&{}, {}, {}, 0);",
                                map_name, key_addr, val_addr
                            )
                            .map_err(err)?;
                        }
                    }
                }

                Opcode::NullCheck => {
                    if let Some(ptr_op) = inst.operands.get(0) {
                        let ptr_expr = format_operand(ptr_op);
                        
                        if branch_cond_vars.contains(&inst.result.0) {
                            let res = res_name.unwrap_or_else(|| format!("v{}", inst.result.0));
                            writeln!(out, "    {} = ({} != 0);", res, ptr_expr).map_err(err)?;
                        } else {
                            writeln!(out, "    if (!({})) goto __solnix_null_fail;", ptr_expr)
                                .map_err(err)?;

                            if let Some(res) = res_name {
                                writeln!(out, "    {} = 1;", res).map_err(err)?;
                            }
                        }
                    }
                }
            }

            inst_index += 1;
        }

        match &block.terminator {
            crate::ir::unit::Terminator::Return(op) => {
                writeln!(out, "    return {};", format_operand(op)).map_err(err)?;
            }
            crate::ir::unit::Terminator::Jump(target) => {
                if !block_ids.contains(&target.0) {
                    return Err(format!("Invalid jump target: {:?}", target));
                }
                writeln!(out, "    goto __block_{};", target.0).map_err(err)?;
            }
            crate::ir::unit::Terminator::Branch {
                condition,
                true_block,
                false_block,
            } => {
                if !block_ids.contains(&true_block.0) || !block_ids.contains(&false_block.0) {
                    return Err("Invalid branch targets".to_string());
                }
                writeln!(
                    out,
                    "    if ({}) goto __block_{}; else goto __block_{};",
                    format_operand(condition),
                    true_block.0,
                    false_block.0
                )
                .map_err(err)?;
            }
        }

        writeln!(out).map_err(err)?;
    }

    writeln!(out, "__solnix_null_fail:").map_err(err)?;
    writeln!(out, "    return 0;").map_err(err)?;
    writeln!(out, "}}").map_err(err)?;
    Ok(())
}

fn format_operand(op: &Operand) -> String {
    match op {
        Operand::Var(VarId(id)) => format!("v{}", id),
        Operand::Immediate(val) => val.to_string(),
    }
}

fn emit_license(out: &mut String, lic: &str) -> Result<(), String> {
    writeln!(out, "char LICENSE[] SEC(\"license\") = \"{}\";", lic).map_err(err)?;
    writeln!(out).map_err(err)?;
    Ok(())
}

fn err(e: std::fmt::Error) -> String {
    e.to_string()
}
