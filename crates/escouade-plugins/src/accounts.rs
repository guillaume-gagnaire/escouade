//! The names of an account's secret and of its budget, and the budget a plugin's answer carries (the app's
//! `plugins::accounts` serves the accounts).

use crate::runtime::secret_name;
use escouade_plugin_protocol::forge::RateLimit;
use serde_json::Value;

/// The keychain's name of the secret of the account `account` that `plugin` serves.
pub fn vault_name(plugin: &str, account: &str) -> String {
    secret_name(plugin, &format!("account:{account}"))
}

/// What a plugin says of an account's budget of calls, kept for that account: the calls of all
/// its points spend it (GitHub's 5,000 an hour are its token's, Issues and pull requests alike).
pub fn budget_key(plugin: &str, account: &str) -> String {
    vault_name(plugin, account)
}

/// The budget an answer of a plugin carries (`{ value, rateLimit }`), and its value.
pub fn split_budget(v: Value) -> (Value, Option<RateLimit>) {
    match v {
        Value::Object(mut o)
            if o.contains_key("value") && o.contains_key("rateLimit") && o.len() == 2 =>
        {
            let rate = o
                .remove("rateLimit")
                .and_then(|r| serde_json::from_value::<RateLimit>(r).ok());
            (o.remove("value").unwrap_or(Value::Null), rate)
        }
        v => (v, None),
    }
}
