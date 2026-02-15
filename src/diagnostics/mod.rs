pub mod category;
pub mod diagnostic;
pub mod error_code;
pub mod source_manager;

// Re-export commonly used types
pub use category::ErrorCategory;
pub use diagnostic::{CompileDiagnostic, DiagnosticBuilder};
pub use error_code::ErrorCode;
pub use source_manager::{FileId, Span, SourceManager};
