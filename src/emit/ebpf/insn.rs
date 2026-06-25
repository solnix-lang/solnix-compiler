#[derive(Debug, Clone)]
pub struct Insn {
    pub code: u8,
    pub dst: u8,
    pub src: u8,
    pub off: i16,
    pub imm: i32,
}

impl Insn {
    pub fn new(code: u8, dst: u8, src: u8, off: i16, imm: i32) -> Self {
        Self {
            code,
            dst,
            src,
            off,
            imm,
        }
    }

    pub fn to_le_bytes(&self) -> [u8; 8] {
        let mut out = [0_u8; 8];
        out[0] = self.code;
        out[1] = (self.dst & 0x0f) | ((self.src & 0x0f) << 4);
        out[2..4].copy_from_slice(&self.off.to_le_bytes());
        out[4..8].copy_from_slice(&self.imm.to_le_bytes());
        out
    }
}

pub const R0: u8 = 0;
pub const R1: u8 = 1;
pub const R2: u8 = 2;
pub const R3: u8 = 3;
pub const R4: u8 = 4;
pub const R6: u8 = 6;
pub const R10: u8 = 10;

pub const BPF_LD_DW_IMM: u8 = 0x18;
pub const BPF_CALL: u8 = 0x85;
pub const BPF_EXIT: u8 = 0x95;
pub const BPF_JA: u8 = 0x05;
pub const BPF_JEQ_IMM: u8 = 0x15;
pub const BPF_JNE_IMM: u8 = 0x55;
pub const BPF_MOV64_IMM: u8 = 0xb7;
pub const BPF_MOV64_REG: u8 = 0xbf;

pub fn alu64_imm(op: crate::ir::BinaryOp) -> u8 {
    match op {
        crate::ir::BinaryOp::Add => 0x07,
        crate::ir::BinaryOp::Sub => 0x17,
        crate::ir::BinaryOp::Mul => 0x27,
        crate::ir::BinaryOp::Div => 0x37,
        crate::ir::BinaryOp::Shl => 0x67,
        crate::ir::BinaryOp::Shr => 0x77,
        crate::ir::BinaryOp::Mod => 0x97,
    }
}

pub fn alu64_reg(op: crate::ir::BinaryOp) -> u8 {
    alu64_imm(op) | 0x08
}

pub fn ldx_mem(size: u8) -> Result<u8, String> {
    match size {
        1 => Ok(0x71),
        2 => Ok(0x69),
        4 => Ok(0x61),
        8 => Ok(0x79),
        _ => Err(format!("unsupported load size: {}", size)),
    }
}

pub fn stx_mem(size: u8) -> Result<u8, String> {
    match size {
        1 => Ok(0x73),
        2 => Ok(0x6b),
        4 => Ok(0x63),
        8 => Ok(0x7b),
        _ => Err(format!("unsupported store size: {}", size)),
    }
}
