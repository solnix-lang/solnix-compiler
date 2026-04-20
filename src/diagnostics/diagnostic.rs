use super::category::{ErrorCategory, ErrorType};
use super::error_code::ErrorCode;
use super::source_manager::{SourceManager, Span};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("{error_type}: {code}\n  {message}")]
pub struct CompileDiagnostic {
    #[source_code]
    pub src: Option<miette::NamedSource<std::sync::Arc<String>>>,

    #[label("{label_message}")]
    pub span: SourceSpan,

    pub code: String,
    pub category: ErrorCategory,
    pub error_type: ErrorType,
    pub message: String,
    pub label_message: String,
}

pub struct DiagnosticBuilder {
    category: ErrorCategory,
    code: ErrorCode,
    message: String,
    label_message: Option<String>,
}

impl DiagnosticBuilder {
    pub fn new(category: ErrorCategory, code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            category,
            code,
            message: message.into(),
            label_message: None,
        }
    }

    pub fn build(
        self,
        span: &Span,
        source_manager: &SourceManager,
    ) -> Result<CompileDiagnostic, String> {
        let src = source_manager
            .get_named_source(span.file)
            .ok_or("File not found")?;

        let label_message = self
            .label_message
            .unwrap_or_else(|| String::from("error here"));

        Ok(CompileDiagnostic {
            src: Some(src),
            span: span.to_source_span(),
            code: self.code.code(),
            category: self.category,
            error_type: self.category.to_error_type(),
            message: self.message,
            label_message,
        })
    }
}

impl CompileDiagnostic {}
