//! RFC 5545 §3.3.10 recurrence rule parsing and expansion.
//!
//! See system-design.md §5.5. This crate is intentionally dependency-free of every other
//! crate in this workspace — it should be independently testable, and eventually
//! independently publishable/reusable outside this project.
//!
//! Conformance: every public function here is exercised by the fixtures in
//! `core/tests/conformance/` (see AGENTS.md rule 3 — that suite must stay at 100%).

use thiserror::Error;

#[derive(Debug, Error)]
pub enum RecurrenceError {
    #[error("invalid RRULE string: {0}")]
    InvalidRrule(String),
    #[error("unsupported recurrence feature: {0}")]
    Unsupported(String),
}

/// Parsed RRULE, per RFC 5545 §3.3.10.
/// TODO(M1): fill in FREQ, INTERVAL, COUNT, UNTIL, BYDAY (incl. negative ordinals like -1SA),
/// BYMONTHDAY, BYMONTH, BYSETPOS, BYYEARDAY, BYWEEKNO, WKST.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecurrenceRule {
    pub raw: String,
}

/// TODO(M1): implement against RFC 5545 §3.3.10. This is the single highest-leverage piece
/// of engineering in the whole project — see system-design.md §2.1 / §5.5 / §11.1 before
/// writing this.
pub fn parse_rrule(raw: &str) -> Result<RecurrenceRule, RecurrenceError> {
    Err(RecurrenceError::Unsupported(format!(
        "parse_rrule not yet implemented (M1) — input was: {raw}"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_until_m1_lands() {
        // Replace with real assertions once parse_rrule is implemented. The conformance
        // fixtures in core/tests/conformance/ are the authoritative test list for this crate.
        assert!(parse_rrule("FREQ=WEEKLY").is_err());
    }
}
