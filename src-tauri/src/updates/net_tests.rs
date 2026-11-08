//! The words and the probe of a failed request of the updater (`net`): what is said in both
//! languages, and what the probe reads of a fake server and a fake proxy.

use super::fake_server::{closed_port, hangs_up, Reply, Server};
use super::net::*;
use crate::i18n::Lang::{En, Fr};
use crate::model::Settings;
use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::Url;

const INSTALLER: &str =
    "https://github.com/guillaume-gagnaire/escouade/releases/download/v1.8.0/Escouade_1.8.0_x64-setup.exe";

/// The failure of the installer's download that the user of a corporate proxy meets.
fn refused() -> Failure {
    Failure {
        what: What::Download,
        url: INSTALLER.into(),
        status: Some(403),
        answered: true,
        host: "release-assets.githubusercontent.com".into(),
        proxy: Some("127.0.0.1:9000".into()),
        agent: Some("Zscaler/6.2".into()),
        page: Some("Access denied …".into()),
        detail: "Download request failed with status: 403 Forbidden".into(),
        stuck: None,
        trusted: Vec::new(),
        direct: None,
    }
}

fn unreachable_host() -> Failure {
    Failure {
        what: What::Check,
        url: "https://github.com/x/y/releases/latest/download/latest.json".into(),
        status: None,
        answered: false,
        host: "github.com".into(),
        proxy: Some("127.0.0.1:9000".into()),
        agent: None,
        page: None,
        detail: "tcp connect error: refused".into(),
        stuck: Some(Stuck::ProxyDown),
        trusted: Vec::new(),
        direct: None,
    }
}

fn url(s: &str) -> Url {
    s.parse().unwrap()
}

/// A network that goes through `proxy`, or nothing: never the system's, which a test cannot know.
fn net(proxy: Option<&Server>) -> Network {
    Network {
        proxy: proxy.map(|p| url(&format!("http://{}", p.address()))),
        system_proxy: false,
        insecure_tls: false,
    }
}

// ---------- settings and identity ----------

#[test]
fn the_user_agent_names_the_app_its_system_and_its_architecture() {
    let os = if cfg!(windows) {
        "Windows"
    } else if cfg!(target_os = "macos") {
        "macOS"
    } else {
        "Linux"
    };
    assert_eq!(
        user_agent(),
        format!(
            "Escouade/{} ({os}; {})",
            env!("CARGO_PKG_VERSION"),
            std::env::consts::ARCH
        )
    );
}

#[test]
fn the_network_follows_the_proxy_and_the_tls_setting() {
    let none = Network::from_settings(&Settings::default()).unwrap();
    assert!(none.proxy.is_none());
    assert!(!none.insecure_tls);
    // With no proxy set, the system's is followed, as before.
    assert!(none.system_proxy);
    assert_eq!(none.via(Route::Proxy), Via::System);
    assert_eq!(none.via(Route::Direct), Via::Nothing);
    let set = Settings {
        proxy_url: "  http://127.0.0.1:9000 ".into(),
        insecure_tls: true,
        ..Settings::default()
    };
    let net = Network::from_settings(&set).unwrap();
    assert_eq!(
        net.proxy.as_ref().unwrap().as_str(),
        "http://127.0.0.1:9000/"
    );
    assert!(net.insecure_tls);
    // The proxy of the settings is the route's, and no route without a proxy is one through it.
    assert_eq!(
        net.via(Route::Proxy),
        Via::Proxy(&"http://127.0.0.1:9000".parse().unwrap())
    );
    assert_eq!(net.via(Route::Direct), Via::Nothing);
    // A network that follows nothing (the tests').
    assert_eq!(Network::default().via(Route::Proxy), Via::Nothing);
}

#[test]
fn a_proxy_address_that_is_not_one_is_told_to_the_user() {
    let set = Settings {
        proxy_url: "127.0.0.1:9000".into(),
        ..Settings::default()
    };
    let e = Network::from_settings(&set).unwrap_err();
    assert!(
        e.to_string().starts_with("adresse du proxy invalide"),
        "{e}"
    );
}

// ---------- what a failure is ----------

#[test]
fn a_failure_is_passing_unless_the_server_said_no_for_good() {
    let with = |status: Option<u16>| Failure {
        status,
        ..refused()
    };
    // No answer (the network), or an answer that could not be read.
    assert!(with(None).transient());
    for code in [403, 404, 408, 425, 429, 500, 502, 503, 504, 599] {
        assert!(with(Some(code)).transient(), "{code}");
    }
    for code in [400, 401, 405, 410, 451] {
        assert!(!with(Some(code)).transient(), "{code}");
    }
    // A tunnel: refused, a block that may be lifted like a 403; but a password asked for does not
    // come by itself, as a 407 does not.
    let tunnel = |stuck| Failure {
        status: None,
        answered: false,
        stuck: Some(stuck),
        ..refused()
    };
    assert!(tunnel(Stuck::TunnelRefused).transient());
    assert!(tunnel(Stuck::ProxyDown).transient());
    assert!(!tunnel(Stuck::TunnelAuth).transient());
}

/// A failure that went through a proxy (or did not) and ended as told.
fn through(proxy: bool, status: Option<u16>, answered: bool, stuck: Option<Stuck>) -> Failure {
    Failure {
        proxy: proxy.then(|| "127.0.0.1:9000".into()),
        status,
        answered,
        stuck,
        ..refused()
    }
}

#[test]
fn only_a_proxy_that_is_not_there_is_gone_around() {
    // The proxy cannot be reached, or never answered its tunnel.
    assert!(through(true, None, false, Some(Stuck::ProxyDown)).proxy_unreachable());
    // What it answers is its rule, kept: a 403, a 407, a tunnel turned down.
    assert!(!through(true, Some(403), true, None).proxy_unreachable());
    assert!(!through(true, Some(407), true, None).proxy_unreachable());
    assert!(!through(true, None, false, Some(Stuck::TunnelRefused)).proxy_unreachable());
    assert!(!through(true, None, false, Some(Stuck::TunnelAuth)).proxy_unreachable());
    // Another way to fail through it (a certificate refused, a connection reset): not that.
    assert!(!through(true, None, false, None).proxy_unreachable());
    // An answer of the server, through it.
    assert!(!through(true, Some(404), true, None).proxy_unreachable());
    assert!(!through(true, None, true, None).proxy_unreachable());
    // Without a proxy there is none to go around.
    assert!(!through(false, None, false, Some(Stuck::ProxyDown)).proxy_unreachable());
    assert!(!through(false, None, false, None).proxy_unreachable());
}

