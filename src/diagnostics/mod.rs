//! Diagnostic system for the Solnix compiler
//!
//! This module provides a unified diagnostic system for reporting errors
//! and other compilation information. It handles:
//!
//! - **Error Categories**: Lexical, Parse, Semantic, Codegen, Optimizer, I/O
//! - **Error Codes**: Unique codes (E0XXX) for each error type
//! - **Source Management**: File tracking and span information
//! - **Diagnostic Builder**: Fluent API for creating diagnostics
//!
//! # Example
//!
//! ```no_run
//! use solnixc::diagnostics::{
//!     CompileDiagnostic, DiagnosticBuilder, ErrorCode, ErrorCategory,
//!     Span, SourceManager,
//! };
//!
//! let mut source_manager = SourceManager::new();
//! let file_id = source_manager.add_file(
//!     "example.snx".to_string(),
//!     "let = 42".to_string(),
//! );
//! let span = Span::new(file_id, 4..5);
//!
//! let diagnostic = DiagnosticBuilder::new(
//!     ErrorCategory::Parse,
//!     ErrorCode::UnexpectedToken,
//!     "unexpected token",
//! )
//! .with_label("unexpected here")
//! .build(&span, &source_manager)
//! .expect("should build");
//! ```

pub mod category;
pub mod diagnostic;
pub mod error_code;
pub mod source_manager;

// Re-export commonly used types
pub use category::ErrorCategory;
pub use diagnostic::{CompileDiagnostic, DiagnosticBuilder};
pub use error_code::ErrorCode;
pub use source_manager::{FileId, Span, SourceManager};
