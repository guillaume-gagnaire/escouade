//! « Les agents peuvent piloter Escouade » for a harness whose CLI cannot declare a server with
//! its header (Codex: `codex mcp add` takes no header, and `bearer_token` is refused for an HTTP
//! entry; OpenCode and Copilot CLI: no command, or none that writes the header as wanted):
//! Escouade writes the entry in the user config itself, and keeps all the rest of the file as it
//! was, comments and layout included (docs/ENGINES.md, « Le serveur MCP d'Escouade »).
//!
//! A harness only declares where and how (`UserConfig`): the file in the account's folder, its
//! format, the table of the servers and the names of an entry's fields. Reading, adding, removing
//! and the text to paste by hand are the same for all.

use crate::jsonc;
use crate::mcp::SERVER;
use crate::mcp::{Entry, Standing};
use anyhow::{anyhow, bail, Context, Result};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, InlineTable, Item, Table};

/// The format of a user config.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// TOML (Codex's `config.toml`), edited by `toml_edit`.
    Toml,
    /// JSON with comments and trailing commas (OpenCode's `opencode.json`, Copilot CLI's
    /// `mcp-config.json`), edited in its text (`jsonc`).
    Jsonc,
}

/// Where and how a harness's user config holds an MCP server reached over HTTP with a header.
#[derive(Debug)]
pub struct UserConfig {
    /// The file, in the account's configuration folder (`accounts::config_dir`): `config.toml`.
    /// Several: the first that is there (`file_in`), else the first.
    pub files: &'static [&'static str],
    pub format: Format,
    /// The table of the servers, from the top of the file: `["mcp_servers"]`.
    pub servers: &'static [&'static str],
    /// The field of the server's address: `url`.
    pub url: &'static str,
    /// The table of the headers sent at each call: `http_headers`.
    pub headers: &'static str,
    /// The fields the entry starts with, each with its text: `"type": "remote"`.
    pub fixed: &'static [(&'static str, &'static str)],
}

/// The header that carries the token.
const AUTHORIZATION: &str = "Authorization";

impl UserConfig {
    /// The user config in the account's folder `dir`: the first of `files` there (a link to
    /// nothing yet counts), else the first, to be made.
    pub fn file_in(&self, dir: &Path) -> PathBuf {
        self.files
            .iter()
            .map(|f| dir.join(f))
            .find(|p| std::fs::symlink_metadata(p).is_ok())
            .unwrap_or_else(|| dir.join(self.files[0]))
    }

