//! Error categories for the Solnix compiler
//!
//! Each error is classified into one of these categories to help users
//! understand where in the compilation pipeline the error occurred.

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
    /// Errors from the optimizer phase
    Optimizer,
    /// I/O errors
    Io,
}

impl ErrorCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Lexical => "Lexical",
            Self::Parse => "Parse",
            Self::Semantic => "Semantic",
            Self::Codegen => "Codegen",
            Self::Optimizer => "Optimizer",
            Self::Io => "I/O",
        }
    }
}

impl std::fmt::Display for ErrorCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
