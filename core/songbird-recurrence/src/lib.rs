//! RFC 5545 §3.3.10 recurrence rule parsing and expansion.
//!
//! See system-design.md §5.5. No internal workspace dependencies — independently
//! testable and eventually publishable (AGENTS.md rule 1).
//!
//! Conformance: all public functions are exercised by core/tests/conformance/ (AGENTS.md rule 3).

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
