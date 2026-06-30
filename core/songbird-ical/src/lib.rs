//! RFC 5545/5546 iCalendar parse/serialize. See system-design.md §5.6.
//!
//! Design constraints (do not relax without an ADR):
//! - Round-trip fidelity: parse → serialize → re-parse must produce an equivalent result.
//! - Preserve unknown/vendor X- properties on round-trip.
//! - EXDATE VALUE type must match DTSTART VALUE type on serialize (fixes Fossify bug class).

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