#[test]
fn a_proxy_refuses_with_a_403_a_407_or_a_tunnel_turned_down_and_never_by_being_absent() {
    assert!(through(true, Some(403), true, None).refused_by_proxy());
    assert!(through(true, Some(407), true, None).refused_by_proxy());
    assert!(through(true, None, false, Some(Stuck::TunnelRefused)).refused_by_proxy());
    assert!(through(true, None, false, Some(Stuck::TunnelAuth)).refused_by_proxy());
    // Absent: nothing was refused.
    assert!(!through(true, None, false, Some(Stuck::ProxyDown)).refused_by_proxy());
    assert!(!through(true, None, false, None).refused_by_proxy());
    // Not through a proxy: the server's own refusal.
    assert!(!through(false, Some(403), true, None).refused_by_proxy());
    assert!(!through(false, None, false, Some(Stuck::TunnelRefused)).refused_by_proxy());
    // Through it, the server's answer: a missing file, a broken service, a page that is not JSON.
    assert!(!through(true, Some(404), true, None).refused_by_proxy());
    assert!(!through(true, Some(502), true, None).refused_by_proxy());
    assert!(!through(true, None, true, None).refused_by_proxy());
}

// ---------- what is said ----------

#[test]
fn a_refusal_names_the_code_the_domain_the_proxy_and_what_it_said_of_itself() {
    let f = Failure {
        what: What::Check,
        ..refused()
    };
    assert_eq!(
        f.describe(Fr),
        "403 Forbidden de release-assets.githubusercontent.com, par le proxy 127.0.0.1:9000 (Zscaler/6.2 : « Access denied … ») : bloqué par le réseau ou un proxy ?"
    );
    assert_eq!(
        f.describe(En),
        "403 Forbidden from release-assets.githubusercontent.com, through the proxy 127.0.0.1:9000 (Zscaler/6.2: “Access denied …”): blocked by the network or a proxy?"
    );
}

#[test]
fn the_parts_a_refusal_does_not_have_are_left_out_and_only_403_and_407_ask_whether_it_is_blocked() {
    let plain = |status: u16| Failure {
        what: What::Check,
        status: Some(status),
        proxy: None,
        agent: None,
        page: None,
        host: "github.com".into(),
        ..refused()
    };
    assert_eq!(
        plain(403).describe(Fr),
        "403 Forbidden de github.com : bloqué par le réseau ou un proxy ?"
    );
    assert_eq!(
        plain(407).describe(En),
        "407 Proxy Authentication Required from github.com: blocked by the network or a proxy?"
    );
    assert_eq!(plain(404).describe(Fr), "404 Not Found de github.com");
    assert_eq!(plain(502).describe(En), "502 Bad Gateway from github.com");
    // A code the HTTP registry has no name for.
    assert_eq!(plain(599).describe(En), "599 from github.com");
    let page = |agent: Option<&str>, page: Option<&str>| Failure {
        agent: agent.map(Into::into),
        page: page.map(Into::into),
        ..plain(404)
    };
    assert_eq!(
        page(Some("Squid/4.1"), None).describe(Fr),
        "404 Not Found de github.com (Squid/4.1)"
    );
    assert_eq!(
        page(None, Some("Page blocked")).describe(En),
        "404 Not Found from github.com (“Page blocked”)"
    );
    assert_eq!(
        page(None, Some("Bloquée")).describe(Fr),
        "404 Not Found de github.com (« Bloquée »)"
    );
}

#[test]
fn a_download_a_company_proxy_refused_says_so_and_sends_to_the_browser() {
    let f = refused();
    assert_eq!(
        f.describe(Fr),
        "le proxy de ton entreprise (Zscaler/6.2) a refusé l'installateur (403 Forbidden de release-assets.githubusercontent.com, par le proxy 127.0.0.1:9000). Télécharge-le dans le navigateur : il faudra peut-être confirmer l'avertissement du proxy."
    );
    assert_eq!(
        f.describe(En),
        "your company’s proxy (Zscaler/6.2) refused the installer (403 Forbidden from release-assets.githubusercontent.com, through the proxy 127.0.0.1:9000). Download it in the browser: you may have to confirm the proxy’s warning."
    );
}

#[test]
fn a_proxy_that_asks_for_a_password_says_where_to_put_it_for_a_check_and_a_download() {
    let fr = "le proxy 127.0.0.1:9000 demande une authentification (407 Proxy Authentication Required) : mets l'identifiant et le mot de passe dans l'adresse du proxy, dans Réglages › Avancé › Réseau";
    let en = "the proxy 127.0.0.1:9000 asks for authentication (407 Proxy Authentication Required): put the user name and password in the proxy address, in Settings › Advanced › Network";
    for what in [What::Check, What::Download] {
        // Whatever the proxy said of itself: it is not a refusal of the installer to send to the
        // browser.
        let answered = Failure {
            what,
            status: Some(407),
            ..refused()
        };
        assert_eq!(answered.describe(Fr), fr);
        assert_eq!(answered.describe(En), en);
        // The same when it said it to a tunnel, which tells no code to the app.
        let tunnel = Failure {
            what,
            status: None,
            answered: false,
            agent: None,
            page: None,
            stuck: Some(Stuck::TunnelAuth),
            ..refused()
        };
        assert_eq!(tunnel.describe(Fr), fr);
        assert_eq!(tunnel.describe(En), en);
    }
    // No proxy of the settings (the system's, or none): the address to fix is not known.
    let system = Failure {
        proxy: None,
        what: What::Check,
        status: Some(407),
        agent: None,
        page: None,
        ..refused()
    };
    assert_eq!(
        system.describe(En),
        "407 Proxy Authentication Required from release-assets.githubusercontent.com: blocked by the network or a proxy?"
    );
}

