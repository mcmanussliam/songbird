//! Encryption primitives. See system-design.md §8 for the full key-management design.
//!
//! This crate has NO knowledge of CalDAV, storage, or sync — pure, independently auditable
//! crypto primitives only. See AGENTS.md rule 2: a server-side decryption key must never
//! exist anywhere in this system. Nothing in this crate should make that possible.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("encryption failed")]
    EncryptFailed,
    #[error("decryption failed (wrong key, tampered ciphertext, or corrupt data)")]
    DecryptFailed,
}

/// Per-calendar symmetric content key (XChaCha20-Poly1305). TODO(M4): implement
/// generate/encrypt/decrypt, plus the X25519 ECDH + HKDF envelope-wrapping flow used during
/// group invites (system-design.md §8.3) and the on-device push-delta decrypt flow (§7.6, §15.5).
pub struct CalendarKey {
    _private: (),
}

impl CalendarKey {
    pub fn generate() -> Self {
        unimplemented!("CalendarKey::generate not yet implemented (M4)")
    }
}
