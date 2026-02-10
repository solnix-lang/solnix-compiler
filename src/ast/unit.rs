use crate::parser::SourceLoc;

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Unit {
    pub name: String,
    pub loc: SourceLoc,
    pub sections: Vec<String>,
    pub license: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Stmt {
    pub kind: StmtKind,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub enum StmtKind {
    Return(Box<Expr>),
    VarDecl(VarDecl),
    HeapVarDecl(HeapVarDecl),
    Assignment(Assignment),
    IfGuard(IfGuard),
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Assignment {
    pub target: Box<Expr>,
    pub op: AssignmentOp,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(unused)]
pub enum AssignmentOp {
    Assign,
    AddAssign,
    SubAssign,
    MulAssign,
    DivAssign,
    ModAssign,
}

#[derive(Debug, Clone)]
pub struct IfGuard {
    pub condition: Expr,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct Expr {
    pub kind: ExprKind,
    pub loc: SourceLoc,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Shl,
    Shr,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct BinaryExpr {
    pub op: BinOp,
    pub left: Box<Expr>,
    pub right: Box<Expr>,
}

#[derive(Debug, Clone)]
pub struct CallExpr {
    pub name: String,
    pub args: Vec<Expr>,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub enum ExprKind {
    Variable(String),
    Number(i64),
    MethodCall(MethodCall),
    HeapLookup(HeapLookup),
    Dereference(Box<Expr>),
    Binary(BinaryExpr),
    Call(CallExpr),
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct VarDecl {
    pub name: String,
    pub var_type: VarType,
    pub value: Box<Expr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(unused)]
pub enum VarType {
    Reg,
    Imm,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct HeapLookup {
    pub map_name: String,
    pub key_expr: Box<Expr>,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct MethodCall {
    pub receiver: String,
    pub method: String,
    pub arg: Box<Expr>,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct HeapVarDecl {
    pub name: String,
    pub lookup: HeapLookup,
}
