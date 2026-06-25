use crate::parser::SourceLoc;

#[derive(Debug, Clone)]
pub struct EventDecl {
    pub name: String,
    pub fields: Vec<EventField>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
pub struct EventField {
    pub name: String,
    pub ty: EventType,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventType {
    /// 32-bit unsigned integer
    U32,

    /// 64-bit unsigned integer
    U64,

    /// 32-bit signed integer
    I32,

    /// 64-bit signed integer
    I64,

    /// Fixed-size byte array (e.g. bytes[256])
    Bytes(u32),

    /// Fixed-size integer array (e.g. u32[16])
    Array { elem: PrimitiveType, len: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    U32,
    U64,
    I32,
    I64,
}
