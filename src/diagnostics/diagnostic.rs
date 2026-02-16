use super::category::ErrorCategory;
use super::error_code::ErrorCode;
use super::source_manager::{Span, SourceManager};
use miette::{Diagnostic, SourceSpan};
use thiserror::Error;

#[derive(Error, Debug, Diagnostic)]
#[error("{category} error: {code}\n  {message}")]
pub struct CompileDiagnostic {
    #[source_code]
    pub file_content: String,

    #[label("{label_message}")]
    pub span: SourceSpan,

    pub code: String,
    pub category: ErrorCategory,
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
        })
    }
}

impl CompileDiagnostic {
    
}
