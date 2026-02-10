use std::fmt::Write;
use crate::ir::{UnitIr, Opcode, Operand, BinaryOp, VarId};
use std::collections::HashSet;

pub fn emit_tracepoint(out: &mut String, unit: &UnitIr, sec: &str) -> Result<(), String> {
    emit_license(out, &unit.license)?;
    
    writeln!(out, "SEC(\"{}\")", sec).map_err(err)?;
    writeln!(out, "int {}(void *ctx) {{", unit.name).map_err(err)?;
    writeln!(out, "    (void)ctx;").map_err(err)?;
    
    let mut pointer_vars = HashSet::new();
    for block in &unit.blocks {
        for inst in &block.instructions {
            if let Opcode::CallMap { .. } = inst.opcode {
                pointer_vars.insert(inst.result.0);
            }
        }
    }
    
    let mut declared = HashSet::new();
    for block in &unit.blocks {
        for inst in &block.instructions {
            if declared.insert(inst.result.0) {
                let c_type = match inst.result_type {
                    crate::ast::Type::U64 => "__u64",
                    crate::ast::Type::U32 => "__u32",
                    crate::ast::Type::I64 => "__s64",
                    crate::ast::Type::I32 => "__s32",
                };

                if pointer_vars.contains(&inst.result.0) {
                    writeln!(out, "    {} *v{} = 0;", c_type, inst.result.0).map_err(err)?;
                } else {
                    writeln!(out, "    {} v{} = 0;", c_type, inst.result.0).map_err(err)?;
                }
            }
        }
    }
    
    let mut in_if_block = false;
    for block in &unit.blocks {
        for inst in &block.instructions {
            let res = format!("v{}", inst.result.0);
            
            match &inst.opcode {
                Opcode::Binary { op } => {
                    if inst.operands.len() >= 2 {
                        let left = format_operand(&inst.operands[0]);
                        let right = format_operand(&inst.operands[1]);
                        let op_str = match op {
                            BinaryOp::Add => "+", BinaryOp::Sub => "-",
                            BinaryOp::Mul => "*", BinaryOp::Div => "/", BinaryOp::Mod => "%",
                            BinaryOp::Shl => "<<", BinaryOp::Shr => ">>",
                        };
                        writeln!(out, "    {} = {} {} {};", res, left, op_str, right).map_err(err)?;
                    }
                }

                Opcode::LoadKey => {
                    if let Some(operand) = inst.operands.get(0) {
                        let val = format_operand(operand);
                        writeln!(out, "    {} = {};", res, val).map_err(err)?;
                    }
                }

                Opcode::CallMap { map_name } => {
                    if let Some(key_op) = inst.operands.get(0) {
                        let key_val = format_operand(key_op);
                        writeln!(out, "    {} = bpf_map_lookup_elem(&{}, &{});", res, map_name, key_val).map_err(err)?;
                    }
                }

                Opcode::NullCheck => {
                    if let Some(ptr_op) = inst.operands.get(0) {
                        let ptr_val = format_operand(ptr_op);
                        writeln!(out, "    if ({}) {{", ptr_val).map_err(err)?;
                        in_if_block = true;
                    }
                }

                Opcode::Store { .. } => {
                    if inst.operands.len() >= 2 {
                        let ptr = format_operand(&inst.operands[0]);
                        let val = format_operand(&inst.operands[1]);
                        writeln!(out, "        *{} = {};", ptr, val).map_err(err)?;
                    }
                    if in_if_block {
                        writeln!(out, "    }}").map_err(err)?;
                        in_if_block = false;
                    }
                }

                _ => {}
            }
        }
    }

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