#[test]
fn a_tunnel_the_proxy_turned_down_is_told_as_the_proxys_refusal_and_not_gone_around() {
    let tunnel = |what| Failure {
        what,
        status: None,
        answered: false,
        host: "github.com".into(),
        agent: None,
        page: None,
        stuck: Some(Stuck::TunnelRefused),
        ..refused()
    };
    assert_eq!(
        tunnel(What::Check).describe(Fr),
        "le proxy 127.0.0.1:9000 a refusé d'ouvrir la connexion à github.com (tunnel CONNECT refusé) : une règle de ton entreprise le bloque peut-être."
    );
    assert_eq!(
        tunnel(What::Check).describe(En),
        "the proxy 127.0.0.1:9000 refused to open the connection to github.com (CONNECT tunnel refused): a rule of your company may be blocking it."
    );
    // The installer has the browser to go to.
    assert_eq!(
        tunnel(What::Download).describe(Fr),
        "le proxy 127.0.0.1:9000 a refusé d'ouvrir la connexion à github.com (tunnel CONNECT refusé) : une règle de ton entreprise le bloque peut-être. Télécharge l'installateur dans le navigateur."
    );
    assert_eq!(
        tunnel(What::Download).describe(En),
        "the proxy 127.0.0.1:9000 refused to open the connection to github.com (CONNECT tunnel refused): a rule of your company may be blocking it. Download the installer in the browser."
    );
}

#[test]
fn a_403_through_a_proxy_has_no_tail_about_a_try_without_it() {
    // The Zscaler of the user who met this: refused by the proxy, nothing tried around it.
    let f = refused();
    assert!(f.direct.is_none());
    assert!(!f.describe(Fr).contains("sans proxy"));
    assert!(!f.describe(En).contains("without the proxy"));
}

#[test]
fn a_download_refused_by_something_that_never_named_itself_is_a_plain_refusal() {
    // No name: the proxy is not blamed.
    let nameless = Failure {
        agent: None,
        page: None,
        ..refused()
    };
    assert_eq!(
        nameless.describe(En),
        "403 Forbidden from release-assets.githubusercontent.com, through the proxy 127.0.0.1:9000: blocked by the network or a proxy?"
    );
    // A missing file is not a proxy's refusal, whatever it called itself.
    let missing = Failure {
        status: Some(404),
        ..refused()
    };
    assert!(missing
        .describe(Fr)
        .starts_with("404 Not Found de release-assets.githubusercontent.com"));
    // Without a proxy in between, the server's own refusal.
    let direct = Failure {
        proxy: None,
        ..refused()
    };
    assert!(direct.describe(Fr).starts_with("403 Forbidden de "));
}

#[test]
fn a_host_that_cannot_be_reached_is_told_with_the_proxy_it_was_asked_through() {
    let f = unreachable_host();
    assert_eq!(
        f.describe(Fr),
        "connexion impossible à github.com par le proxy 127.0.0.1:9000 (tcp connect error: refused)"
    );
    assert_eq!(
        f.describe(En),
        "couldn’t reach github.com through the proxy 127.0.0.1:9000 (tcp connect error: refused)"
    );
    let direct = Failure { proxy: None, ..f };
    assert_eq!(
        direct.describe(Fr),
        "connexion impossible à github.com (tcp connect error: refused)"
    );
    assert_eq!(
        direct.describe(En),
        "couldn’t reach github.com (tcp connect error: refused)"
    );
}

#[test]
fn an_answer_that_cannot_be_read_is_told_with_what_the_plugin_made_of_it() {
    let f = Failure {
        what: What::Check,
        status: None,
        answered: true,
        proxy: None,
        agent: None,
        page: None,
        host: "github.com".into(),
        detail: "error decoding response body".into(),
        ..refused()
    };
    assert_eq!(
        f.describe(Fr),
        "réponse inattendue de github.com : error decoding response body"
    );
    assert_eq!(
        f.describe(En),
        "unexpected answer from github.com: error decoding response body"
    );
}

#[test]
fn a_try_without_the_proxy_that_failed_too_is_told_after_the_first() {
    let f = Failure {
        direct: Some(Box::new(Failure {
            proxy: None,
            ..unreachable_host()
        })),
        ..unreachable_host()
    };
    assert_eq!(
        f.describe(Fr),
        "connexion impossible à github.com par le proxy 127.0.0.1:9000 (tcp connect error: refused) ; sans proxy : connexion impossible à github.com (tcp connect error: refused)"
    );
    assert_eq!(
        f.describe(En),
        "couldn’t reach github.com through the proxy 127.0.0.1:9000 (tcp connect error: refused); without the proxy: couldn’t reach github.com (tcp connect error: refused)"
    );
    // The window shows it in the language of the interface (French in tests).
    assert_eq!(f.to_string(), f.describe(Fr));
}

#[test]
fn a_failure_goes_through_an_anyhow_error_and_comes_out_of_it() {
    let e = anyhow::Error::new(refused());
    assert_eq!(e.downcast_ref::<Failure>(), Some(&refused()));
    assert_eq!(e.to_string(), refused().describe(Fr));
}

// ---------- the page of the release ----------

#[test]
fn the_browser_is_sent_to_the_page_of_the_release_the_link_belongs_to() {
    assert_eq!(
        release_page(INSTALLER, &[]),
        "https://github.com/guillaume-gagnaire/escouade/releases/tag/v1.8.0"
    );
    assert_eq!(refused().page_url(), release_page(INSTALLER, &[]));
    // A query or a fragment is not the tag's.
    assert_eq!(
        release_page(
            "https://github.com/me/app/releases/download/v2.0.0-beta.1/app.dmg?x=1#top",
            &[]
        ),
        "https://github.com/me/app/releases/tag/v2.0.0-beta.1"
    );
    // Not a link to a file of a release: the latest one.
    for other in [
        "https://github.com/guillaume-gagnaire/escouade/releases/latest/download/latest.json",
        "https://github.com/me/app/releases/download/v1.0.0",
        "https://github.com/me/app/releases/download//app.exe",
        "",
    ] {
        assert_eq!(release_page(other, &[]), RELEASES_URL, "{other}");
    }
}

