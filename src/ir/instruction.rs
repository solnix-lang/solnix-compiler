use crate::ast::Type;

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

    // memory
    LoadKey,
    Store { size: u8 },

    // ctx
    LoadCtx { offset: i32, size: u8 },
    LoadPacket { offset: i32, size: u8 },

    CopyCtxToMem { offset: i32, size: u32 },

    // checks
    NullCheck,

    // alu
    Binary { op: BinaryOp },

    // maps
    CallMap { map_name: String },   // lookup -> returns pointer (u64)
    UpdateMap { map_name: String }, // update(key, value) -> returns status (u64)

    RingBufReserve { map_name: String, size: u32 },
    RingBufSubmit { map_name: String },
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

#[derive(Debug, Clone)]
pub enum Operand {
    Var(VarId),
    Immediate(i64),
}
