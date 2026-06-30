//! Encryption primitives. No knowledge of CalDAV, storage, or sync — pure, independently
//! auditable crypto. A server-side decryption key must never exist anywhere in this system.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum CryptoError {
    #[error("encryption failed")]
    EncryptFailed,
    #[error("decryption failed (wrong key, tampered ciphertext, or corrupt data)")]
    DecryptFailed,
}

/// Per-calendar symmetric content key (XChaCha20-Poly1305).
/// TODO(M4): implement generate/encrypt/decrypt, plus the X25519 ECDH + HKDF
/// envelope-wrapping flow for group invites and the on-device push-delta decrypt flow.
pub struct CalendarKey {
    _private: (),
}

impl CalendarKey {
    pub fn generate() -> Self {
        unimplemented!("CalendarKey::generate not yet implemented (M4)")
    }
}