#[test]
fn a_page_is_opened_only_on_github_or_on_the_origin_of_the_endpoint() {
    let path = "/me/app/releases/download/v1.0.0/app.exe";
    let tag = "/me/app/releases/tag/v1.0.0";
    // GitHub, in https: the usual.
    assert_eq!(
        release_page(&format!("https://github.com{path}"), &[]),
        format!("https://github.com{tag}")
    );
    // A latest.json that was tampered with (a proxy that does not verify, a mirror) sends the
    // browser nowhere else: not to http, not to another domain or one that looks like GitHub, not
    // to another port or a login in front of it.
    for evil in [
        "http://github.com",
        "https://evil.example",
        "https://github.com.evil.example",
        "https://evil.example/github.com",
        "https://api.github.com",
        "https://github.com:8443",
        "https://github.com@evil.example",
        "https://githu8.com",
    ] {
        assert_eq!(
            release_page(&format!("{evil}{path}"), &[]),
            RELEASES_URL,
            "{evil}"
        );
    }
    // A link that is not one.
    assert_eq!(
        release_page("not a url/releases/download/v1.0.0/x", &[]),
        RELEASES_URL
    );
    // The endpoint of latest.json is trusted, with its scheme, its host and its port, and no other.
    let endpoints = [url("http://127.0.0.1:8080/latest.json")];
    assert_eq!(
        release_page(&format!("http://127.0.0.1:8080{path}"), &endpoints),
        format!("http://127.0.0.1:8080{tag}")
    );
    for other in [
        "http://127.0.0.1:9090",
        "https://127.0.0.1:8080",
        "http://localhost:8080",
        "http://127.0.0.1",
        "http://github.com",
    ] {
        assert_eq!(
            release_page(&format!("{other}{path}"), &endpoints),
            RELEASES_URL,
            "{other}"
        );
    }
    // GitHub stays trusted when the endpoint is elsewhere, and any one of the endpoints counts.
    let several = [
        url("http://127.0.0.1:8080/latest.json"),
        url("https://mirror.example/latest.json"),
    ];
    assert_eq!(
        release_page(&format!("https://github.com{path}"), &several),
        format!("https://github.com{tag}")
    );
    assert_eq!(
        release_page(&format!("https://mirror.example{path}"), &several),
        format!("https://mirror.example{tag}")
    );
    // A failure carries the endpoints it was made with.
    let f = Failure {
        url: format!("http://127.0.0.1:8080{path}"),
        trusted: endpoints.to_vec(),
        ..refused()
    };
    assert_eq!(f.page_url(), format!("http://127.0.0.1:8080{tag}"));
    let tampered = Failure {
        trusted: Vec::new(),
        ..f
    };
    assert_eq!(tampered.page_url(), RELEASES_URL);
}

// ---------- the text of a page ----------

#[test]
fn the_text_of_a_page_has_no_tags_its_blanks_merged_and_a_limited_length() {
    let html = "<html><head><title>Blocked</title><style>p { color: red }</style>\n<script>var a = '<b>';</script></head>\n<body><h1>Access  denied</h1>\n\n<p>Your company&#39;s policy &amp; the proxy&nbsp;say no.</p></body></html>";
    assert_eq!(
        page_text(html),
        "Blocked Access denied Your company's policy & the proxy say no."
    );
    assert_eq!(page_text("   \n\t "), "");
    assert_eq!(
        page_text("plain text\nover two lines"),
        "plain text over two lines"
    );
    // A link on the page is told without its query: it may be a signed one.
    assert_eq!(
        page_text("<p>Blocked https://cdn.example/a?token=SECRET&x=1 by policy</p>"),
        "Blocked https://cdn.example/a by policy"
    );
    // A page cut in the middle of a tag or a script says what came before.
    assert_eq!(page_text("Denied <a href=\"https://x"), "Denied");
    assert_eq!(page_text("Denied<script>never ends"), "Denied");
    // At most 160 characters, ellipsis included, whatever their width.
    let long = page_text(&"é".repeat(500));
    assert_eq!(long.chars().count(), 160);
    assert!(long.ends_with('…'));
    assert_eq!(page_text(&"a".repeat(160)).chars().count(), 160);
    assert!(!page_text(&"a".repeat(160)).ends_with('…'));
}

// ---------- who answered ----------

fn headers(pairs: &[(&'static str, &'static str)]) -> HeaderMap {
    pairs
        .iter()
        .map(|(k, v)| (k.parse().unwrap(), HeaderValue::from_static(v)))
        .collect()
}

#[test]
fn a_proxy_gives_its_name_in_its_headers_or_its_page_but_a_server_does_not() {
    // Its own header first.
    assert_eq!(
        agent_of(&headers(&[("proxy-agent", "Zscaler/6.2")]), None).as_deref(),
        Some("Zscaler/6.2")
    );
    // A `Server` that names a proxy.
    assert_eq!(
        agent_of(&headers(&[("server", "BlueCoat-ProxySG")]), None).as_deref(),
        Some("BlueCoat-ProxySG")
    );
    assert_eq!(
        agent_of(&headers(&[("server", "Squid/4.1")]), None).as_deref(),
        Some("Squid/4.1")
    );
    // GitHub, nginx and the like are not proxies.
    for server in ["GitHub.com", "nginx", "Microsoft-IIS/10.0", "AmazonS3"] {
        assert_eq!(agent_of(&headers(&[("server", server)]), None), None);
    }
    // A page that names one.
    assert_eq!(
        agent_of(
            &HeaderMap::new(),
            Some("Access denied by the Zscaler web security service"),
        )
        .as_deref(),
        Some("Zscaler")
    );
    assert_eq!(
        agent_of(&HeaderMap::new(), Some("Blocked by Palo Alto Networks")).as_deref(),
        Some("Palo Alto")
    );
    assert_eq!(
        agent_of(&HeaderMap::new(), Some("Not found on GitHub.com")),
        None
    );
    assert_eq!(agent_of(&HeaderMap::new(), None), None);
}

#[test]
fn a_via_never_names_a_proxy_not_even_one_that_calls_itself_squid() {
    // Every hop that relays an answer adds itself to `Via`, a refusal of GitHub's as well as a
    // proxy's own: it says who relayed, not who refused.
    for via in [
        "1.1 varnish",
        "1.1 proxy.corp",
        "1.1 proxy (squid)",
        "1.1 varnish, 1.1 proxy.corp (Squid/4.1)",
        "1.1 x.cloudfront.net (CloudFront)",
        "1.1 zscaler.corp (Zscaler/6.2)",
    ] {
        assert_eq!(agent_of(&headers(&[("via", via)]), None), None, "{via}");
    }
    // The server that refused still says who it is, and a header of the proxy still wins.
    assert_eq!(
        agent_of(
            &headers(&[("via", "1.1 proxy (squid)"), ("server", "GitHub.com")]),
            None
        ),
        None
    );
    assert_eq!(
        agent_of(
            &headers(&[("via", "1.1 proxy (squid)"), ("proxy-agent", "Zscaler/6.2")]),
            None
        )
        .as_deref(),
        Some("Zscaler/6.2")
    );
}

