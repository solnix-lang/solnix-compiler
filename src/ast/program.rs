use super::{MapDecl, Unit};

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Program {
    pub maps: Vec<MapDecl>,
    pub units: Vec<Unit>,
    pub events: Vec<crate::ast::EventDecl>,
}