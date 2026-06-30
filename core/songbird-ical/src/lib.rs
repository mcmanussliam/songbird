//! RFC 5545/5546 iCalendar parse/serialize. See system-design.md §5.6.
//!
//! Design constraints (do not relax these without an ADR):
//! - Round-trip fidelity: parse → serialize → re-parse must produce an equivalent result that
//!   re-imports cleanly into the source server (this is the exact property Fossify's EXDATE
//!   export bug violated — see system-design.md §2.1 / §11.1).
//! - Preserve unknown/vendor X- properties on round-trip where feasible.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IcalError {
    #[error("malformed iCalendar content: {0}")]
    Malformed(String),
}

/// A single parsed VEVENT. TODO(M1): flesh out to match the Event entity in
/// system-design.md §5.3 closely — this type should mirror RFC 5545 fields directly
/// rather than inventing a parallel internal model.
#[derive(Debug, Clone)]
pub struct VEvent {
    pub uid: String,
    pub raw: String,
}

/// TODO(M1): implement.
pub fn parse_vevent(_ics: &str) -> Result<VEvent, IcalError> {
    Err(IcalError::Malformed("parse_vevent not yet implemented (M1)".into()))
}

/// TODO(M1): implement. Must satisfy the round-trip fixtures in
/// core/tests/conformance/exdate_value_type_match.ics.
pub fn serialize_vevent(_event: &VEvent) -> String {
    unimplemented!("serialize_vevent not yet implemented (M1)")
}
