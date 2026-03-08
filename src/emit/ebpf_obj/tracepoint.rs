// src/emit/ebpf_obj/tracepoint.rs
use std::collections::{HashMap, HashSet};

use crate::{
    ast::Type,
    ir::{BinaryOp, Opcode, Operand, UnitIr, VarId},
};

use super::{insn::*, maps::MapsSection};

#[derive(Clone, Debug)]
pub struct RelocRequest {
    pub insn_byte_off: u64,
    pub map_name: String,
}

#[derive(Clone, Debug)]
pub struct ProgramSection {
    pub section_name: String,
    pub symbol_name: String,
    pub code: Vec<u8>,
    pub relocs: Vec<RelocRequest>,
}

#[derive(Clone, Copy, Debug)]
enum Label {
    Block(u32),
    NullFail,
}

#[derive(Clone, Debug)]
struct JumpFixup {
    at_insn: usize,
    target: Label,
}

#[derive(Clone, Debug)]
struct FrameLayout {
    slots: HashMap<u32, i16>,
    types: HashMap<u32, Type>,
    pointer_vars: HashSet<u32>,
    scratch_a: i16,
    scratch_b: i16,
    frame_size: usize,
}

struct Codegen<'a> {
    unit: &'a UnitIr,
    maps: &'a MapsSection,
    frame: FrameLayout,
    used_vars: HashSet<u32>,
    insns: Vec<BpfInsn>,
    relocs: Vec<RelocRequest>,
    block_pos: HashMap<u32, usize>,
    fixups: Vec<JumpFixup>,
    block_ids: HashSet<u32>,
}