#[test]
fn the_name_of_a_proxy_is_cut_to_a_line_like_a_page() {
    let long = HeaderValue::from_str(&"Z".repeat(500)).unwrap();
    let mut map = HeaderMap::new();
    map.insert("proxy-agent", long);
    let agent = agent_of(&map, None).unwrap();
    assert_eq!(agent.chars().count(), 160);
    assert!(agent.ends_with('…'), "{agent}");
    assert!(agent.starts_with("ZZZZ"));
    // 160 characters are kept whole.
    map.insert(
        "proxy-agent",
        HeaderValue::from_str(&"Z".repeat(160)).unwrap(),
    );
    assert_eq!(agent_of(&map, None).unwrap(), "Z".repeat(160));
}

#[test]
fn every_proxy_the_app_knows_is_recognised_in_a_page_whatever_the_case() {
    for (said, named) in [
        ("zscaler", "Zscaler"),
        ("NETSKOPE", "Netskope"),
        ("Squid", "Squid"),
        ("bluecoat", "Blue Coat"),
        ("Blue Coat ProxySG", "Blue Coat"),
        ("Symantec", "Symantec"),
        ("FORCEPOINT", "Forcepoint"),
        ("websense", "Websense"),
        ("FortiGate", "FortiGate"),
        ("Fortinet", "Fortinet"),
        ("palo alto", "Palo Alto"),
        ("Cisco Umbrella", "Umbrella"),
        ("Sophos", "Sophos"),
        ("Skyhigh", "Skyhigh"),
        ("McAfee", "McAfee"),
        ("Check Point", "Check Point"),
        ("Barracuda", "Barracuda"),
        ("iboss", "iboss"),
    ] {
        assert_eq!(
            agent_of(&HeaderMap::new(), Some(&format!("Blocked by {said}"))).as_deref(),
            Some(named),
            "{said}"
        );
    }
}

// ---------- the probe ----------

const BLOCK_PAGE: &str = "<html><head><title>Zscaler</title></head><body><h1>Access denied</h1><p>This site is blocked by your company.</p></body></html>";

async fn probe_of(server: &Server, path: &str, net: &Network, route: Route, what: What) -> Failure {
    probe(
        &url(&server.url(path)),
        net,
        route,
        "application/json",
        what,
        "the plugin said no".into(),
    )
    .await
}

#[tokio::test]
async fn the_probe_reads_the_code_the_page_and_the_name_of_what_refused() {
    let server = Server::start().await;
    server.script(
        "/latest.json",
        vec![Reply::html(403, BLOCK_PAGE).with("Server", "Zscaler/6.2")],
    );
    let f = probe_of(
        &server,
        "/latest.json",
        &net(None),
        Route::Direct,
        What::Check,
    )
    .await;
    assert_eq!(f.what, What::Check);
    assert_eq!(f.url, server.url("/latest.json"));
    assert_eq!(f.status, Some(403));
    assert!(f.answered);
    assert_eq!(f.host, "127.0.0.1");
    assert_eq!(f.proxy, None);
    assert_eq!(f.agent.as_deref(), Some("Zscaler/6.2"));
    assert_eq!(
        f.page.as_deref(),
        Some("Zscaler Access denied This site is blocked by your company.")
    );
    // The plugin's own words are kept for what the probe cannot tell.
    assert_eq!(f.detail, "the plugin said no");
    assert_eq!(f.direct, None);
    assert!(f.transient());
}

#[tokio::test]
async fn the_probe_asks_as_the_plugin_did_with_the_same_name_and_accept() {
    let server = Server::start().await;
    server.script("/a", vec![Reply::status(404)]);
    probe(
        &url(&server.url("/a")),
        &net(None),
        Route::Direct,
        "application/octet-stream",
        What::Download,
        String::new(),
    )
    .await;
    let seen = server.hits("/a");
    assert_eq!(seen.len(), 1);
    assert_eq!(seen[0].headers["user-agent"], user_agent());
    assert_eq!(seen[0].headers["accept"], "application/octet-stream");
}

#[tokio::test]
async fn the_probe_goes_through_the_proxy_asked_for_and_not_when_told_to_go_direct() {
    let origin = Server::start().await;
    origin.script("/latest.json", vec![Reply::json(200, "{}")]);
    let proxy = Server::start().await;
    proxy.script(
        "/latest.json",
        vec![Reply::html(403, BLOCK_PAGE).with("Proxy-Agent", "Zscaler/6.2")],
    );
    let network = net(Some(&proxy));
    let through = probe_of(&origin, "/latest.json", &network, Route::Proxy, What::Check).await;
    assert_eq!(through.status, Some(403));
    assert_eq!(through.proxy, Some(proxy.address()));
    assert_eq!(through.agent.as_deref(), Some("Zscaler/6.2"));
    // The proxy was asked, the origin was not.
    assert_eq!(proxy.hits("/latest.json").len(), 1);
    assert!(origin.seen().is_empty());
    // Direct: the origin answers and the success is no failure to explain, only unreadable.
    let direct = probe_of(
        &origin,
        "/latest.json",
        &network,
        Route::Direct,
        What::Check,
    )
    .await;
    assert_eq!(direct.status, None);
    assert!(direct.answered);
    assert_eq!(direct.proxy, None);
    assert_eq!(origin.hits("/latest.json").len(), 1);
    assert_eq!(proxy.hits("/latest.json").len(), 1);
}

