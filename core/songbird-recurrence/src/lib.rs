//! RFC 5545 §3.3.10 recurrence rule parsing and expansion.
//!
//! No internal workspace dependencies — independently testable and eventually publishable.
//! All public functions are exercised by the conformance suite in core/tests/conformance/.

mod expand;
mod rule;
pub mod types;

pub use expand::expand_occurrences;
pub use rule::parse_rrule;
pub use types::*;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecurrenceError {
    #[error("invalid RRULE string: {0}")]
    InvalidRrule(String),
    #[error("unsupported recurrence feature: {0}")]
    Unsupported(String),
}