pub fn emit_tracepoint(
    unit: &UnitIr,
    sec: &str,
    maps: &MapsSection,
) -> Result<ProgramSection, String> {
    let used_vars = collect_used_vars(unit);
    let (var_types, pointer_vars) = collect_var_info(unit, &used_vars);
    let block_ids = unit.blocks.iter().map(|b| b.id.0).collect::<HashSet<_>>();

    let frame = build_frame(&used_vars, var_types, pointer_vars)?;

    let mut cg = Codegen {
        unit,
        maps,
        frame,
        used_vars,
        insns: Vec::new(),
        relocs: Vec::new(),
        block_pos: HashMap::new(),
        fixups: Vec::new(),
        block_ids,
    };

    // prologue: keep ctx in callee-saved r6
    cg.emit(mov64_reg(R6, R1));

    for block in &unit.blocks {
        cg.block_pos.insert(block.id.0, cg.pos());

        for inst in &block.instructions {
            let res_used = cg.used_vars.contains(&inst.result.0);
            let is_side_effect = matches!(
                inst.opcode,
                Opcode::Store { .. }
                    | Opcode::UpdateMap { .. }
                    | Opcode::NullCheck
                    | Opcode::RingBufSubmit { .. }
            );

            if !res_used && !is_side_effect {
                continue;
            }

            match &inst.opcode {
                Opcode::Binary { op } => {
                    cg.emit_binary(inst.result.0, inst.result_type, op, &inst.operands)?;
                }

                Opcode::LoadCtx { offset, size } => {
                    cg.emit_load_ctx(
                        inst.result.0,
                        inst.result_type,
                        *offset as i16,
                        (*size).into(),
                    )?;
                }

                Opcode::LoadPacket { offset, size } => {
                    cg.emit_load_ctx(
                        inst.result.0,
                        inst.result_type,
                        *offset as i16,
                        (*size).into(),
                    )?;
                }

                Opcode::HelperCall { id } => {
                    cg.emit(call(*id as i32));
                    cg.store_r0(inst.result.0)?;
                }

                Opcode::CallMap { map_name } => {
                    cg.emit_map_lookup(inst.result.0, map_name, inst.operands.get(0))?;
                }

                Opcode::UpdateMap { map_name } => {
                    if inst.operands.len() < 2 {
                        return Err("UpdateMap requires key and value operands".to_string());
                    }
                    cg.emit_map_update(
                        res_used.then_some(inst.result.0),
                        map_name,
                        &inst.operands[0],
                        &inst.operands[1],
                    )?;
                }

                Opcode::NullCheck => {
                    let ptr = inst
                        .operands
                        .get(0)
                        .ok_or_else(|| "NullCheck missing operand".to_string())?;
                    cg.emit_nullcheck_to_bool(inst.result.0, ptr)?;
                }

                Opcode::LoadKey => {
                    let ptr = inst
                        .operands
                        .get(0)
                        .ok_or_else(|| "LoadKey missing operand".to_string())?;
                    cg.emit_load_from_ptr(inst.result.0, inst.result_type, ptr)?;
                }

                Opcode::Store { size } => {
                    if inst.operands.len() < 2 {
                        return Err("Store requires ptr and value".to_string());
                    }
                    cg.emit_store_to_ptr(*size as usize, &inst.operands[0], &inst.operands[1])?;
                }

                Opcode::RingBufReserve { map_name, size } => {
                    cg.emit_ringbuf_reserve(inst.result.0, map_name, *size as i64)?;
                }

                Opcode::RingBufSubmit { .. } => {
                    let ptr = inst
                        .operands
                        .get(0)
                        .ok_or_else(|| "RingBufSubmit missing pointer operand".to_string())?;
                    cg.emit_ringbuf_submit(ptr)?;
                }
            }
        }

        match &block.terminator {
            crate::ir::unit::Terminator::Return(op) => {
                cg.load_operand(R0, op)?;
                cg.emit(exit());
            }
            crate::ir::unit::Terminator::Jump(target) => {
                if !cg.block_ids.contains(&target.0) {
                    return Err(format!("invalid jump target {}", target.0));
                }
                let at = cg.pos();
                cg.emit(ja(0));
                cg.fixups.push(JumpFixup {
                    at_insn: at,
                    target: Label::Block(target.0),
                });
            }
            crate::ir::unit::Terminator::Branch {
                condition,
                true_block,
                false_block,
            } => {
                if !cg.block_ids.contains(&true_block.0) || !cg.block_ids.contains(&false_block.0) {
                    return Err("invalid branch targets".to_string());
                }

                cg.load_operand(R1, condition)?;

                let at_false = cg.pos();
                cg.emit(jmp_imm(BPF_JEQ, R1, 0, 0));
                cg.fixups.push(JumpFixup {
                    at_insn: at_false,
                    target: Label::Block(false_block.0),
                });

                let at_true = cg.pos();
                cg.emit(ja(0));
                cg.fixups.push(JumpFixup {
                    at_insn: at_true,
                    target: Label::Block(true_block.0),
                });
            }
        }
    }

    let null_fail_pos = cg.pos();
    cg.emit(mov64_imm(R0, 0));
    cg.emit(exit());

    cg.patch_fixups(null_fail_pos)?;

    Ok(ProgramSection {
        section_name: sec.to_string(),
        symbol_name: unit.name.clone(),
        code: serialize_insns(&cg.insns),
        relocs: cg.relocs,
    })
}

impl<'a> Codegen<'a> {
    fn pos(&self) -> usize {
        self.insns.len()
    }

    fn emit(&mut self, insn: BpfInsn) {
        self.insns.push(insn);
    }

    fn emit2(&mut self, pair: [BpfInsn; 2]) {
        self.insns.push(pair[0]);
        self.insns.push(pair[1]);
    }

    fn patch_fixups(&mut self, null_fail_pos: usize) -> Result<(), String> {
        for fix in &self.fixups {
            let target = match fix.target {
                Label::Block(id) => *self
                    .block_pos
                    .get(&id)
                    .ok_or_else(|| format!("missing block position for {}", id))?,
                Label::NullFail => null_fail_pos,
            };

            let off = target as isize - fix.at_insn as isize - 1;
            if off < i16::MIN as isize || off > i16::MAX as isize {
                return Err("jump offset out of range".to_string());
            }
            self.insns[fix.at_insn].off = off as i16;
        }
        Ok(())
    }