#[tokio::test]
async fn a_success_is_unreadable_not_refused_and_its_body_is_not_read() {
    let server = Server::start().await;
    server.script(
        "/latest.json",
        vec![Reply::html(200, "<html>Sign in to continue</html>")],
    );
    let f = probe_of(
        &server,
        "/latest.json",
        &net(None),
        Route::Direct,
        What::Check,
    )
    .await;
    assert_eq!(f.status, None);
    assert!(f.answered);
    assert_eq!(f.page, None);
    assert_eq!(f.detail, "the plugin said no");
    assert_eq!(f.host, "127.0.0.1");
    assert!(f.transient());
    assert!(!f.refused_by_proxy());
}

#[tokio::test]
async fn a_host_that_cannot_be_reached_has_no_code_and_a_complete_reason() {
    // A server that hangs up: a closed port would make Windows wait a second or two to refuse.
    let port = hangs_up().await;
    let target = url(&format!("http://127.0.0.1:{port}/latest.json"));
    let f = probe(
        &target,
        &net(None),
        Route::Direct,
        "application/json",
        What::Check,
        "the plugin said no".into(),
    )
    .await;
    assert_eq!(f.status, None);
    assert!(!f.answered);
    assert_eq!(f.host, "127.0.0.1");
    assert_eq!(f.proxy, None);
    // No proxy in the way: nothing to blame.
    assert_eq!(f.stuck, None);
    assert!(f.transient());
    // The network's reason, from the outermost to the cause, and not the plugin's words, nor the
    // address a second time.
    assert!(f.detail.contains("error sending request"), "{}", f.detail);
    assert!(f.detail.contains(": "), "{}", f.detail);
    assert!(!f.detail.contains(&port.to_string()), "{}", f.detail);
    assert!(!f.detail.contains("the plugin said no"));
    assert!(
        f.describe(En).starts_with("couldn’t reach 127.0.0.1 ("),
        "{f}"
    );
}

#[tokio::test]
async fn a_proxy_that_cannot_be_reached_is_the_one_blamed_and_is_gone_around_not_its_refusals() {
    let port = closed_port();
    let dead = Network {
        proxy: Some(url(&format!("http://127.0.0.1:{port}"))),
        ..Network::default()
    };
    let origin = Server::start().await;
    let f = probe_of(&origin, "/latest.json", &dead, Route::Proxy, What::Check).await;
    assert_eq!(f.status, None);
    assert!(!f.answered);
    assert_eq!(f.proxy, Some(format!("127.0.0.1:{port}")));
    assert_eq!(f.stuck, Some(Stuck::ProxyDown));
    assert!(f.proxy_unreachable());
    assert!(!f.refused_by_proxy());
    assert!(origin.seen().is_empty());
}

#[tokio::test]
async fn a_tunnel_the_proxy_turns_down_or_leaves_unanswered_is_told_apart() {
    // The proxy is asked for a tunnel to an https server (`CONNECT host:port`), which is never
    // reached.
    let target = Server::start().await;
    let tunnel = format!("127.0.0.1:{}", target.port);
    let https = url(&format!("https://{tunnel}/latest.json"));
    let proxy = Server::start().await;
    let ask = || async {
        probe(
            &https,
            &net(Some(&proxy)),
            Route::Proxy,
            "application/json",
            What::Check,
            "the plugin said no".into(),
        )
        .await
    };
    // A refusal, whatever its code: the proxy's rule, kept.
    for code in [403, 502] {
        proxy.script(&tunnel, vec![Reply::status(code)]);
        let f = ask().await;
        assert_eq!((f.status, f.answered), (None, false), "{code}");
        assert_eq!(f.stuck, Some(Stuck::TunnelRefused), "{code}");
        assert!(f.refused_by_proxy() && !f.proxy_unreachable(), "{code}");
        assert_eq!(f.host, "127.0.0.1");
    }
    // It wants credentials.
    proxy.script(&tunnel, vec![Reply::status(407)]);
    let f = ask().await;
    assert_eq!((f.status, f.answered), (None, false));
    assert_eq!(f.stuck, Some(Stuck::TunnelAuth));
    assert!(f.refused_by_proxy() && !f.proxy_unreachable());
    // No answer to the tunnel: the proxy is not working, and nothing was refused.
    proxy.script(&tunnel, vec![Reply::hang_up()]);
    let f = ask().await;
    assert_eq!(f.stuck, Some(Stuck::ProxyDown));
    assert!(f.proxy_unreachable() && !f.refused_by_proxy());
    // Four tunnels asked, nothing sent on.
    assert_eq!(proxy.hits(&tunnel).len(), 4);
    assert!(target.seen().is_empty());
}

#[tokio::test]
async fn a_proxy_that_hangs_up_on_the_request_is_not_reachable_either() {
    let port = hangs_up().await;
    let silent = Network {
        proxy: Some(url(&format!("http://127.0.0.1:{port}"))),
        ..Network::default()
    };
    // A tunnel: the proxy closes the connection before it says anything.
    let f = probe(
        &url("https://127.0.0.1:9/latest.json"),
        &silent,
        Route::Proxy,
        "application/json",
        What::Check,
        String::new(),
    )
    .await;
    assert_eq!((f.status, f.answered), (None, false));
    assert_eq!(f.stuck, Some(Stuck::ProxyDown));
    // The words of the address asked for are not the proxy's: reqwest repeats it in its error.
    let f = probe(
        &url("https://127.0.0.1:9/tunnel/unsuccessful/proxy-authorization-required"),
        &silent,
        Route::Proxy,
        "application/json",
        What::Check,
        String::new(),
    )
    .await;
    assert_eq!(f.stuck, Some(Stuck::ProxyDown));
}

#[tokio::test]
async fn nothing_is_the_proxys_fault_when_the_request_did_not_go_through_one() {
    let port = hangs_up().await;
    let f = probe(
        &url(&format!("http://127.0.0.1:{port}/latest.json")),
        &net(None),
        Route::Direct,
        "application/json",
        What::Check,
        String::new(),
    )
    .await;
    assert!(!f.answered);
    assert_eq!(f.stuck, None);
    assert!(!f.proxy_unreachable() && !f.refused_by_proxy());
}

