use crate::ast::Unit;
use crate::parser::SourceLoc;
use crate::sema::section::SectionValidator;

#[derive(Debug, thiserror::Error)]
pub enum UnitValidationError {
    #[error("Unit name cannot be empty")]
    EmptyUnitName(SourceLoc),

    #[error("Unit must have at least one section")]
    NoSections(SourceLoc),

    #[error("Invalid section name")]
    InvalidSection(SourceLoc),

    #[error("License is required for eBPF programs")]
    MissingLicense(SourceLoc),

    #[error("Unit must have at least one return statement or instruction")]
    NoReturnOrInstructions(SourceLoc),
}

pub fn check_unit(unit: &Unit) -> Result<(), UnitValidationError> {
    if unit.name.is_empty() {
        return Err(UnitValidationError::EmptyUnitName(unit.loc));
    }

    if unit.sections.is_empty() {
        return Err(UnitValidationError::NoSections(unit.loc));
    }

    for section in &unit.sections {
        if !SectionValidator::is_valid(section) {
            return Err(UnitValidationError::InvalidSection(unit.loc));
        }
    }

    let license = unit
        .license
        .as_deref()
        .ok_or_else(|| UnitValidationError::MissingLicense(unit.loc))?;

    let valid_licenses = ["GPL", "Dual BSD/GPL", "GPL v2", "GPL-2.0"];
    if !valid_licenses.contains(&license) {
        // unknown license — warn not emitted here; caller may choose to warn
    }

    if unit.body.is_empty() {
        return Err(UnitValidationError::NoReturnOrInstructions(unit.loc));
    }

    Ok(())
}