    fn slot_of(&self, var: u32) -> Result<i16, String> {
        self.frame
            .slots
            .get(&var)
            .copied()
            .ok_or_else(|| format!("missing stack slot for v{}", var))
    }

    fn store_reg_to_var(&mut self, reg: u8, var: u32) -> Result<(), String> {
        let off = self.slot_of(var)?;
        self.emit(stx_mem(Size::Dw, R10, off, reg));
        Ok(())
    }

    fn store_r0(&mut self, var: u32) -> Result<(), String> {
        self.store_reg_to_var(R0, var)
    }

    fn load_var(&mut self, dst: u8, var: u32) -> Result<(), String> {
        let off = self.slot_of(var)?;
        self.emit(ldx_mem(Size::Dw, dst, R10, off));
        Ok(())
    }

    fn lea_stack(&mut self, dst: u8, off: i16) {
        self.emit(mov64_reg(dst, R10));
        self.emit(alu64_imm(BPF_ADD, dst, off as i32));
    }

    fn load_imm64(&mut self, dst: u8, imm: i64) {
        if fits_i32(imm) {
            self.emit(mov64_imm(dst, imm as i32));
        } else {
            self.emit2(ld_imm64(dst, imm));
        }
    }

    fn load_operand(&mut self, dst: u8, op: &Operand) -> Result<(), String> {
        match op {
            Operand::Var(VarId(id)) => self.load_var(dst, *id),
            Operand::Immediate(v) => {
                self.load_imm64(dst, *v);
                Ok(())
            }
        }
    }

    fn operand_addr(&mut self, dst: u8, op: &Operand, scratch_slot: i16) -> Result<(), String> {
        match op {
            Operand::Var(VarId(id)) => {
                let off = self.slot_of(*id)?;
                self.lea_stack(dst, off);
                Ok(())
            }
            Operand::Immediate(v) => {
                self.load_imm64(R0, *v);
                self.emit(stx_mem(Size::Dw, R10, scratch_slot, R0));
                self.lea_stack(dst, scratch_slot);
                Ok(())
            }
        }
    }

    fn emit_load_ctx(
        &mut self,
        dst_var: u32,
        result_type: Type,
        offset: i16,
        size: usize,
    ) -> Result<(), String> {
        let sz = match size {
            1 => Size::B,
            2 => Size::H,
            4 => Size::W,
            8 => Size::Dw,
            _ => return Err(format!("unsupported LoadCtx size {}", size)),
        };

        self.emit(ldx_mem(sz, R0, R6, offset));

        if matches!(result_type, Type::I32) && size == 4 {
            self.emit(alu64_imm(BPF_LSH, R0, 32));
            self.emit(alu64_imm(BPF_ARSH, R0, 32));
        }

        self.store_reg_to_var(R0, dst_var)
    }

    fn emit_binary(
        &mut self,
        dst_var: u32,
        _result_type: Type,
        op: &BinaryOp,
        operands: &[Operand],
    ) -> Result<(), String> {
        if operands.len() < 2 {
            return Err("Binary opcode needs 2 operands".to_string());
        }

        self.load_operand(R0, &operands[0])?;

        let op_bits = match op {
            BinaryOp::Add => BPF_ADD,
            BinaryOp::Sub => BPF_SUB,
            BinaryOp::Mul => BPF_MUL,
            BinaryOp::Div => BPF_DIV,
            BinaryOp::Mod => BPF_MOD,
            BinaryOp::Shl => BPF_LSH,
            BinaryOp::Shr => BPF_RSH,
        };

        match &operands[1] {
            Operand::Immediate(v) if fits_i32(*v) => {
                if matches!(op, BinaryOp::Div | BinaryOp::Mod) && *v == 0 {
                    return Err("division/modulo by zero".to_string());
                }
                self.emit(alu64_imm(op_bits, R0, *v as i32));
            }
            rhs => {
                self.load_operand(R1, rhs)?;
                if matches!(op, BinaryOp::Div | BinaryOp::Mod) {
                    let at = self.pos();
                    self.emit(jmp_imm(BPF_JEQ, R1, 0, 0));
                    self.fixups.push(JumpFixup {
                        at_insn: at,
                        target: Label::NullFail,
                    });
                }
                self.emit(alu64_reg(op_bits, R0, R1));
            }
        }

        self.store_reg_to_var(R0, dst_var)
    }

