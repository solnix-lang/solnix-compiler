#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorType {
    /// Invalid token or lexical error (from Lexer stage)
    InvalidToken,
    /// Syntax error (from Parser stage)
    SyntaxError,
    /// Type error or semantic error (from Semantic stage)
    TypeError,
    /// Internal error from IR/Codegen stage
    InternalError,
}

impl ErrorType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidToken => "Invalid token",
            Self::SyntaxError => "Syntax error",
            Self::TypeError => "Type error",
            Self::InternalError => "Internal error",
        }
    }
}

impl std::fmt::Display for ErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorCategory {
    /// Errors from the lexical analysis phase
    Lexical,
    /// Errors from the parsing phase
    Parse,
    /// Errors from the semantic analysis phase (type checking, name resolution, etc.)
    Semantic,
    /// Errors from the code generation phase
    Codegen,
}

impl ErrorCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Lexical => "Lexical",
            Self::Parse => "Parse",
            Self::Semantic => "Semantic",
            Self::Codegen => "Codegen",
        }
    }

    /// Map compilation stage category to a clean, user-friendly error type
    pub fn to_error_type(&self) -> ErrorType {
        match self {
            Self::Lexical => ErrorType::InvalidToken,
            Self::Parse => ErrorType::SyntaxError,
            Self::Semantic => ErrorType::TypeError,
            Self::Codegen => ErrorType::InternalError,
        }
    }
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
