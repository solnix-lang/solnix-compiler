//! Diagnostic messages for compilation errors
//!
//! This module provides the unified diagnostic structure used throughout
//! the compiler for reporting errors to users.

use super::category::ErrorCategory;
use super::error_code::ErrorCode;
use super::source_manager::{Span, SourceManager};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

/// A compilation diagnostic (error, warning, or note)
#[derive(Error, Debug, Diagnostic)]
#[error("{category} error: {code}\n  {message}")]
pub struct CompileDiagnostic {
    #[source_code]
    #[allow(unused_assignments)]
    pub file_content: String,

    #[label("{label_message}")]
    pub span: SourceSpan,

    #[allow(dead_code)]
    pub code: String,
    pub category: ErrorCategory,
    pub message: String,
    pub label_message: String,
    #[allow(dead_code)]
    pub file_name: String,
}

/// Builder for creating diagnostics with fluent API
pub struct DiagnosticBuilder {
    category: ErrorCategory,
    code: ErrorCode,
    message: String,
    label_message: Option<String>,
    help: Option<String>,
}

impl DiagnosticBuilder {
    pub fn new(category: ErrorCategory, code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            category,
            code,
            message: message.into(),
            label_message: None,
            help: None,
        }
    }

    pub fn build(
        self,
        span: &Span,
        source_manager: &SourceManager,
    ) -> Result<CompileDiagnostic, String> {
        let file_name = source_manager
            .file_name(span.file)
            .ok_or("File not found")?
            .to_string();

        let file_content = source_manager
            .file_content(span.file)
            .ok_or("File content not found")?
            .to_string();

        let label_message = self
            .label_message
            .unwrap_or_else(|| String::from("error here"));

        Ok(CompileDiagnostic {
            file_content,
            span: span.to_source_span(),
            code: self.code.code(),
            category: self.category,
            message: self.message,
            label_message,
            file_name,
        })
    }
}

impl CompileDiagnostic {
    /// Create a new diagnostic with all required fields
    pub fn new(
        file_name: String,
        file_content: String,
        span: SourceSpan,
        code: ErrorCode,
        category: ErrorCategory,
        message: impl Into<String>,
        label_message: impl Into<String>,
    ) -> Self {
        Self {
            file_content,
            span,
            code: code.code(),
            category,
            message: message.into(),
            label_message: label_message.into(),
            file_name,
        }
    }
    pub fn code_str(&self) -> &str {
        &self.code
    }

    /// Get the category as a string
    pub fn category_str(&self) -> &str {
        self.category.as_str()
    }
}