    fn emit_map_lookup(
        &mut self,
        dst_var: u32,
        map_name: &str,
        key: Option<&Operand>,
    ) -> Result<(), String> {
        self.require_map(map_name)?;

        let insn_idx = self.pos();
        self.emit2(ld_imm64(R1, 0));
        self.relocs.push(RelocRequest {
            insn_byte_off: (insn_idx as u64) * 8,
            map_name: map_name.to_string(),
        });

        let key_op = key.ok_or_else(|| format!("CallMap '{}' missing key operand", map_name))?;
        self.operand_addr(R2, key_op, self.frame.scratch_a)?;

        self.emit(call(1)); // bpf_map_lookup_elem
        self.store_reg_to_var(R0, dst_var)
    }

    fn emit_map_update(
        &mut self,
        dst_var: Option<u32>,
        map_name: &str,
        key: &Operand,
        value: &Operand,
    ) -> Result<(), String> {
        self.require_map(map_name)?;

        let insn_idx = self.pos();
        self.emit2(ld_imm64(R1, 0));
        self.relocs.push(RelocRequest {
            insn_byte_off: (insn_idx as u64) * 8,
            map_name: map_name.to_string(),
        });

        self.operand_addr(R2, key, self.frame.scratch_a)?;
        self.operand_addr(R3, value, self.frame.scratch_b)?;
        self.emit(mov64_imm(R4, 0)); // BPF_ANY
        self.emit(call(2)); // bpf_map_update_elem

        if let Some(v) = dst_var {
            self.store_reg_to_var(R0, v)?;
        }

        Ok(())
    }

    fn emit_null_guard_reg(&mut self, reg: u8) {
        let at = self.pos();
        self.emit(jmp_imm(BPF_JEQ, reg, 0, 0));
        self.fixups.push(JumpFixup {
            at_insn: at,
            target: Label::NullFail,
        });
    }

    fn emit_nullcheck_to_bool(&mut self, dst_var: u32, ptr: &Operand) -> Result<(), String> {
        self.load_operand(R1, ptr)?;
        self.emit(mov64_imm(R0, 0));

        let jne_set1 = self.pos();
        self.emit(jmp_imm(BPF_JNE, R1, 0, 0));

        self.store_reg_to_var(R0, dst_var)?;

        let ja_done = self.pos();
        self.emit(ja(0));

        let set1_pos = self.pos();
        self.emit(mov64_imm(R0, 1));
        self.store_reg_to_var(R0, dst_var)?;

        let done_pos = self.pos();

        self.insns[jne_set1].off = (set1_pos as isize - jne_set1 as isize - 1) as i16;
        self.insns[ja_done].off = (done_pos as isize - ja_done as isize - 1) as i16;

        Ok(())
    }

    fn emit_load_from_ptr(
        &mut self,
        dst_var: u32,
        result_type: Type,
        ptr: &Operand,
    ) -> Result<(), String> {
        self.load_operand(R1, ptr)?;
        self.emit_null_guard_reg(R1);

        let size = match result_type {
            Type::U32 | Type::I32 => Size::W,
            Type::U64 | Type::I64 => Size::Dw,
        };

        self.emit(ldx_mem(size, R0, R1, 0));

        if matches!(result_type, Type::I32) {
            self.emit(alu64_imm(BPF_LSH, R0, 32));
            self.emit(alu64_imm(BPF_ARSH, R0, 32));
        }

        self.store_reg_to_var(R0, dst_var)
    }

