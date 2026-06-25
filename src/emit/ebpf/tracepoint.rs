use std::collections::{HashMap, HashSet};

use crate::ir::unit::Terminator;
use crate::ir::{BinaryOp, Opcode, Operand, UnitIr, VarId};

use super::elf::ProgramReloc;
use super::insn::*;

#[derive(Debug)]
pub struct CompiledProgram {
    pub code: Vec<u8>,
    pub relocs: Vec<ProgramReloc>,
}

struct Builder {
    insns: Vec<Insn>,
    relocs: Vec<ProgramReloc>,
    stack_slots: HashMap<u32, i16>,
    scratch_slots: Vec<i16>,
    fixups: Vec<(usize, u32)>,
    block_offsets: HashMap<u32, usize>,
    next_scratch: usize,
}

impl Builder {
    fn new(unit: &UnitIr) -> Result<Self, String> {
        let mut ids = HashSet::new();
        for block in &unit.blocks {
            for inst in &block.instructions {
                ids.insert(inst.result.0);
                for op in &inst.operands {
                    if let Operand::Var(VarId(id)) = op {
                        ids.insert(*id);
                    }
                }
            }
            match &block.terminator {
                Terminator::Return(Operand::Var(VarId(id)))
                | Terminator::Branch {
                    condition: Operand::Var(VarId(id)),
                    ..
                } => {
                    ids.insert(*id);
                }
                _ => {}
            }
        }

        let mut ids: Vec<u32> = ids.into_iter().collect();
        ids.sort_unstable();

        let scratch_count = 8_usize;
        let total_slots = ids.len() + scratch_count;
        let frame_bytes = total_slots * 8;
        if frame_bytes > 512 {
            return Err(format!(
                "tracepoint '{}' needs {} bytes of eBPF stack, maximum is 512",
                unit.name, frame_bytes
            ));
        }

        let mut stack_slots = HashMap::new();
        for (idx, id) in ids.iter().enumerate() {
            stack_slots.insert(*id, -8 * (idx as i16 + 1));
        }

        let scratch_base = ids.len() as i16;
        let scratch_slots = (0..scratch_count)
            .map(|i| -8 * (scratch_base + i as i16 + 1))
            .collect();

        Ok(Self {
            insns: Vec::new(),
            relocs: Vec::new(),
            stack_slots,
            scratch_slots,
            fixups: Vec::new(),
            block_offsets: HashMap::new(),
            next_scratch: 0,
        })
    }

    fn push(&mut self, insn: Insn) {
        self.insns.push(insn);
    }

    fn bytes_len(&self) -> u64 {
        (self.insns.len() * 8) as u64
    }

    fn emit_load_operand(&mut self, dst: u8, op: &Operand) -> Result<(), String> {
        match op {
            Operand::Immediate(value) => {
                let imm = imm32(*value)?;
                self.push(Insn::new(BPF_MOV64_IMM, dst, 0, 0, imm));
            }
            Operand::Var(VarId(id)) => {
                let off = *self
                    .stack_slots
                    .get(id)
                    .ok_or_else(|| format!("missing stack slot for v{}", id))?;
                self.push(Insn::new(ldx_mem(8)?, dst, R10, off, 0));
            }
        }
        Ok(())
    }

    fn emit_store_var(&mut self, id: VarId, src: u8) -> Result<(), String> {
        let off = *self
            .stack_slots
            .get(&id.0)
            .ok_or_else(|| format!("missing stack slot for v{}", id.0))?;
        self.push(Insn::new(stx_mem(8)?, R10, src, off, 0));
        Ok(())
    }

    fn emit_operand_addr(&mut self, dst: u8, op: &Operand) -> Result<(), String> {
        match op {
            Operand::Var(VarId(id)) => {
                let off = *self
                    .stack_slots
                    .get(id)
                    .ok_or_else(|| format!("missing stack slot for v{}", id))?;
                self.push(Insn::new(BPF_MOV64_REG, dst, R10, 0, 0));
                self.push(Insn::new(0x07, dst, 0, 0, off as i32));
            }
            Operand::Immediate(value) => {
                let slot = self.take_scratch()?;
                self.push(Insn::new(BPF_MOV64_IMM, dst, 0, 0, imm32(*value)?));
                self.push(Insn::new(stx_mem(8)?, R10, dst, slot, 0));
                self.push(Insn::new(BPF_MOV64_REG, dst, R10, 0, 0));
                self.push(Insn::new(0x07, dst, 0, 0, slot as i32));
            }
        }
        Ok(())
    }

    fn take_scratch(&mut self) -> Result<i16, String> {
        let slot = self
            .scratch_slots
            .get(self.next_scratch % self.scratch_slots.len())
            .copied()
            .ok_or_else(|| "no scratch stack slots available".to_string())?;
        self.next_scratch += 1;
        Ok(slot)
    }

    fn emit_map_ldimm(&mut self, dst: u8, map_name: &str) {
        let offset = self.bytes_len();
        self.push(Insn::new(BPF_LD_DW_IMM, dst, 1, 0, 0));
        self.push(Insn::new(0, 0, 0, 0, 0));
        self.relocs.push(ProgramReloc {
            offset,
            symbol: map_name.to_string(),
        });
    }

