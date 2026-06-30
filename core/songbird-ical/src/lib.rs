//! RFC 5545/5546 iCalendar parse/serialize.
//!
//! Invariants:
//! - Round-trip fidelity: parse → serialize → re-parse must produce an equivalent result.
//! - Preserve unknown/vendor X- properties on round-trip.
//! - EXDATE VALUE type must match DTSTART VALUE type on serialize.

mod parse;
mod serialize;
pub mod types;

pub use parse::{parse_icalendar, parse_vevent};
pub use serialize::{serialize_icalendar, serialize_vevent};
pub use types::*;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IcalError {
    #[error("malformed iCalendar content: {0}")]
    Malformed(String),
}
