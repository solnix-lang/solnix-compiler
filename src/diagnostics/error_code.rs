//! Error codes for the Solnix compiler
//!
//! Each compilation error has a unique code (E0XXX) to help users
//! find documentation and solutions online.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // Lexical errors (E010x)
    UnterminatedComment = 0x0101,
    InvalidCharacter = 0x0102,
    UnterminatedString = 0x0103,
    InvalidEscapeSequence = 0x0104,
    InvalidNumber = 0x0105,

    // Parse errors (E020x)
    UnexpectedToken = 0x0201,
    UnexpectedEof = 0x0202,
    ExpectedToken = 0x0203,
    InvalidStatement = 0x0204,
    InvalidExpression = 0x0205,
    DuplicateProgram = 0x0206,
    InvalidMapDeclaration = 0x0207,
    InvalidFunctionDeclaration = 0x0208,

    // Semantic errors (E030x)
    UndefinedIdentifier = 0x0301,
    DuplicateIdentifier = 0x0302,
    TypeMismatch = 0x0303,
    InvalidType = 0x0304,
    InvalidSectionType = 0x0305,
    InvalidProgramType = 0x0306,
    MissingMainProgram = 0x0307,
    InvalidFunctionSignature = 0x0308,
    InvalidMapType = 0x0309,
    BorrowCheckerError = 0x030A,

    // Codegen errors (E040x)
    CodegenError = 0x0401,
    UnsupportedFeature = 0x0402,
    InvalidInstruction = 0x0403,

    // Optimizer errors (E050x)
    OptimizerError = 0x0501,

    // I/O errors (E060x)
    FileNotFound = 0x0601,
    FileReadError = 0x0602,
    FileWriteError = 0x0603,
}

impl ErrorCode {
    pub fn code(&self) -> String {
        format!("E{:04X}", *self as u32)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UnterminatedComment => "unterminated comment",
            Self::InvalidCharacter => "invalid character",
            Self::UnterminatedString => "unterminated string literal",
            Self::InvalidEscapeSequence => "invalid escape sequence",
            Self::InvalidNumber => "invalid number",
            Self::UnexpectedToken => "unexpected token",
            Self::UnexpectedEof => "unexpected end of file",
            Self::ExpectedToken => "expected token",
            Self::InvalidStatement => "invalid statement",
            Self::InvalidExpression => "invalid expression",
            Self::DuplicateProgram => "duplicate program declaration",
            Self::InvalidMapDeclaration => "invalid map declaration",
            Self::InvalidFunctionDeclaration => "invalid function declaration",
            Self::UndefinedIdentifier => "undefined identifier",
            Self::DuplicateIdentifier => "duplicate identifier",
            Self::TypeMismatch => "type mismatch",
            Self::InvalidType => "invalid type",
            Self::InvalidSectionType => "invalid section type",
            Self::InvalidProgramType => "invalid program type",
            Self::MissingMainProgram => "missing main program",
            Self::InvalidFunctionSignature => "invalid function signature",
            Self::InvalidMapType => "invalid map type",
            Self::BorrowCheckerError => "borrow checker error",
            Self::CodegenError => "code generation error",
            Self::UnsupportedFeature => "unsupported feature",
            Self::InvalidInstruction => "invalid instruction",
            Self::OptimizerError => "optimizer error",
            Self::FileNotFound => "file not found",
            Self::FileReadError => "file read error",
            Self::FileWriteError => "file write error",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.as_str())
    }
}