    fn emit_jump_to_block(&mut self, target: u32) {
        let idx = self.insns.len();
        self.push(Insn::new(BPF_JA, 0, 0, 0, 0));
        self.fixups.push((idx, target));
    }

    fn patch_jumps(&mut self) -> Result<(), String> {
        for (idx, block_id) in &self.fixups {
            let target = *self
                .block_offsets
                .get(block_id)
                .ok_or_else(|| format!("invalid jump target block {}", block_id))?;
            let rel = target as isize - *idx as isize - 1;
            if rel < i16::MIN as isize || rel > i16::MAX as isize {
                return Err(format!("jump to block {} exceeds eBPF range", block_id));
            }
            self.insns[*idx].off = rel as i16;
        }
        Ok(())
    }
}

pub fn compile_tracepoint(unit: &UnitIr) -> Result<CompiledProgram, String> {
    let mut b = Builder::new(unit)?;
    let branch_cond_vars = branch_condition_vars(unit);
    b.push(Insn::new(BPF_MOV64_REG, R6, R1, 0, 0));

    for block in &unit.blocks {
        b.block_offsets.insert(block.id.0, b.insns.len());

        for inst in &block.instructions {
            let _result_type = inst.result_type;
            match &inst.opcode {
                Opcode::Binary { op } => emit_binary(&mut b, inst.result, *op, &inst.operands)?,
                Opcode::LoadKey => {
                    let ptr = inst
                        .operands
                        .first()
                        .ok_or_else(|| "LoadKey expects one operand".to_string())?;
                    b.emit_load_operand(R1, ptr)?;
                    b.push(Insn::new(ldx_mem(8)?, R0, R1, 0, 0));
                    b.emit_store_var(inst.result, R0)?;
                }
                Opcode::Store { size } => {
                    if inst.operands.len() != 2 {
                        return Err("Store expects pointer and value operands".to_string());
                    }
                    b.emit_load_operand(R1, &inst.operands[0])?;
                    b.emit_load_operand(R2, &inst.operands[1])?;
                    b.push(Insn::new(stx_mem(*size)?, R1, R2, 0, 0));
                }
                Opcode::LoadCtx { offset, size } | Opcode::LoadPacket { offset, size } => {
                    b.push(Insn::new(ldx_mem(*size)?, R0, R6, checked_i16(*offset)?, 0));
                    b.emit_store_var(inst.result, R0)?;
                }
                Opcode::HelperCall { id } => {
                    emit_helper_call(&mut b, inst.result, *id, &inst.operands)?
                }
                Opcode::CallMap { map_name } => {
                    let key = inst
                        .operands
                        .first()
                        .ok_or_else(|| "map lookup expects a key operand".to_string())?;
                    b.emit_map_ldimm(R1, map_name);
                    b.emit_operand_addr(R2, key)?;
                    b.push(Insn::new(BPF_CALL, 0, 0, 0, 1));
                    b.emit_store_var(inst.result, R0)?;
                }
                Opcode::UpdateMap { map_name } => {
                    if inst.operands.len() != 2 {
                        return Err("map update expects key and value operands".to_string());
                    }
                    b.emit_map_ldimm(R1, map_name);
                    b.emit_operand_addr(R2, &inst.operands[0])?;
                    b.emit_operand_addr(R3, &inst.operands[1])?;
                    b.push(Insn::new(BPF_MOV64_IMM, R4, 0, 0, 0));
                    b.push(Insn::new(BPF_CALL, 0, 0, 0, 2));
                    b.emit_store_var(inst.result, R0)?;
                }
                Opcode::NullCheck => {
                    let ptr = inst
                        .operands
                        .first()
                        .ok_or_else(|| "NullCheck expects pointer operand".to_string())?;
                    b.emit_load_operand(R0, ptr)?;
                    if branch_cond_vars.contains(&inst.result.0) {
                        b.push(Insn::new(BPF_JEQ_IMM, R0, 0, 2, 0));
                        b.push(Insn::new(BPF_MOV64_IMM, R0, 0, 0, 1));
                        b.push(Insn::new(BPF_JA, 0, 0, 1, 0));
                        b.push(Insn::new(BPF_MOV64_IMM, R0, 0, 0, 0));
                        b.emit_store_var(inst.result, R0)?;
                    } else {
                        b.push(Insn::new(BPF_JEQ_IMM, R0, 0, 1, 0));
                        b.push(Insn::new(BPF_JA, 0, 0, 2, 0));
                        b.push(Insn::new(BPF_MOV64_IMM, R0, 0, 0, 0));
                        b.push(Insn::new(BPF_EXIT, 0, 0, 0, 0));
                        b.push(Insn::new(BPF_MOV64_IMM, R0, 0, 0, 1));
                        b.emit_store_var(inst.result, R0)?;
                    }
                }
                Opcode::RingBufReserve { map_name, size } => {
                    b.emit_map_ldimm(R1, map_name);
                    b.push(Insn::new(BPF_MOV64_IMM, R2, 0, 0, imm32(*size as i64)?));
                    b.push(Insn::new(BPF_MOV64_IMM, R3, 0, 0, 0));
                    b.push(Insn::new(BPF_CALL, 0, 0, 0, 131));
                    b.emit_store_var(inst.result, R0)?;
                }
                Opcode::RingBufSubmit { map_name } => {
                    let _map_name = map_name;
                    let ptr = inst
                        .operands
                        .first()
                        .ok_or_else(|| "ringbuf submit expects pointer operand".to_string())?;
                    b.emit_load_operand(R1, ptr)?;
                    b.push(Insn::new(BPF_MOV64_IMM, R2, 0, 0, 0));
                    b.push(Insn::new(BPF_CALL, 0, 0, 0, 132));
                    b.push(Insn::new(BPF_MOV64_IMM, R0, 0, 0, 0));
                    b.emit_store_var(inst.result, R0)?;
                }
                Opcode::CopyCtxToMem { offset, size } => {
                    let dest = inst
                        .operands
                        .first()
                        .ok_or_else(|| "CopyCtxToMem expects destination operand".to_string())?;
                    b.emit_load_operand(R1, dest)?;
                    b.push(Insn::new(BPF_MOV64_IMM, R2, 0, 0, imm32(*size as i64)?));
                    b.push(Insn::new(BPF_MOV64_REG, R3, R6, 0, 0));
                    b.push(Insn::new(0x07, R3, 0, 0, *offset));
                    b.push(Insn::new(BPF_CALL, 0, 0, 0, 113));
                    b.emit_store_var(inst.result, R0)?;
                }
            }
        }

        match &block.terminator {
            Terminator::Return(op) => {
                b.emit_load_operand(R0, op)?;
                b.push(Insn::new(BPF_EXIT, 0, 0, 0, 0));
            }
            Terminator::Jump(target) => b.emit_jump_to_block(target.0),
            Terminator::Branch {
                condition,
                true_block,
                false_block,
            } => {
                b.emit_load_operand(R0, condition)?;
                let jne_idx = b.insns.len();
                b.push(Insn::new(BPF_JNE_IMM, R0, 0, 0, 0));
                b.emit_jump_to_block(false_block.0);
                b.fixups.push((jne_idx, true_block.0));
            }
        }
    }

    b.patch_jumps()?;

    let mut code = Vec::with_capacity(b.insns.len() * 8);
    for insn in &b.insns {
        code.extend_from_slice(&insn.to_le_bytes());
    }

    Ok(CompiledProgram {
        code,
        relocs: b.relocs,
    })
}