#[tokio::test]
async fn a_403_the_server_gave_and_a_proxy_relayed_is_not_blamed_on_the_proxy() {
    let origin = Server::start().await;
    let installer = "/me/app/releases/download/v1.0.0/app.exe";
    let proxy = Server::start().await;
    // Squid adds itself to the `Via` of everything it relays, GitHub's refusals too.
    proxy.script(
        installer,
        vec![Reply::html(403, "<p>Forbidden</p>")
            .with("Via", "1.1 proxy (squid)")
            .with("Server", "GitHub.com")],
    );
    let f = probe_of(
        &origin,
        installer,
        &net(Some(&proxy)),
        Route::Proxy,
        What::Download,
    )
    .await;
    assert_eq!(f.status, Some(403));
    assert_eq!(f.agent, None);
    let said = f.describe(Fr);
    assert!(
        said.starts_with("403 Forbidden de 127.0.0.1, par le proxy "),
        "{said}"
    );
    assert!(!said.contains("ton entreprise"), "{said}");
    assert!(!said.contains("Squid") && !said.contains("squid"), "{said}");
    assert!(
        said.ends_with(" : bloqué par le réseau ou un proxy ?"),
        "{said}"
    );
}

// ---------- the classes of a failed connection ----------

/// An error with a cause, like the layers of reqwest's errors: the first text is the outermost.
#[derive(Debug)]
struct Layer(
    &'static str,
    Option<Box<dyn std::error::Error + Send + Sync>>,
);

impl std::fmt::Display for Layer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.0)
    }
}

