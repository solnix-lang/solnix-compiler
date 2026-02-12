pub mod instruction;
pub mod program;
pub mod unit;
pub use program::lower_program;
pub mod ctx;


pub use instruction::{
    Instruction,
    VarId,
    Opcode,
    BinaryOp,
    Operand,
};

pub use program::{
    ProgramIr,
};

pub use unit::{
    UnitIr,
};

#[derive(Debug, thiserror::Error)]
pub enum LoweringError {
    #[error("Failed to lower unit: {0}")]
    UnitLowering(String),

    #[error("Invalid operand in expression")]
    InvalidOperand,
}
