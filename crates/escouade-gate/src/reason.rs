//! Why the gate refused a connection, said so that the agent can read it (§ 4.4): the `403` of a `CONNECT`, the line
//! `escouade-server via` writes on the standard error for `ssh`, the journal.

use chrono::{Local, TimeZone};

/// The language the refusals are written in: the server's.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Fr,
    En,
}

/// Why a connection was refused or failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    /// No token in the proxy's address.
    NoToken,
    /// A token the gate did not give, or took back.
    UnknownToken,
    /// A host of a private network, out of the project's list.
    NotInList { target: String, project: String },
    /// In the project's list, out of its connector's ceiling.
    NotInCeiling { target: String, connector: String },
    /// Refused by the connector itself.
    RefusedByConnector {
        target: String,
        connector: String,
        why: String,
    },
    /// Its connector's channel is cut.
    Down { connector: String, since_ms: u64 },
    /// Its connector is not paired (any more).
    UnknownConnector { target: String, connector: String },
    /// The host could not be reached.
    Unreachable { target: String, why: String },
    /// The space's proxy refused it.
    UpstreamRefused { target: String, status: u16 },
    /// What came is no request of a proxy.
    Malformed(String),
}

/// « d'Acme », « de Globex ».
fn of_fr(name: &str) -> String {
    let vowel = name
        .chars()
        .next()
        .is_some_and(|c| "aeiouyàâäéèêëîïôöùûüAEIOUYÀÂÄÉÈÊËÎÏÔÖÙÛÜ".contains(c));
    if vowel {
        format!("d’{name}")
    } else {
        format!("de {name}")
    }
}

/// « 14:32 », or the day before it when it is not today's.
fn since(ms: u64, lang: Lang) -> String {
    let Some(at) = Local.timestamp_millis_opt(ms as i64).single() else {
        return "?".into();
    };
    let today = Local::now().date_naive() == at.date_naive();
    match (lang, today) {
        (_, true) => at.format("%H:%M").to_string(),
        (Lang::Fr, false) => at.format("le %d/%m à %H:%M").to_string(),
        (Lang::En, false) => at.format("%Y-%m-%d %H:%M").to_string(),
    }
}

