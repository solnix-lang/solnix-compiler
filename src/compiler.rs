use crate::diagnostics::{
    CompileDiagnostic, DiagnosticBuilder, ErrorCategory, ErrorCode, SourceManager, Span,
};
use crate::lexer::Lexer;
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
        DiagnosticBuilder::new(ErrorCategory::Codegen, ErrorCode::CodegenError, message)
            .build(span, &self.source_manager)
    }
}

pub fn compile(input_path: &Path, output_path: &Path) -> Result<(), miette::Report> {
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

    let tokens = match Lexer::new(&src).tokenize() {
        Ok(tokens) => tokens,
        Err(e) => {
            let span = Span::new(_file_id, e.0.offset..e.0.offset + 1);
            let code = lexical_error_code(&e.1);

            match error_handler.lexical_error(code, e.1.clone(), &span) {
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

    let program = match parser::parse_tokens(tokens) {
        Ok(prog) => prog,
        Err(e) => {
            let span = Span::new(_file_id, e.0.offset..e.0.offset + 1);
            let error_result = error_handler.parse_error(e.1.clone(), &span);

            match error_result {
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

    if let Err(sem_err) = crate::sema::check_program(&program) {
        use crate::diagnostics::ErrorCode;
        use crate::parser::SourceLoc;

        // Determine a SourceLoc for the error
        let loc: SourceLoc = match &sem_err {
            crate::sema::SemanticError::MapError(me) => match me {
                crate::sema::map::MapValidationError::DuplicateMapName(_, l)
                | crate::sema::map::MapValidationError::InvalidMaxEntries(l)
                | crate::sema::map::MapValidationError::InvalidType(l)
                | crate::sema::map::MapValidationError::UnknownMapMethod(_, l)
                | crate::sema::map::MapValidationError::InvalidMapMethodArity(_, _, _, l)
                | crate::sema::map::MapValidationError::UnknownMapName(_, l)
                | crate::sema::map::MapValidationError::UnexpectedKeyValue(l)
                | crate::sema::map::MapValidationError::MissingKey(l)
                | crate::sema::map::MapValidationError::MissingValue(l) => *l,
            },
            crate::sema::SemanticError::EventError(ee) => match ee {
                crate::sema::event::EventValidationError::DuplicateEventName(_, l)
                | crate::sema::event::EventValidationError::EmptyEvent(_, l)
                | crate::sema::event::EventValidationError::DuplicateField(_, l)
                | crate::sema::event::EventValidationError::ZeroLengthArray(l)
                | crate::sema::event::EventValidationError::NestedArray(l)
                | crate::sema::event::EventValidationError::EventTooLarge(_, _, l) => *l,
            },
            crate::sema::SemanticError::UnitError(ue) => match ue {
                crate::sema::unit::UnitValidationError::EmptyUnitName(l)
                | crate::sema::unit::UnitValidationError::NoSections(l)
                | crate::sema::unit::UnitValidationError::InvalidSection(l)
                | crate::sema::unit::UnitValidationError::MissingLicense(l)
                | crate::sema::unit::UnitValidationError::NoReturnOrInstructions(l) => *l,
            },
        };

        let span = Span::new(_file_id, loc.offset..loc.offset + 1);

        // Map semantic error to an ErrorCode and message
        let (code, msg) = match &sem_err {
            crate::sema::SemanticError::MapError(me) => match me {
                crate::sema::map::MapValidationError::DuplicateMapName(name, _) => (
                    ErrorCode::DuplicateIdentifier,
                    format!("Duplicate map name: '{}'", name),
                ),

                crate::sema::map::MapValidationError::InvalidMaxEntries(_) => (
                    ErrorCode::InvalidMapDeclaration,
                    "Map 'max_entries' must be greater than zero".to_string(),
                ),

                crate::sema::map::MapValidationError::InvalidType(_) => {
                    (ErrorCode::InvalidMapType, "Invalid map type".to_string())
                }

                crate::sema::map::MapValidationError::UnknownMapName(name, _) => (
                    ErrorCode::InvalidMapDeclaration, // change if you have a better code
                    format!("Unknown map '{}'", name),
                ),

                crate::sema::map::MapValidationError::UnknownMapMethod(method, _) => (
                    ErrorCode::InvalidMapDeclaration, // change if you have a better code
                    format!("Unknown map method '{}'", method),
                ),

                crate::sema::map::MapValidationError::InvalidMapMethodArity(
                    method,
                    expected,
                    got,
                    _,
                ) => (
                    ErrorCode::InvalidMapDeclaration, // change if you have a better code
                    format!(
                        "Map method '{}' expects {} args, got {}",
                        method, expected, got
                    ),
                ),
                crate::sema::map::MapValidationError::MissingKey(_) => (
                    ErrorCode::InvalidMapDeclaration,
                    "Map missing required field: key".to_string(),
                ),

                crate::sema::map::MapValidationError::MissingValue(_) => (
                    ErrorCode::InvalidMapDeclaration,
                    "Map missing required field: value".to_string(),
                ),

                crate::sema::map::MapValidationError::UnexpectedKeyValue(_) => (
                    ErrorCode::InvalidMapDeclaration,
                    "This map type must not define key/value".to_string(),
                ),
            },
            crate::sema::SemanticError::EventError(ee) => match ee {
                crate::sema::event::EventValidationError::DuplicateEventName(name, _) => (
                    ErrorCode::DuplicateIdentifier,
                    format!("Duplicate event name: '{}'", name),
                ),
                crate::sema::event::EventValidationError::EmptyEvent(name, _) => (
                    ErrorCode::InvalidEventDeclaration,
                    format!("Event '{}' must contain at least one field", name),
                ),
                crate::sema::event::EventValidationError::DuplicateField(name, _) => (
                    ErrorCode::InvalidEventDeclaration,
                    format!("Duplicate field '{}' in event", name),
                ),
                crate::sema::event::EventValidationError::ZeroLengthArray(_) => (
                    ErrorCode::InvalidEventDeclaration,
                    "Array length must be greater than zero".to_string(),
                ),
                crate::sema::event::EventValidationError::NestedArray(_) => (
                    ErrorCode::InvalidEventDeclaration,
                    "Nested arrays are not allowed in event fields".to_string(),
                ),
                crate::sema::event::EventValidationError::EventTooLarge(name, size, _) => (
                    ErrorCode::InvalidEventDeclaration,
                    format!(
                        "Event '{}' exceeds maximum allowed size ({} bytes)",
                        name, size
                    ),
                ),
            },
            crate::sema::SemanticError::UnitError(ue) => match ue {
                crate::sema::unit::UnitValidationError::EmptyUnitName(_) => (
                    ErrorCode::InvalidProgramType,
                    "Unit name cannot be empty".to_string(),
                ),
                crate::sema::unit::UnitValidationError::NoSections(_) => (
                    ErrorCode::InvalidSectionType,
                    "Unit must have at least one section".to_string(),
                ),
                crate::sema::unit::UnitValidationError::InvalidSection(_) => (
                    ErrorCode::InvalidSectionType,
                    "Invalid section name".to_string(),
                ),
                crate::sema::unit::UnitValidationError::MissingLicense(_) => (
                    ErrorCode::InvalidProgramType,
                    "License is required for eBPF programs".to_string(),
                ),
                crate::sema::unit::UnitValidationError::NoReturnOrInstructions(_) => (
                    ErrorCode::InvalidProgramType,
                    "Unit must have at least one return statement or instruction".to_string(),
                ),
            },
        };

        match error_handler.semantic_error(code, msg, &span) {
            Ok(diag) => {
                let report = error_handler.report_error(diag);
                eprintln!("{}", report);
                return Err(report);
            }
            Err(_) => {
                let report = miette::miette!("Semantic error: {}", sem_err);
                eprintln!("{}", report);
                return Err(report);
            }
        }
    }

    let program_ir = crate::ir::lower_program(&program)
        .map_err(|e| miette::miette!("{:?}", e))
        .wrap_err("Lowering failed")?;

    // Emit native eBPF ELF object with stage-aware error handling.
    if let Err(e) = crate::emit::ebpf::emit_program(&program_ir, output_path) {
        let span = Span::new(_file_id, 0..1);
        let error_msg = format!("Code generation failed: {}", e);

        match error_handler.codegen_error(error_msg.clone(), &span) {
            Ok(diag) => {
                let report = error_handler.report_error(diag);
                eprintln!("{}", report);
                return Err(report);
            }
            Err(_) => {
                let report = miette::miette!("{}", error_msg);
                eprintln!("{}", report);
                return Err(report);
            }
        }
    }

    Ok(())
}

fn lexical_error_code(message: &str) -> ErrorCode {
    match message {
        msg if msg.contains("Unterminated comment") => ErrorCode::UnterminatedComment,
        msg if msg.contains("Unterminated string") => ErrorCode::UnterminatedString,
        msg if msg.contains("Invalid escape sequence") => ErrorCode::InvalidEscapeSequence,
        _ => ErrorCode::InvalidCharacter,
    }
}
