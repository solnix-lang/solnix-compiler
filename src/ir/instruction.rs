use crate::ast::Type;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Instruction {
    pub result: VarId,
    pub opcode: Opcode,
    pub operands: Vec<Operand>,
    pub result_type: Type,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(pub u32);

#[derive(Debug, Clone)]
pub enum Opcode {
    HelperCall { id: u32 },
    LoadKey,
    Store { size: u8 },
    LoadCtx { offset: i32, size: u8 },
    LoadPacket { offset: i32, size: u8 },
    NullCheck,
    Binary { op: BinaryOp },
    CallMap { map_name: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Shl,
    Shr,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub enum Operand {
    Var(VarId),
    Immediate(i64),
}
