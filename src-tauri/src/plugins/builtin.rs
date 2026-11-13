//! The registry of the plugins of Escouade's own hand, in `escouade-plugins` (docs/CRATES.md); here, what serves a code host
//! of the first hand to the app's tests through its plugin.

pub use escouade_plugins::builtin::*;

/// A client of the code host `kind` for the account `creds`, served by the plugin of the first hand for it (`plugin`), its
/// token lent: what the core does for an account of the first hand.
#[cfg(test)]
pub fn served_forge(
    plugin: &str,
    kind: &str,
    creds: &crate::forge::Credentials,
) -> Box<dyn crate::forge::Forge> {
    served_forge_on(plugin, kind, creds, &[])
}

/// `served_forge`, its module reaching the fake services of `routes` by the hosts of its manifest.
#[cfg(test)]
pub fn served_forge_on(
    plugin: &str,
    kind: &str,
    creds: &crate::forge::Credentials,
    routes: &[(&str, &str)],
) -> Box<dyn crate::forge::Forge> {
    let rt = test_runtime(kind);
    let host = crate::forge::host_of(&creds.url);
    rt.lend(plugin, &format!("account:{host}"), &creds.token);
    rt.set_accounts(std::sync::Arc::new(move |_| vec![host.clone()]));
    let mut reached = routes.to_vec();
    reached.push(("fake.test", creds.api.as_deref().unwrap_or(&creds.url)));
    let_reach(&rt, &reached);
    Box::new(super::adapters::ProcessForge::new(rt, plugin, kind, creds))
}
