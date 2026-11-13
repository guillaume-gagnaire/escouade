//! The network settings a connection follows (`tls::Net`, audit SEC-07), as the settings give them: the app's `tls`
//! builds its TLS configuration from them, and `Settings::claude_env` reads whether a proxy is set for the agents.

/// The network settings a connection follows: the proxy, who goes around it, whether a
/// certificate is checked through it, and the file of extra authorities.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct Net {
    pub proxy: String,
    pub no_proxy: String,
    /// « Ignorer la vérification des certificats TLS »: through the proxy alone.
    pub insecure: bool,
    /// The path of a `.pem` file of authorities trusted as well; empty, none.
    pub extra_ca: String,
}

impl Net {
    pub fn of(settings: &crate::model::Settings) -> Self {
        Self {
            proxy: settings.proxy_url.trim().to_string(),
            no_proxy: settings.no_proxy.clone(),
            insecure: settings.insecure_tls,
            extra_ca: settings.extra_ca_file.trim().to_string(),
        }
    }

    /// Whether the certificates are left unchecked for some connections: the setting is on and a
    /// proxy is configured (with none, the setting has no effect).
    pub fn unchecked_by_proxy(&self) -> bool {
        self.insecure && proxy_url(self).is_some()
    }
}

/// The proxy of the settings, as an address.
pub fn proxy_url(net: &Net) -> Option<url::Url> {
    if net.proxy.is_empty() {
        return None;
    }
    let raw = if net.proxy.contains("://") {
        net.proxy.clone()
    } else {
        format!("http://{}", net.proxy)
    };
    url::Url::parse(&raw).ok()
}
