// src/emit/ebpf_obj/insn.rs
#[derive(Clone, Copy, Debug)]
pub struct BpfInsn {
    pub code: u8,
    pub regs: u8,
    pub off: i16,
    pub imm: i32,
}

impl BpfInsn {
    pub fn new(code: u8, dst: u8, src: u8, off: i16, imm: i32) -> Self {
        Self {
            code,
            regs: dst | (src << 4),
            off,
            imm,
        }
    }

    pub fn to_bytes(self) -> [u8; 8] {
        let mut out = [0u8; 8];
        out[0] = self.code;
        out[1] = self.regs;
        out[2..4].copy_from_slice(&self.off.to_le_bytes());
        out[4..8].copy_from_slice(&self.imm.to_le_bytes());
        out
    }
}

pub fn serialize_insns(insns: &[BpfInsn]) -> Vec<u8> {
    let mut out = Vec::with_capacity(insns.len() * 8);
    for insn in insns {
        out.extend_from_slice(&insn.to_bytes());
    }
    out
}

pub const R0: u8 = 0;
pub const R1: u8 = 1;
pub const R2: u8 = 2;
pub const R3: u8 = 3;
pub const R4: u8 = 4;
pub const R5: u8 = 5;
pub const R6: u8 = 6;
pub const R7: u8 = 7;
pub const R8: u8 = 8;
pub const R9: u8 = 9;
pub const R10: u8 = 10;

// classes
pub const BPF_LD: u8 = 0x00;
pub const BPF_LDX: u8 = 0x01;
pub const BPF_ST: u8 = 0x02;
pub const BPF_STX: u8 = 0x03;
pub const BPF_ALU: u8 = 0x04;
pub const BPF_JMP: u8 = 0x05;
pub const BPF_ALU64: u8 = 0x07;

// sizes / mode
pub const BPF_W: u8 = 0x00;
pub const BPF_H: u8 = 0x08;
pub const BPF_B: u8 = 0x10;
pub const BPF_DW: u8 = 0x18;
pub const BPF_IMM: u8 = 0x00;
pub const BPF_MEM: u8 = 0x60;

// src
pub const BPF_K: u8 = 0x00;
pub const BPF_X: u8 = 0x08;

// alu / jmp ops
pub const BPF_ADD: u8 = 0x00;
pub const BPF_SUB: u8 = 0x10;
pub const BPF_MUL: u8 = 0x20;
pub const BPF_DIV: u8 = 0x30;
pub const BPF_MOD: u8 = 0x90;
pub const BPF_LSH: u8 = 0x60;
pub const BPF_RSH: u8 = 0x70;
pub const BPF_MOV: u8 = 0xb0;
pub const BPF_ARSH: u8 = 0xc0;

pub const BPF_JA: u8 = 0x00;
pub const BPF_JEQ: u8 = 0x10;
pub const BPF_JNE: u8 = 0x50;
pub const BPF_CALL: u8 = 0x80;
pub const BPF_EXIT: u8 = 0x90;

#[derive(Clone, Copy)]
pub enum Size {
    B,
    H,
    W,
    Dw,
}

impl Size {
    pub fn code(self) -> u8 {
        match self {
            Size::B => BPF_B,
            Size::H => BPF_H,
            Size::W => BPF_W,
            Size::Dw => BPF_DW,
        }
    }
}

pub fn mov64_reg(dst: u8, src: u8) -> BpfInsn {
    BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_X, dst, src, 0, 0)
}

pub fn mov64_imm(dst: u8, imm: i32) -> BpfInsn {
    BpfInsn::new(BPF_ALU64 | BPF_MOV | BPF_K, dst, 0, 0, imm)
}

pub fn alu64_reg(op: u8, dst: u8, src: u8) -> BpfInsn {
    BpfInsn::new(BPF_ALU64 | op | BPF_X, dst, src, 0, 0)
}

pub fn alu64_imm(op: u8, dst: u8, imm: i32) -> BpfInsn {
    BpfInsn::new(BPF_ALU64 | op | BPF_K, dst, 0, 0, imm)
}

pub fn ldx_mem(size: Size, dst: u8, src: u8, off: i16) -> BpfInsn {
    BpfInsn::new(BPF_LDX | size.code() | BPF_MEM, dst, src, off, 0)
}

pub fn stx_mem(size: Size, dst: u8, off: i16, src: u8) -> BpfInsn {
    BpfInsn::new(BPF_STX | size.code() | BPF_MEM, dst, src, off, 0)
}

pub fn st_mem(size: Size, dst: u8, off: i16, imm: i32) -> BpfInsn {
    BpfInsn::new(BPF_ST | size.code() | BPF_MEM, dst, 0, off, imm)
}

pub fn jmp_imm(op: u8, dst: u8, imm: i32, off: i16) -> BpfInsn {
    BpfInsn::new(BPF_JMP | op | BPF_K, dst, 0, off, imm)
}

pub fn ja(off: i16) -> BpfInsn {
    BpfInsn::new(BPF_JMP | BPF_JA, 0, 0, off, 0)
}

pub fn call(helper_id: i32) -> BpfInsn {
    BpfInsn::new(BPF_JMP | BPF_CALL, 0, 0, 0, helper_id)
}

pub fn exit() -> BpfInsn {
    BpfInsn::new(BPF_JMP | BPF_EXIT, 0, 0, 0, 0)
}

pub fn ld_imm64(dst: u8, imm: i64) -> [BpfInsn; 2] {
    let lo = imm as i32;
    let hi = (imm >> 32) as i32;
    [
        BpfInsn::new(BPF_LD | BPF_DW | BPF_IMM, dst, 0, 0, lo),
        BpfInsn::new(0, 0, 0, 0, hi),
    ]
}

pub fn fits_i32(v: i64) -> bool {
    v >= i32::MIN as i64 && v <= i32::MAX as i64
}