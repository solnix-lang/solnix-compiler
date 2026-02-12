use crate::ir::ctx::CtxMethod;
use crate::parser::SourceLoc;

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct Unit {
    pub name: String,
    pub loc: SourceLoc,
    pub sections: Vec<String>,
    pub kind: ProgramKind,   
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
pub enum ProgramKind {
    Xdp,
    Tc,
    Socket,
    Cgroup,
    Kprobe,
    Tracepoint,
    RawTracepoint,
    Fentry,
    Fexit,
    Lsm,
    Unknown,
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
    pub arg: Option<Box<Expr>>,
}

#[derive(Debug, Clone)]
#[allow(unused)]
pub struct HeapVarDecl {
    pub name: String,
    pub lookup: HeapLookup,
}

impl ProgramKind {
    pub fn from_section(section: &str) -> Self {
        match section {
            "xdp" => Self::Xdp,

            "tc" | "classifier" => Self::Tc,
            s if s.starts_with("tc") => Self::Tc,

            s if s.starts_with("sk_") => Self::Socket,

            s if s.starts_with("cgroup/") => Self::Cgroup,

            s if s.starts_with("kprobe/") || s.starts_with("kretprobe/") => Self::Kprobe,

            s if s.starts_with("tracepoint/") => Self::Tracepoint,

            s if s.starts_with("raw_tracepoint/") => Self::RawTracepoint,

            s if s.starts_with("fentry/") => Self::Fentry,
            s if s.starts_with("fexit/") => Self::Fexit,

            s if s.starts_with("lsm/") => Self::Lsm,

            _ => Self::Unknown,
        }
    }

    pub fn allows_ctx_method(self, method: CtxMethod) -> bool {
        self.allowed_ctx_methods().contains(&method)
    }

    pub fn allowed_ctx_methods(self) -> &'static [CtxMethod] {
        use CtxMethod::*;

        match self {
            ProgramKind::Xdp => &[
                LoadU8, LoadU16, LoadU32, LoadU64, LoadI8, LoadI16, LoadI32, LoadI64,
            ],

            ProgramKind::Tc => &[LoadU8, LoadU16, LoadU32, LoadU64],

            ProgramKind::Socket => &[LoadU8, LoadU16, LoadU32],

            ProgramKind::Kprobe => &[LoadU64, GetPidTgid, GetUidGid],

            ProgramKind::Tracepoint => &[LoadU64, GetPidTgid],

            ProgramKind::RawTracepoint => &[LoadU64],

            ProgramKind::Fentry | ProgramKind::Fexit => &[LoadU64],

            ProgramKind::Cgroup => &[LoadU32, LoadU64],

            ProgramKind::Lsm => &[LoadU64],

            ProgramKind::Unknown => &[],
        }
    }
}
