//! The lines the client writes in the conversation, in the app's language (`fr`, else English). `{name}` is the
//! harness's, as its manifest names it.

pub(crate) fn tr(lang: &str, fr: String, en: String) -> String {
    if lang == "fr" {
        fr
    } else {
        en
    }
}

pub(crate) fn protocol_refused(lang: &str, name: &str, version: &str) -> String {
    let ours = crate::PROTOCOL;
    tr(
        lang,
        format!("{name} parle ACP {version}, Escouade ACP {ours} : mets {name} à jour."),
        format!("{name} speaks ACP {version}, Escouade ACP {ours}: update {name}."),
    )
}

pub(crate) fn session_refused(lang: &str, name: &str, e: &str) -> String {
    tr(
        lang,
        format!("{name} n’a pas ouvert la session de l’agent : {e}. S’il demande une connexion, connecte-le dans un terminal."),
        format!("{name} didn’t open the agent’s session: {e}. If it asks for a sign-in, sign it in in a terminal."),
    )
}

pub(crate) fn not_resumed(lang: &str, name: &str) -> String {
    tr(
        lang,
        format!("La session {name} n’a pas pu être reprise : nouvelle session (contexte vidé)"),
        format!("The {name} session couldn’t be resumed: new session (context cleared)"),
    )
}

pub(crate) fn fence_missing(lang: &str, name: &str) -> String {
    tr(
        lang,
        format!("{name} lit et écrit les fichiers lui-même, sans les faire passer par Escouade (son manifeste le dit) : la clôture des projets privés ne tiendrait pas, l’agent ne démarre pas dans ce projet privé."),
        format!("{name} reads and writes files by itself, without passing them through Escouade (its manifest says so): the fence of private projects wouldn’t hold, the agent doesn’t start in this private project."),
    )
}

pub(crate) fn read_only_missing(lang: &str, name: &str) -> String {
    tr(
        lang,
        format!("{name} n’a pas de mode qui ne change rien (son manifeste n’en dit aucun) : il ne tient pas une discussion en lecture seule."),
        format!("{name} has no mode that changes nothing (its manifest names none): it doesn’t hold a read-only discussion."),
    )
}

pub(crate) fn fence_leak(lang: &str, name: &str, path: &str) -> String {
    tr(
        lang,
        format!("{name} a lu ou modifié {path} sans le faire passer par Escouade : la clôture des projets privés ne tient pas pour ces accès, que seules ses permissions protègent."),
        format!("{name} read or changed {path} without passing it through Escouade: the fence of private projects doesn’t hold for these, which only its permissions protect."),
    )
}

pub(crate) fn outside(lang: &str) -> String {
    tr(
        lang,
        "Hors du dossier de l’agent".into(),
        "Outside the agent’s folder".into(),
    )
}

pub(crate) fn mcp_tool(lang: &str, server: &str, tool: &str) -> String {
    tr(
        lang,
        format!("Serveur MCP {server}, outil {tool}"),
        format!("MCP server {server}, tool {tool}"),
    )
}

pub(crate) fn not_translated(lang: &str, name: &str, what: &str) -> String {
    tr(lang, format!("{name} : {what}"), format!("{name}: {what}"))
}

/// What the agent is told of a file refused (in English: the model reads it).
pub(crate) fn fenced_file(path: &str) -> String {
    format!("Refused by Escouade: {path} is in a private project, out of this agent's reach.")
}

pub(crate) fn not_absolute(path: &str) -> String {
    format!("Refused by Escouade: {path} is not an absolute path.")
}

pub(crate) fn out_of_reach(path: &str) -> String {
    format!("Refused by Escouade: {path} is out of this agent's folders.")
}

pub(crate) fn read_only_file(path: &str) -> String {
    format!("Refused by Escouade: this agent is read-only, {path} is not written.")
}