fn branch_condition_vars(unit: &UnitIr) -> HashSet<u32> {
    let mut out = HashSet::new();
    for block in &unit.blocks {
        if let Terminator::Branch {
            condition: Operand::Var(VarId(id)),
            ..
        } = &block.terminator
        {
            out.insert(*id);
        }
    }
    out
}

fn emit_binary(
    b: &mut Builder,
    result: VarId,
    op: BinaryOp,
    operands: &[Operand],
) -> Result<(), String> {
    if operands.len() != 2 {
        return Err("binary instruction expects two operands".to_string());
    }
    b.emit_load_operand(R0, &operands[0])?;
    match &operands[1] {
        Operand::Immediate(value) => b.push(Insn::new(alu64_imm(op), R0, 0, 0, imm32(*value)?)),
        Operand::Var(_) => {
            b.emit_load_operand(R1, &operands[1])?;
            b.push(Insn::new(alu64_reg(op), R0, R1, 0, 0));
        }
    }
    b.emit_store_var(result, R0)
}

fn emit_helper_call(
    b: &mut Builder,
    result: VarId,
    id: u32,
    operands: &[Operand],
) -> Result<(), String> {
    match id {
        5 | 14 | 15 | 35 => {
            if !operands.is_empty() {
                return Err(format!("helper {} expects no operands", id));
            }
        }
        16 => {
            return Err(
                "ctx.get_current_comm requires a destination buffer; IR does not carry one yet"
                    .to_string(),
            );
        }
        202 | 204 => {
            if operands.len() != 3 {
                return Err(format!("helper {} expects dest, size, src operands", id));
            }
            b.emit_load_operand(R1, &operands[0])?;
            b.emit_load_operand(R2, &operands[1])?;
            b.emit_load_operand(R3, &operands[2])?;
        }
        other => return Err(format!("unsupported helper id {}", other)),
    }

    b.push(Insn::new(BPF_CALL, 0, 0, 0, id as i32));
    b.emit_store_var(result, R0)
}

fn imm32(value: i64) -> Result<i32, String> {
    i32::try_from(value).map_err(|_| format!("immediate {} does not fit in eBPF imm32", value))
}

fn checked_i16(value: i32) -> Result<i16, String> {
    i16::try_from(value).map_err(|_| format!("offset {} does not fit in eBPF off16", value))
}
