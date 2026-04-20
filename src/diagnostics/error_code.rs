#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCode {
    // Lexical errors (E010x)
    UnterminatedComment = 0x0101,
    InvalidCharacter = 0x0102,
    UnterminatedString = 0x0103,
    InvalidEscapeSequence = 0x0104,

    // Parse errors (E020x)
    UnexpectedToken = 0x0201,
    InvalidMapDeclaration = 0x0207,
    InvalidEventDeclaration = 0x0208,

    // Semantic errors (E030x)
    DuplicateIdentifier = 0x0302,
    InvalidSectionType = 0x0305,
    InvalidProgramType = 0x0306,
    InvalidMapType = 0x0309,

    // Codegen errors (E040x)
    CodegenError = 0x0401,
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
            Self::UnexpectedToken => "unexpected token",
            Self::InvalidMapDeclaration => "invalid map declaration",
            Self::InvalidEventDeclaration => "invalid event declaration",
            Self::DuplicateIdentifier => "duplicate identifier",
            Self::InvalidSectionType => "invalid section type",
            Self::InvalidProgramType => "invalid program type",
            Self::InvalidMapType => "invalid map type",
            Self::CodegenError => "code generation error",
        }
    }
}

impl std::fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code(), self.as_str())
    }
}
