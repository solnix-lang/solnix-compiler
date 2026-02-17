use crate::ast::{EventDecl, EventType, PrimitiveType};
use crate::parser::SourceLoc;
use std::collections::HashSet;

#[derive(Debug, thiserror::Error)]
pub enum EventValidationError {

    #[error("Duplicate event name: {0}")]
    DuplicateEventName(String, SourceLoc),

    #[error("Event '{0}' must contain at least one field")]
    EmptyEvent(String, SourceLoc),

    #[error("Duplicate field '{0}' in event")]
    DuplicateField(String, SourceLoc),

    #[error("Array length must be greater than zero")]
    ZeroLengthArray(SourceLoc),

    #[error("Nested arrays are not allowed in event fields")]
    NestedArray(SourceLoc),

    #[error("Event '{0}' exceeds maximum allowed size ({1} bytes)")]
    EventTooLarge(String, u32, SourceLoc),
}

const MAX_EVENT_SIZE: u32 = 512;

pub fn check_event(
    event: &EventDecl,
    event_names: &mut HashSet<String>,
) -> Result<(), EventValidationError> {

    // 1️Duplicate event name
    if !event_names.insert(event.name.clone()) {
        return Err(EventValidationError::DuplicateEventName(
            event.name.clone(),
            event.loc,
        ));
    }

    // 2️Empty event
    if event.fields.is_empty() {
        return Err(EventValidationError::EmptyEvent(
            event.name.clone(),
            event.loc,
        ));
    }

    let mut field_names = HashSet::new();

    let mut offset: u32 = 0;
    let mut max_align: u32 = 1;

    for field in &event.fields {

        // 3️Duplicate field name
        if !field_names.insert(field.name.clone()) {
            return Err(EventValidationError::DuplicateField(
                field.name.clone(),
                field.loc,
            ));
        }

        // 4️Validate field type + get layout
        let (size, align) = validate_event_type(&field.ty, field.loc)?;

        max_align = max_align.max(align);

        // 5️Apply alignment padding
        offset = align_up(offset, align);

        offset += size;
    }

    // 6️ Final struct alignment
    offset = align_up(offset, max_align);

    // 7️ Total size check
    if offset > MAX_EVENT_SIZE {
        return Err(EventValidationError::EventTooLarge(
            event.name.clone(),
            offset,
            event.loc,
        ));
    }

    Ok(())
}

fn validate_event_type(
    ty: &EventType,
    loc: SourceLoc,
) -> Result<(u32, u32), EventValidationError> {

    match ty {

        EventType::U32 | EventType::I32 => Ok((4, 4)),

        EventType::U64 | EventType::I64 => Ok((8, 8)),

        EventType::Bytes(len) => {
            if *len == 0 {
                return Err(EventValidationError::ZeroLengthArray(loc));
            }

            // bytes are byte-aligned
            Ok((*len, 1))
        }

        EventType::Array { elem, len } => {
            if *len == 0 {
                return Err(EventValidationError::ZeroLengthArray(loc));
            }

            // Nested array detection safeguard (future-proof)
            // Since elem is PrimitiveType, nesting is impossible now,
            // but keeping guard makes compiler robust for future AST changes.
            if matches!(elem, _) == false {
                return Err(EventValidationError::NestedArray(loc));
            }

            let (elem_size, elem_align) = primitive_layout(elem);

            Ok((elem_size * len, elem_align))
        }
    }
}

fn primitive_layout(p: &PrimitiveType) -> (u32, u32) {
    match p {
        PrimitiveType::U32 | PrimitiveType::I32 => (4, 4),
        PrimitiveType::U64 | PrimitiveType::I64 => (8, 8),
    }
}

#[inline]
fn align_up(offset: u32, align: u32) -> u32 {
    (offset + align - 1) & !(align - 1)
}


pub fn compute_event_size(event: &EventDecl) -> u32 {
    let mut offset: u32 = 0;
    let mut max_align: u32 = 1;

    for field in &event.fields {
        let (size, align) = validate_event_type(&field.ty, field.loc)
            .expect("Event should already be validated");

        max_align = max_align.max(align);

        offset = align_up(offset, align);
        offset += size;
    }

    align_up(offset, max_align)
}

pub fn compute_field_offset(event: &EventDecl, field_name: &str) -> Option<u32> {
    let mut offset: u32 = 0;
    let mut max_align: u32 = 1;

    // First pass: calculate alignments
    for field in &event.fields {
        let (_, align) = validate_event_type(&field.ty, field.loc)
            .expect("Event should already be validated");
        max_align = max_align.max(align);
    }

    // Second pass: calculate field offset
    for field in &event.fields {
        let (size, align) = validate_event_type(&field.ty, field.loc)
            .expect("Event should already be validated");

        offset = align_up(offset, align);

        if field.name == field_name {
            return Some(offset);
        }

        offset += size;
    }

    None
}