    /// What the file `config` says of Escouade's server. An entry that has no address or no bearer
    /// token in its headers is there with them empty: not the one wanted.
    pub fn declared(&self, config: &Path) -> Standing {
        let text = match std::fs::read_to_string(config) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Standing::NoFile,
            Err(_) => return Standing::Unreadable,
        };
        match self.format {
            Format::Toml => self.declared_toml(&text),
            Format::Jsonc => self.declared_json(&text),
        }
    }

    fn declared_toml(&self, text: &str) -> Standing {
        let Ok(doc) = text.parse::<DocumentMut>() else {
            return Standing::Unreadable;
        };
        let mut at: &Item = doc.as_item();
        for key in self.servers.iter().copied().chain([SERVER]) {
            match at.as_table_like().and_then(|t| t.get(key)) {
                Some(next) => at = next,
                None => return Standing::Absent,
            }
        }
        let field = |key: &str| at.as_table_like().and_then(|t| t.get(key));
        let url = field(self.url).and_then(Item::as_str).unwrap_or_default();
        let token = field(self.headers)
            .and_then(Item::as_table_like)
            .and_then(|h| h.get(AUTHORIZATION))
            .and_then(Item::as_str)
            .and_then(|h| h.strip_prefix("Bearer "))
            .unwrap_or_default();
        Standing::Declared(Entry {
            url: url.to_string(),
            token: token.to_string(),
        })
    }

    fn declared_json(&self, text: &str) -> Standing {
        let Ok(root) = jsonc::parse(text) else {
            return Standing::Unreadable;
        };
        let root = root.to_value();
        let mut at = &root;
        for key in self.servers.iter().copied().chain([SERVER]) {
            match at.get(key) {
                Some(next) => at = next,
                None => return Standing::Absent,
            }
        }
        let url = at[self.url].as_str().unwrap_or_default();
        let token = at[self.headers][AUTHORIZATION]
            .as_str()
            .and_then(|h| h.strip_prefix("Bearer "))
            .unwrap_or_default();
        Standing::Declared(Entry {
            url: url.to_string(),
            token: token.to_string(),
        })
    }

    /// `entry` written in the file `config` (made, with its folder, when there is none), in place
    /// of an entry of that name; the rest of the file as it was. A `config` that is a link (an
    /// account sharing its Principal's settings, `accounts::link_file`) stays one: the file it
    /// leads to is the one written (`written_at`).
    pub fn add(&self, config: &Path, entry: &Entry) -> Result<()> {
        let config = &written_at(config);
        let text = self.read(config, true)?;
        let written = match self.format {
            Format::Toml => {
                let (mut doc, crlf) = self.parse_toml(config, &text)?;
                self.servers_of(&mut doc)?
                    .insert(SERVER, Item::Table(self.table(entry)));
                toml_text(&doc, crlf)
            }
            Format::Jsonc => self.add_json(config, &text, entry)?,
        };
        self.write(config, &written)
    }

    /// The server's entry taken out of the file `config`, and the table of the servers too when
    /// nothing is left in it and it had no line of its own (`[mcp_servers]`; JSON: when it is the
    /// one `add` made, and the file too when `add` made it from none, or from a blank one): the
    /// file as it was before `add`. No file, or no entry: nothing written.
    pub fn remove(&self, config: &Path) -> Result<()> {
        let config = &written_at(config);
        if !config.exists() {
            return Ok(());
        }
        let text = self.read(config, false)?;
        match self.format {
            Format::Toml => {
                let (mut doc, crlf) = self.parse_toml(config, &text)?;
                if !self.take_out(doc.as_table_mut(), self.servers)? {
                    return Ok(());
                }
                self.write(config, &toml_text(&doc, crlf))
            }
            Format::Jsonc => {
                let root = self.parse_json(config, &text)?;
                let Some(left) = self.remove_json(&text, &root) else {
                    return Ok(());
                };
                let made = matches!(self.declared_json(&text), Standing::Declared(had) if text == self.fresh(&had));
                if made {
                    return std::fs::remove_file(config).with_context(|| {
                        tr!(
                            "{file} n’a pas pu être supprimé",
                            "{file} could not be deleted",
                            file = config.display()
                        )
                    });
                }
                self.write(config, &left)
            }
        }
    }

    /// What to add to the file by hand: the entry alone, as `add` writes it (JSON: in the objects
    /// that hold it, as a file of its own would have it).
    pub fn snippet(&self, entry: &Entry) -> String {
        if self.format == Format::Jsonc {
            return self.fresh(entry).trim().to_string();
        }
        let mut doc = DocumentMut::new();
        match self.servers_of(&mut doc) {
            Ok(servers) => {
                servers.insert(SERVER, Item::Table(self.table(entry)));
            }
            Err(_) => return String::new(),
        }
        doc.to_string().trim().to_string()
    }

    /// The entry: the address, then the headers on one line.
    fn table(&self, entry: &Entry) -> Table {
        let mut headers = InlineTable::new();
        headers.insert(AUTHORIZATION, format!("Bearer {}", entry.token).into());
        let mut table = Table::new();
        table.insert(self.url, toml_edit::value(entry.url.as_str()));
        table.insert(self.headers, toml_edit::value(headers));
        table
    }

    /// The text of the file (an empty one when there is none and `missing_ok`, else an error).
    fn read(&self, config: &Path, missing_ok: bool) -> Result<String> {
        match std::fs::read_to_string(config) {
            Ok(text) => Ok(text),
            Err(e) if missing_ok && e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
            Err(e) => Err(e).with_context(|| {
                tr!(
                    "{file} n’a pas pu être lu",
                    "{file} could not be read",
                    file = config.display()
                )
            }),
        }
    }

    /// The TOML parsed, and whether its lines end with CRLF (`toml_edit` writes LF alone).
    fn parse_toml(&self, config: &Path, text: &str) -> Result<(DocumentMut, bool)> {
        let doc = text.parse::<DocumentMut>().map_err(|e| {
            anyhow!(tr!(
                "{file} n’est pas un TOML valide, il n’a pas été modifié : {e}",
                "{file} is not valid TOML, it was left alone: {e}",
                file = config.display(),
                e = e.message()
            ))
        })?;
        Ok((doc, text.contains("\r\n")))
    }

    /// The JSON parsed: an object.
    fn parse_json(&self, config: &Path, text: &str) -> Result<jsonc::Node> {
        let root = jsonc::parse(text).map_err(|e| {
            anyhow!(tr!(
                "{file} n’est pas un JSON valide, il n’a pas été modifié : {e}",
                "{file} is not valid JSON, it was left alone: {e}",
                file = config.display()
            ))
        })?;
        if root.members().is_none() {
            bail!(tr!(
                "{file} n’est pas un objet JSON, il n’a pas été modifié",
                "{file} is not a JSON object, it was left alone",
                file = config.display()
            ));
        }
        Ok(root)
    }

    /// The file written in one go, readable by its owner only: it holds a token.
    fn write(&self, config: &Path, text: &str) -> Result<()> {
        if let Some(dir) = config.parent() {
            std::fs::create_dir_all(dir)?;
        }
        escouade_model::paths::write_private(config, text.as_bytes()).with_context(|| {
            tr!(
                "{file} n’a pas pu être écrit",
                "{file} could not be written",
                file = config.display()
            )
        })
    }

    /// The table of the servers, made as the header of the entry alone (`[mcp_servers.escouade]`)
    /// when there is none.
    fn servers_of<'a>(&self, doc: &'a mut DocumentMut) -> Result<&'a mut Table> {
        let mut at = doc.as_table_mut();
        for key in self.servers {
            let item = at.entry(key).or_insert_with(|| {
                let mut table = Table::new();
                table.set_implicit(true);
                Item::Table(table)
            });
            at = item.as_table_mut().ok_or_else(|| self.not_a_table(key))?;
        }
        Ok(at)
    }

    /// The server's entry taken out of `table`, under `path`; whether there was one. A table of
    /// the path left empty and implicit goes with it.
    fn take_out(&self, table: &mut Table, path: &[&str]) -> Result<bool> {
        let Some((key, rest)) = path.split_first() else {
            return Ok(table.remove(SERVER).is_some());
        };
        let Some(item) = table.get_mut(key) else {
            return Ok(false);
        };
        let inner = item.as_table_mut().ok_or_else(|| self.not_a_table(key))?;
        let removed = self.take_out(inner, rest)?;
        if removed && inner.is_empty() && inner.is_implicit() {
            table.remove(key);
        }
        Ok(removed)
    }

    fn not_a_table(&self, key: &str) -> anyhow::Error {
        anyhow!(tr!(
            "« {key} » n’est pas une table dans {file} : il n’a pas été modifié",
            "“{key}” is not a table in {file}: it was left alone",
            file = self.files[0]
        ))
    }

    // ----- JSON -----

    /// The file `add` makes where there is none: the entry in the objects that hold it.
    fn fresh(&self, entry: &Entry) -> String {
        let member = self.member_text(self.servers, entry, "  ", "  ", "\n");
        format!("{{\n  {member}\n}}\n")
    }

    /// The member that holds the entry under `path` (`"mcp": { "escouade": { … } }`), its line
    /// indented by `indent`, `unit` more for each level, its lines ending with `nl`.
    fn member_text(
        &self,
        path: &[&str],
        entry: &Entry,
        indent: &str,
        unit: &str,
        nl: &str,
    ) -> String {
        let inner = format!("{indent}{unit}");
        let Some((key, rest)) = path.split_first() else {
            let mut fields: Vec<String> = self
                .fixed
                .iter()
                .map(|(k, v)| format!("{inner}{}: {}", quoted(k), quoted(v)))
                .collect();
            fields.push(format!(
                "{inner}{}: {}",
                quoted(self.url),
                quoted(&entry.url)
            ));
            fields.push(format!(
                "{inner}{}: {{ {}: {} }}",
                quoted(self.headers),
                quoted(AUTHORIZATION),
                quoted(&format!("Bearer {}", entry.token))
            ));
            return format!(
                "{}: {{{nl}{}{nl}{indent}}}",
                quoted(SERVER),
                fields.join(&format!(",{nl}"))
            );
        };
        format!(
            "{}: {{{nl}{inner}{}{nl}{indent}}}",
            quoted(key),
            self.member_text(rest, entry, &inner, unit, nl)
        )
    }

    /// `text` with `entry` in place of the server's entry there, or added at the end of the object
    /// of the servers (made at the end of the file's object when there is none). Every other byte
    /// as it was: what `remove_json` takes out is exactly what this put in.
    fn add_json(&self, config: &Path, text: &str, entry: &Entry) -> Result<String> {
        if text.trim().is_empty() {
            return Ok(self.fresh(entry));
        }
        let root = self.parse_json(config, text)?;
        let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let unit = indent_unit(text, &root);
        let mut at = &root;
        let mut depth = 0;
        for key in self.servers {
            let Some((_, m)) = at.member(key) else { break };
            if m.value.members().is_none() {
                return Err(self.not_a_table(key));
            }
            at = &m.value;
            depth += 1;
        }
        let rest = &self.servers[depth..];
        let outer = jsonc::line_indent(text, at.start);
        if rest.is_empty() {
            if let Some((_, m)) = at.member(SERVER) {
                let indent = jsonc::own_line_indent(text, m.key_start)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("{outer}{unit}"));
                let member = self.member_text(&[], entry, &indent, &unit, nl);
                let (head, tail) = (&text[..m.key_start], &text[m.value.end..]);
                return Ok(format!("{head}{member}{tail}"));
            }
        }
        let members = at.members().unwrap_or_default();
        Ok(match members.last() {
            Some(last) => {
                let indent = jsonc::own_line_indent(text, last.key_start)
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("{outer}{unit}"));
                let member = self.member_text(rest, entry, &indent, &unit, nl);
                let (head, tail) = text.split_at(last.value.end);
                format!("{head},{nl}{indent}{member}{tail}")
            }
            None => {
                let indent = format!("{outer}{unit}");
                let member = self.member_text(rest, entry, &indent, &unit, nl);
                let (open, close) = (at.start + 1, at.end - 1);
                // A comma after it (both harnesses take one): an object of the user's, emptied
                // again by `remove_json`, is not one `add` made, which goes with the entry.
                if text[open..close].trim().is_empty() {
                    let (head, tail) = (&text[..open], &text[close..]);
                    format!("{head}{nl}{indent}{member},{nl}{outer}{tail}")
                } else {
                    let (head, tail) = text.split_at(open);
                    format!("{head}{nl}{indent}{member},{tail}")
                }
            }
        })
    }

    /// `text` with the server's entry taken out, and each object of the servers it leaves empty
    /// when it is the one `add` made for it, byte for byte (an object of the user's, `"mcp": {}`,
    /// stays); None when there is no entry.
    fn remove_json(&self, text: &str, root: &jsonc::Node) -> Option<String> {
        let Standing::Declared(had) = self.declared_json(text) else {
            return None;
        };
        let nl = if text.contains("\r\n") { "\r\n" } else { "\n" };
        let unit = indent_unit(text, root);
        // Each object on the way, with the place of the member that leads on.
        let mut chain: Vec<(&jsonc::Node, usize)> = Vec::new();
        let mut at = root;
        for key in self.servers.iter().copied().chain([SERVER]) {
            let (i, m) = at.member(key)?;
            chain.push((at, i));
            at = &m.value;
        }
        let alone = |o: &jsonc::Node| o.members().map_or(0, <[_]>::len) == 1;
        // Whether the member that holds the last object of `chain` is the one `add` writes.
        let made = |chain: &[(&jsonc::Node, usize)]| {
            let level = chain.len() - 1;
            let (parent, i) = chain[level - 1];
            let m = &parent.members().unwrap_or_default()[i];
            let indent = jsonc::own_line_indent(text, m.key_start)
                .map(str::to_string)
                .unwrap_or_else(|| format!("{}{unit}", jsonc::line_indent(text, parent.start)));
            text[m.key_start..m.value.end]
                == self.member_text(&self.servers[level - 1..], &had, &indent, &unit, nl)
        };
        while chain.len() > 1 && chain.last().is_some_and(|(o, _)| alone(o)) && made(&chain) {
            chain.pop();
        }
        let &(object, i) = chain.last()?;
        let members = object.members()?;
        let m = &members[i];
        let (from, to) = if i > 0 {
            (members[i - 1].value.end, m.value.end)
        } else if let Some(next) = members.get(1) {
            (m.key_start, next.key_start)
        } else {
            // The only one: its comma too, and the blanks around it when nothing else is left.
            let (open, close) = (object.start + 1, object.end - 1);
            let mut to = m.value.end;
            let after = &text[to..close];
            if after.trim_start().starts_with(',') {
                to += after.len() - after.trim_start().len() + 1;
            }
            if text[to..close].trim().is_empty() && text[open..m.key_start].trim().is_empty() {
                (open, close)
            } else {
                (open, to)
            }
        };
        Some(format!("{}{}", &text[..from], &text[to..]))
    }
}

