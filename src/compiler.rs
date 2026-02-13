use crate::diagnostics::{CompileDiagnostic, DiagnosticBuilder, ErrorCategory, ErrorCode, Span, SourceManager};
use crate::parser;
use miette::{IntoDiagnostic, Report, WrapErr};
use std::fs;
use std::path::Path;

pub struct ErrorHandler {
    source_manager: SourceManager,
}

impl ErrorHandler {
    pub fn new() -> Self {
        Self {
            source_manager: SourceManager::new(),
        }
    }

    /// Register a source file
    pub fn add_file(&mut self, path: String, content: String) -> crate::diagnostics::FileId {
        self.source_manager.add_file(path, content)
    }

    /// Format and display a diagnostic error
    pub fn report_error(&self, diagnostic: CompileDiagnostic) -> Report {
        Report::from(diagnostic)
    }

    /// Create a parse error diagnostic
    pub fn parse_error(
        &self,
        message: impl Into<String>,
        span: &Span,
    ) -> Result<CompileDiagnostic, String> {
        DiagnosticBuilder::new(ErrorCategory::Parse, ErrorCode::UnexpectedToken, message)
            .build(span, &self.source_manager)
    }

    /// Create a semantic error diagnostic
    pub fn semantic_error(
        &self,
        code: ErrorCode,
        message: impl Into<String>,
        span: &Span,
    ) -> Result<CompileDiagnostic, String> {
        DiagnosticBuilder::new(ErrorCategory::Semantic, code, message)
            .build(span, &self.source_manager)
    }

    /// Create a lexical error diagnostic
    pub fn lexical_error(
        &self,
        code: ErrorCode,
        message: impl Into<String>,
        span: &Span,
    ) -> Result<CompileDiagnostic, String> {
        DiagnosticBuilder::new(ErrorCategory::Lexical, code, message)
            .build(span, &self.source_manager)
    }

    /// Create a codegen error diagnostic
    pub fn codegen_error(
        &self,
        message: impl Into<String>,
        span: &Span,
    ) -> Result<CompileDiagnostic, String> {
        DiagnosticBuilder::new(
            ErrorCategory::Codegen,
            ErrorCode::CodegenError,
            message,
        )
        .build(span, &self.source_manager)
    }
}

pub fn compile(input_path: &Path, _output_path: &Path) -> Result<(), miette::Report> {
    match input_path.extension().and_then(|e| e.to_str()) {
        Some("snx") => {}
        _ => {
            return Err(miette::miette!(
                "Invalid source file extension. Expected a .snx file, got: {}",
                input_path.display()
            ));
        }
    }

    if !input_path.exists() {
        return Err(miette::miette!(
            "Input file does not exist: {}",
            input_path.display()
        ));
    }

    let src = fs::read_to_string(input_path)
        .into_diagnostic()
        .wrap_err(format!(
            "Failed to read input file: {}",
            input_path.display()
        ))?;

    if src.is_empty() {
        return Err(miette::miette!("Empty input source code"));
    }

    let mut error_handler = ErrorHandler::new();
    let _file_id = error_handler.add_file(input_path.display().to_string(), src.clone());

    let _program = match parser::parse(&src) {
        Ok(prog) => prog,
        Err(e) => {
            let span = Span::new(_file_id, e.0.offset..e.0.offset + 1 );
            match error_handler.parse_error(e.1.clone(), &span) {
                Ok(diag) => {
                    let report = error_handler.report_error(diag);
                    eprintln!("{}", report);
                    return Err(report);
                }
                Err(_) => {
                    let report = miette::miette!("{}", e.1);
                    eprintln!("{}", report);
                    return Err(report);
                }
            }
        }
    };
    
    crate::build::build(input_path)
        .map_err(|e| miette::miette!("{}", e))
        .wrap_err("Build failed")?;

    Ok(())
}
