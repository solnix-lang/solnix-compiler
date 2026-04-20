
use crate::parser::SourceLoc;

#[derive(Debug, Clone)]
pub struct MapDecl {
    pub name: String,
    pub map_type: MapType,
    pub key_type: Option<Type>,
    pub value_type: Option<Type>,
    pub max_entries: Option<u32>,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MapType {
    Hash,
    Array,
    Ringbuf,
    LruHash,    
    ProgArray,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    U32,
    U64,
    I32,
    I64,
}