    fn emit_store_to_ptr(
        &mut self,
        size: usize,
        ptr: &Operand,
        value: &Operand,
    ) -> Result<(), String> {
        self.load_operand(R1, ptr)?;
        self.emit_null_guard_reg(R1);
        self.load_operand(R2, value)?;

        let sz = match size {
            1 => Size::B,
            2 => Size::H,
            4 => Size::W,
            8 => Size::Dw,
            _ => return Err(format!("unsupported Store size {}", size)),
        };

        self.emit(stx_mem(sz, R1, 0, R2));
        Ok(())
    }

    fn emit_ringbuf_reserve(
        &mut self,
        dst_var: u32,
        map_name: &str,
        size: i64,
    ) -> Result<(), String> {
        self.require_map(map_name)?;

        let insn_idx = self.pos();
        self.emit2(ld_imm64(R1, 0));
        self.relocs.push(RelocRequest {
            insn_byte_off: (insn_idx as u64) * 8,
            map_name: map_name.to_string(),
        });

        self.load_imm64(R2, size);
        self.emit(mov64_imm(R3, 0));
        self.emit(call(131)); // bpf_ringbuf_reserve
        self.store_reg_to_var(R0, dst_var)
    }

    fn emit_ringbuf_submit(&mut self, ptr: &Operand) -> Result<(), String> {
        self.load_operand(R1, ptr)?;
        self.emit_null_guard_reg(R1);
        self.emit(mov64_imm(R2, 0));
        self.emit(call(132)); // bpf_ringbuf_submit
        Ok(())
    }

    fn require_map(&self, name: &str) -> Result<(), String> {
        if self.maps.by_name.contains_key(name) {
            Ok(())
        } else {
            Err(format!("unknown map '{}'", name))
        }
    }
}

fn collect_used_vars(unit: &UnitIr) -> HashSet<u32> {
    let mut used = HashSet::new();

    for block in &unit.blocks {
        for inst in &block.instructions {

            // include result variables
            used.insert(inst.result.0);

            for op in &inst.operands {
                if let Operand::Var(VarId(id)) = op {
                    used.insert(*id);
                }
            }
        }

        match &block.terminator {
            crate::ir::unit::Terminator::Return(op) => {
                if let Operand::Var(VarId(id)) = op {
                    used.insert(*id);
                }
            }
            crate::ir::unit::Terminator::Jump(_) => {}
            crate::ir::unit::Terminator::Branch { condition, .. } => {
                if let Operand::Var(VarId(id)) = condition {
                    used.insert(*id);
                }
            }
        }
    }

    used
}

fn collect_var_info(unit: &UnitIr, used_vars: &HashSet<u32>) -> (HashMap<u32, Type>, HashSet<u32>) {
    let mut var_types = HashMap::new();
    let mut ptrs = HashSet::new();

    for block in &unit.blocks {
        for inst in &block.instructions {
            var_types.entry(inst.result.0).or_insert(inst.result_type);

            if used_vars.contains(&inst.result.0)
                && matches!(
                    inst.opcode,
                    Opcode::CallMap { .. } | Opcode::RingBufReserve { .. }
                )
            {
                ptrs.insert(inst.result.0);
            }
        }
    }

    (var_types, ptrs)
}

fn build_frame(
    used_vars: &HashSet<u32>,
    types: HashMap<u32, Type>,
    pointer_vars: HashSet<u32>,
) -> Result<FrameLayout, String> {
    let mut vars = used_vars.iter().copied().collect::<Vec<_>>();
    vars.sort_unstable();

    let mut slots = HashMap::new();
    let mut next = 8usize;

    for id in vars {
        slots.insert(id, -(next as i16));
        next += 8;
    }

    let scratch_a = -(next as i16);
    next += 8;

    let scratch_b = -(next as i16);
    next += 8;

    let frame_size = align_up(next, 16);
    if frame_size > 512 {
        return Err(format!(
            "stack frame too large: {} bytes (max 512)",
            frame_size
        ));
    }

    Ok(FrameLayout {
        slots,
        types,
        pointer_vars,
        scratch_a,
        scratch_b,
        frame_size,
    })
}

fn align_up(v: usize, a: usize) -> usize {
    (v + (a - 1)) & !(a - 1)
}