/// A key or a text as JSON writes it.
fn quoted(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_default()
}

/// What each level of a JSON file is indented by: its first member's blanks, else two spaces.
fn indent_unit(text: &str, root: &jsonc::Node) -> String {
    root.members()
        .and_then(|m| m.first())
        .and_then(|m| jsonc::own_line_indent(text, m.key_start))
        .filter(|i| !i.is_empty())
        .unwrap_or("  ")
        .to_string()
}

/// The TOML's text, its lines ending with CRLF when they did (`crlf`).
fn toml_text(doc: &DocumentMut, crlf: bool) -> String {
    let text = doc.to_string();
    if crlf {
        text.replace("\r\n", "\n").replace('\n', "\r\n")
    } else {
        text
    }
}

/// The file `config` is written as: itself, or for a link the file it leads to (followed to the
/// end, `canonicalize`; a link to nothing yet, its target), never a file in place of the link,
/// which would end the sharing for good. A link that cannot be read is written as itself.
pub fn written_at(config: &Path) -> PathBuf {
    let is_link = std::fs::symlink_metadata(config).is_ok_and(|m| m.file_type().is_symlink());
    if !is_link {
        return config.to_path_buf();
    }
    if let Ok(real) = std::fs::canonicalize(config) {
        return real;
    }
    match std::fs::read_link(config) {
        Ok(to) => config.parent().map_or(to.clone(), |dir| dir.join(&to)),
        Err(_) => config.to_path_buf(),
    }
}

/// What tells two user configs apart: two accounts whose `config.toml` leads to the same file
/// (one linked to Principal's) have the same.
pub fn same_file_key(config: &Path) -> PathBuf {
    std::fs::canonicalize(config).unwrap_or_else(|_| written_at(config))
}

/// Refused: a harness with no user config of its own to declare the server in.
pub fn none() -> Result<()> {
    bail!(tr!(
        "Ce harness ne déclare pas le serveur MCP d’Escouade.",
        "This harness does not declare Escouade’s MCP server."
    ))
}