impl std::error::Error for Layer {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.1
            .as_deref()
            .map(|e| e as &(dyn std::error::Error + 'static))
    }
}

/// `texts` as layers, the innermost holding an I/O error of `kind` when there is one.
fn layers(texts: &[&'static str], kind: Option<std::io::ErrorKind>) -> Layer {
    let (first, rest) = texts.split_first().unwrap();
    let cause: Option<Box<dyn std::error::Error + Send + Sync>> = if rest.is_empty() {
        kind.map(|k| Box::new(std::io::Error::new(k, "os error")) as _)
    } else {
        Some(Box::new(layers(rest, kind)))
    };
    Layer(first, cause)
}

#[test]
fn the_causes_of_a_connection_that_failed_tell_a_proxy_that_is_not_there_from_one_that_refuses() {
    use std::io::ErrorKind::*;
    let sending = "error sending request";
    let connect = "client error (Connect)";
    let stuck = |texts: &[&'static str], kind, timed_out| stuck_of(&layers(texts, kind), timed_out);
    // Refused, timed out, nowhere to go: the proxy is not there.
    for kind in [
        ConnectionRefused,
        TimedOut,
        NetworkUnreachable,
        HostUnreachable,
    ] {
        assert_eq!(
            stuck(&[sending, connect, "tcp connect error"], Some(kind), false),
            Some(Stuck::ProxyDown),
            "{kind:?}"
        );
    }
    // The request that took too long, whatever the layer that says so.
    assert_eq!(
        stuck(&[sending, "operation timed out"], None, true),
        Some(Stuck::ProxyDown)
    );
    // The name of the proxy is not found.
    assert_eq!(
        stuck(&[sending, connect, "dns error"], Some(Other), false),
        Some(Stuck::ProxyDown)
    );
    // The tunnel: no connection to the proxy, no word from it, a connection broken meanwhile.
    for texts in [
        &[
            sending,
            connect,
            "tunnel error: failed to create underlying connection",
            "tcp connect error",
        ][..],
        &[sending, connect, "tunnel error: unexpected end of file"],
        &[
            sending,
            connect,
            "tunnel error: io error establishing tunnel",
        ],
    ] {
        assert_eq!(
            stuck(texts, Some(ConnectionReset), false),
            Some(Stuck::ProxyDown),
            "{texts:?}"
        );
    }
    // The tunnel turned down, or asking for a password: its answer, which is its rule.
    assert_eq!(
        stuck(
            &[sending, connect, "tunnel error: unsuccessful"],
            None,
            false
        ),
        Some(Stuck::TunnelRefused)
    );
    assert_eq!(
        stuck(
            &[
                sending,
                connect,
                "tunnel error: proxy authorization required"
            ],
            None,
            false
        ),
        Some(Stuck::TunnelAuth)
    );
    // The proxy is there and let the request through: a certificate refused, a connection reset
    // after the answer began, headers too long, are none of that.
    for texts in [
        &[sending, connect, "invalid peer certificate: UnknownIssuer"][..],
        &[
            sending,
            "client error (SendRequest)",
            "connection closed before message completed",
        ],
        &[
            sending,
            connect,
            "tunnel error: proxy response headers too long",
        ],
        &[sending],
    ] {
        assert_eq!(stuck(texts, None, false), None, "{texts:?}");
    }
    assert_eq!(
        stuck(&[sending, "connection reset"], Some(ConnectionReset), false),
        None
    );
    assert_eq!(
        stuck(&[sending, "certificate"], Some(InvalidData), false),
        None
    );
}

// ---------- no signed link ----------

#[test]
fn the_query_of_a_link_is_left_out_of_a_text() {
    let signed = "https://release-assets.githubusercontent.com/github-production-release-asset/1/2?sig=SECRET&se=2026-10-15T10%3A00%3A00Z&sp=r";
    let bare = "https://release-assets.githubusercontent.com/github-production-release-asset/1/2";
    // The way reqwest says it, and the plugin after it.
    assert_eq!(
        without_queries(&format!(
            "error sending request for url ({signed}): timed out"
        )),
        format!("error sending request for url ({bare}): timed out")
    );
    assert_eq!(
        without_queries(&format!("error following redirect for url ({signed})")),
        format!("error following redirect for url ({bare})")
    );
    // Whatever surrounds it, every link of the text, the scheme in any case, the fragment too.
    assert_eq!(
        without_queries(&format!(
            "« {signed} » puis HTTP://Example.org/a?b=SECRET#top \"{signed}\""
        )),
        format!("« {bare} » puis HTTP://Example.org/a \"{bare}\"")
    );
    assert_eq!(
        without_queries("a http://x.example/p?q=1,2&sig=SECRET b"),
        "a http://x.example/p b"
    );
    // Text with no link, and question marks that are not a link's, are left alone.
    for same in [
        "",
        "Pourquoi ? parce que : « é » ? ok",
        "https://github.com/me/app/releases/tag/v1.0.0",
        "http:// ?not=a link",
        "ftp://x.example/a?b=c",
    ] {
        assert_eq!(without_queries(same), same);
    }
}

#[test]
fn an_error_of_the_network_holds_no_signed_link_in_its_chain_nor_in_what_is_told_of_it() {
    let signed = "https://release-assets.githubusercontent.com/x?sig=SECRET&se=1";
    let error = Layer(
        "error sending request for url (https://release-assets.githubusercontent.com/x?sig=SECRET&se=1)",
        Some(Box::new(Layer("connection closed", None))),
    );
    let said = chain(&error);
    assert_eq!(
        said,
        "error sending request for url (https://release-assets.githubusercontent.com/x): connection closed"
    );
    // The same in the causes, not only in the first line.
    let deep = Layer(
        "outer",
        Some(Box::new(Layer(
            "fetching https://release-assets.githubusercontent.com/x?sig=SECRET&se=1 failed",
            None,
        ))),
    );
    assert!(!chain(&deep).contains("SECRET") && !chain(&deep).contains("?sig="));
    // What is told is clean whoever made the failure: the chain's, or a detail written as it came.
    for detail in [said, format!("error sending request for url ({signed})")] {
        let f = Failure {
            what: What::Download,
            status: None,
            answered: true,
            proxy: None,
            agent: None,
            page: None,
            host: "release-assets.githubusercontent.com".into(),
            detail,
            ..refused()
        };
        for lang in [Fr, En] {
            let told = f.describe(lang);
            assert!(
                !told.contains("SECRET") && !told.contains("?sig="),
                "{told}"
            );
            assert!(
                told.contains("https://release-assets.githubusercontent.com/x"),
                "{told}"
            );
        }
    }
}

#[tokio::test]
async fn a_redirection_to_a_signed_link_that_fails_leaves_the_signature_out_of_the_failure() {
    let port = hangs_up().await;
    let releases = Server::start().await;
    releases.script(
        "/setup.exe",
        vec![Reply::redirect(&format!(
            "http://127.0.0.1:{port}/asset?sig=SECRET&se=1"
        ))],
    );
    let f = probe_of(
        &releases,
        "/setup.exe",
        &net(None),
        Route::Direct,
        What::Download,
    )
    .await;
    assert!(!f.answered);
    // The hop that could not be reached is told, its signature is not.
    assert_eq!(f.host, "127.0.0.1");
    assert!(
        !f.detail.contains("SECRET") && !f.detail.contains("sig="),
        "{}",
        f.detail
    );
    for lang in [Fr, En] {
        let told = f.describe(lang);
        assert!(!told.contains("SECRET") && !told.contains("sig="), "{told}");
    }
    assert!(!format!("{f:?}").contains("SECRET"), "{f:?}");
}

#[tokio::test]
async fn the_domain_told_is_the_one_that_answered_after_the_redirections() {
    let assets = Server::start().await;
    assets.script("/assets/setup.exe", vec![Reply::html(403, BLOCK_PAGE)]);
    let releases = Server::start().await;
    // Another name for the same machine: the second server is « release-assets ».
    releases.script(
        "/setup.exe",
        vec![Reply::redirect(&format!(
            "http://localhost:{}/assets/setup.exe",
            assets.port
        ))],
    );
    let f = probe_of(
        &releases,
        "/setup.exe",
        &net(None),
        Route::Direct,
        What::Download,
    )
    .await;
    assert_eq!(f.status, Some(403));
    assert_eq!(f.host, "localhost");
    // What was asked stays what was asked.
    assert_eq!(f.url, releases.url("/setup.exe"));
    assert_eq!(assets.hits("/assets/setup.exe").len(), 1);
}

#[tokio::test]
async fn a_page_is_read_for_what_is_text_only_and_for_a_few_kilobytes_at_most() {
    let server = Server::start().await;
    server.script("/json", vec![Reply::json(403, r#"{"message":"Zscaler"}"#)]);
    server.script("/binary", vec![Reply::bytes(403, b"Zscaler says no")]);
    server.script("/plain", vec![Reply::text(403, "Denied by policy")]);
    server.script("/empty", vec![Reply::text(403, "")]);
    server.script(
        "/long",
        vec![Reply::text(403, &format!("{}END", "word ".repeat(10_000)))],
    );
    // Eight kilobytes of blanks, then the name: past what is read.
    server.script(
        "/late",
        vec![Reply::html(
            403,
            &format!("{}Blocked by Zscaler", " ".repeat(8 * 1024)),
        )],
    );
    let on = |path: &'static str| {
        let server = server.clone();
        async move { probe_of(&server, path, &net(None), Route::Direct, What::Check).await }
    };
    // JSON and files say nothing of a proxy: not read.
    let json = on("/json").await;
    assert_eq!(
        (json.status, json.page, json.agent),
        (Some(403), None, None)
    );
    let binary = on("/binary").await;
    assert_eq!(
        (binary.status, binary.page, binary.agent),
        (Some(403), None, None)
    );
    let plain = on("/plain").await;
    assert_eq!(plain.page.as_deref(), Some("Denied by policy"));
    assert_eq!(on("/empty").await.page, None);
    // Cut to a line.
    let long = on("/long").await.page.unwrap();
    assert_eq!(long.chars().count(), 160);
    assert!(long.ends_with('…') && !long.contains("END"), "{long}");
    let late = on("/late").await;
    assert_eq!(
        (late.status, late.page, late.agent),
        (Some(403), None, None)
    );
}

#[tokio::test]
async fn the_probe_trusts_a_self_signed_certificate_only_with_tls_verification_off() {
    let server = crate::integrations::fake::FakeServer::start_tls().await;
    server.on("GET", "/latest.json", 403, serde_json::json!({}));
    let target = url(&format!("{}/latest.json", server.url));
    let ask = |insecure_tls: bool| {
        let (target, network) = (
            target.clone(),
            Network {
                insecure_tls,
                ..Network::default()
            },
        );
        async move {
            probe(
                &target,
                &network,
                Route::Direct,
                "application/json",
                What::Check,
                String::new(),
            )
            .await
        }
    };
    // Verified: the certificate is refused, there is no answer.
    let verified = ask(false).await;
    assert_eq!((verified.status, verified.answered), (None, false));
    assert_eq!(verified.host, "127.0.0.1");
    // Not verified: the server answers.
    let trusting = ask(true).await;
    assert_eq!((trusting.status, trusting.answered), (Some(403), true));
}
