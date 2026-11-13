//! The names of the accounts the data keeps (`accounts`).

/// The id of the user's own account.
pub const PRINCIPAL: &str = "principal";

/// How far the accounts are brought (`Settings::accounts_version`): 1 since only the accounts
/// added exist (ESC-128, `migrate`).
pub const ACCOUNTS_VERSION: u32 = 1;
