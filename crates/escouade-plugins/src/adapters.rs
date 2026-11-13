//! The codes with which a plugin says an error of its service (the app's `plugins::adapters` reads them, the bench checks
//! them).

/// The code with which a plugin says that the service refused for good (`integrations::Refused`).
pub const REFUSED: i64 = -32010;

/// The code with which a plugin says the HTTP error `status` of its service (the Rust plugins of
/// the first hand answer with it; the tests too).
pub fn http_code(status: u16) -> i64 {
    -(32_000 + i64::from(status))
}

/// The status `code` says, when it is one of `http_code`'s.
pub fn http_status(code: i64) -> Option<u16> {
    u16::try_from(-code - 32_000)
        .ok()
        .filter(|s| (400..600).contains(s))
}
