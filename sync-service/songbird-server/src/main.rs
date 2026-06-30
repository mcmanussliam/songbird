//! Songbird sync service entrypoint. See system-design.md §7.
//!
//! Components (currently modules within this one crate — split into separate crates per
//! §7.3 if/when team size warrants it, that's a later decision, not a Phase 1 one):
//!   api_gateway    — request routing, authn/z, rate limiting
//!   auth           — account creation, device enrollment (§8.2)
//!   event_store    — encrypted event storage (§7.4) — server NEVER holds a decryption key
//!   push_relay     — UnifiedPush/APNs/FCM delivery, carries ciphertext deltas (§7.6, §15.5)
//!   caldav_gateway — Phase 2 (M6): read-only signed ICS links only (§9.3), never full CalDAV
//!   groups         — group/invite/membership management (§7.7)

mod api_gateway;
mod auth;
mod event_store;
mod push_relay;
mod groups;
// mod caldav_gateway; // Phase 2 / M6

#[tokio::main]
async fn main() {
    println!("songbird-server: not yet implemented (M4). See docs/design/system-design.md §7 and §14.");
}