impl Reason {
    /// In a few letters, for the journal.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::NoToken => "no-token",
            Self::UnknownToken => "unknown-token",
            Self::NotInList { .. } => "not-in-list",
            Self::NotInCeiling { .. } => "not-in-ceiling",
            Self::RefusedByConnector { .. } => "refused-by-connector",
            Self::Down { .. } => "connector-down",
            Self::UnknownConnector { .. } => "unknown-connector",
            Self::Unreachable { .. } => "unreachable",
            Self::UpstreamRefused { .. } => "upstream-refused",
            Self::Malformed(_) => "malformed",
        }
    }

    /// The status of the HTTP answer.
    pub fn status(&self) -> u16 {
        match self {
            Self::NoToken | Self::UnknownToken => 407,
            Self::Unreachable { .. } | Self::UpstreamRefused { .. } => 502,
            Self::Malformed(_) => 400,
            _ => 403,
        }
    }

    /// Whether the gate refused it (and not the network that failed).
    pub fn refused(&self) -> bool {
        self.status() != 502
    }

    /// The sentence the agent reads.
    pub fn say(&self, lang: Lang) -> String {
        match (self, lang) {
            (Self::NoToken, Lang::Fr) => "la porte d’Escouade veut le jeton de l’agent dans l’adresse du proxy (http://<jeton>@127.0.0.1:<port>)".into(),
            (Self::NoToken, Lang::En) => "Escouade's gate wants the agent's token in the proxy's address (http://<token>@127.0.0.1:<port>)".into(),
            (Self::UnknownToken, Lang::Fr) => "la porte d’Escouade ne connaît pas ce jeton (un agent arrêté, ou un serveur redémarré)".into(),
            (Self::UnknownToken, Lang::En) => "Escouade's gate does not know this token (a stopped agent, or a restarted server)".into(),
            (Self::NotInList { target, project }, Lang::Fr) => format!("{target} n’est pas dans la liste du projet {project}"),
            (Self::NotInList { target, project }, Lang::En) => format!("{target} is not in the list of the project {project}"),
            (Self::NotInCeiling { target, connector }, Lang::Fr) => format!("{target} n’est pas permis par le connecteur {}", of_fr(connector)),
            (Self::NotInCeiling { target, connector }, Lang::En) => format!("{target} is not allowed by the connector of {connector}"),
            (Self::RefusedByConnector { target, connector, why }, Lang::Fr) => format!("{target} n’est pas permis par le connecteur {} ({why})", of_fr(connector)),
            (Self::RefusedByConnector { target, connector, why }, Lang::En) => format!("{target} is not allowed by the connector of {connector} ({why})"),
            (Self::Down { connector, since_ms }, Lang::Fr) => format!("le réseau privé {} est injoignable depuis {}", of_fr(connector), since(*since_ms, lang)),
            (Self::Down { connector, since_ms }, Lang::En) => format!("the private network of {connector} is unreachable since {}", since(*since_ms, lang)),
            (Self::UnknownConnector { target, connector }, Lang::Fr) => format!("{target} passe par le connecteur {connector}, qui n’est pas appairé"),
            (Self::UnknownConnector { target, connector }, Lang::En) => format!("{target} goes through the connector {connector}, which is not paired"),
            (Self::Unreachable { target, why }, Lang::Fr) => format!("{target} est injoignable : {why}"),
            (Self::Unreachable { target, why }, Lang::En) => format!("{target} can't be reached: {why}"),
            (Self::UpstreamRefused { target, status }, Lang::Fr) => format!("le proxy de l’espace refuse {target} ({status})"),
            (Self::UpstreamRefused { target, status }, Lang::En) => format!("the space's proxy refuses {target} ({status})"),
            (Self::Malformed(why), Lang::Fr) => format!("requête de proxy illisible : {why}"),
            (Self::Malformed(why), Lang::En) => format!("unreadable proxy request: {why}"),
        }
    }
}

/// `text` in ASCII, for the reason phrase of an HTTP status line: the accents of French taken off, the rest as `?`.
pub fn ascii(text: &str) -> String {
    text.chars()
        .map(|c| match c {
            'à' | 'â' | 'ä' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' => 'i',
            'ô' | 'ö' => 'o',
            'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            'À' | 'Â' | 'Ä' => 'A',
            'É' | 'È' | 'Ê' | 'Ë' => 'E',
            'Î' | 'Ï' => 'I',
            'Ô' | 'Ö' => 'O',
            'Ù' | 'Û' | 'Ü' => 'U',
            'Ç' => 'C',
            '’' => '\'',
            c if c.is_ascii() && !c.is_ascii_control() => c,
            _ => '?',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_refusals_read_as_the_study_says() {
        let r = Reason::NotInList {
            target: "gitlab.acme.internal:22".into(),
            project: "Démo".into(),
        };
        assert_eq!(
            r.say(Lang::Fr),
            "gitlab.acme.internal:22 n’est pas dans la liste du projet Démo"
        );
        let c = Reason::NotInCeiling {
            target: "x:1".into(),
            connector: "Acme".into(),
        };
        assert_eq!(
            c.say(Lang::Fr),
            "x:1 n’est pas permis par le connecteur d’Acme"
        );
        let g = Reason::NotInCeiling {
            target: "x:1".into(),
            connector: "Globex".into(),
        };
        assert!(g.say(Lang::Fr).ends_with("le connecteur de Globex"));
        let now = chrono::Local::now();
        let d = Reason::Down {
            connector: "Acme".into(),
            since_ms: now.timestamp_millis() as u64,
        };
        assert_eq!(
            d.say(Lang::Fr),
            format!(
                "le réseau privé d’Acme est injoignable depuis {}",
                now.format("%H:%M")
            )
        );
        assert_eq!(r.status(), 403);
        assert_eq!(Reason::NoToken.status(), 407);
        assert_eq!(ascii("réseau privé d’Acme"), "reseau prive d'Acme");
    }
}
