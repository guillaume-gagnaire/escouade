//! JSON with comments and trailing commas, as OpenCode's `opencode.json(c)` and Copilot CLI's
//! `mcp-config.json` are read (both take them, tried on OpenCode 1.19.3 and Copilot CLI 1.0.98),
//! parsed with where each value lies in the text: a member is put in or taken out with every other
//! byte of the file as it was (`userconfig`).

use serde_json::Value;

/// A value, from its first byte to the one after its last.
#[derive(Debug)]
pub struct Node {
    pub start: usize,
    pub end: usize,
    pub kind: Kind,
}

#[derive(Debug)]
pub enum Kind {
    Object(Vec<Member>),
    Array(Vec<Node>),
    Scalar(Value),
}

/// A member of an object: its key (decoded), where the key's quote opens, its value.
#[derive(Debug)]
pub struct Member {
    pub key: String,
    pub key_start: usize,
    pub value: Node,
}

impl Node {
    /// Its members, for an object.
    pub fn members(&self) -> Option<&[Member]> {
        match &self.kind {
            Kind::Object(m) => Some(m),
            _ => None,
        }
    }

    /// The member `key` of an object, with its place among the members.
    pub fn member(&self, key: &str) -> Option<(usize, &Member)> {
        self.members()?
            .iter()
            .enumerate()
            .find(|(_, m)| m.key == key)
    }

    /// The value as `serde_json` has it (the last of two members of one name wins, as in
    /// JavaScript).
    pub fn to_value(&self) -> Value {
        match &self.kind {
            Kind::Object(members) => Value::Object(
                members
                    .iter()
                    .map(|m| (m.key.clone(), m.value.to_value()))
                    .collect(),
            ),
            Kind::Array(items) => Value::Array(items.iter().map(Node::to_value).collect()),
            Kind::Scalar(v) => v.clone(),
        }
    }
}

/// How deep values may nest: deeper, the text is refused rather than the stack used up.
const DEPTH: usize = 128;

/// `text` parsed: one value, blanks and comments around it (a byte order mark first is skipped).
/// Err with what is wrong and where (its byte).
pub fn parse(text: &str) -> Result<Node, String> {
    let mut p = Parser {
        text,
        s: text.as_bytes(),
        i: if text.starts_with('\u{feff}') { 3 } else { 0 },
    };
    let root = p.value(0)?;
    p.skip()?;
    if p.i < p.s.len() {
        return Err(p.error(Problem::EndExpected));
    }
    Ok(root)
}

/// What is wrong in a text.
#[derive(Clone, Copy)]
enum Problem {
    EndExpected,
    CommentOpen,
    TooDeep,
    ValueExpected,
    StringOpen,
    BadString,
    BadValue,
    KeyExpected,
    ColonExpected,
    ObjectGoesOn,
    ArrayGoesOn,
}

impl Problem {
    fn said(self) -> String {
        match self {
            Problem::EndExpected => tr!("fin du texte attendue", "end of text expected"),
            Problem::CommentOpen => tr!("commentaire non fermé", "comment not closed"),
            Problem::TooDeep => tr!("valeurs trop imbriquées", "values nested too deep"),
            Problem::ValueExpected => tr!("valeur attendue", "value expected"),
            Problem::StringOpen => tr!("chaîne non fermée", "string not closed"),
            Problem::BadString => tr!("chaîne invalide", "invalid string"),
            Problem::BadValue => tr!("valeur invalide", "invalid value"),
            Problem::KeyExpected => tr!("clé ou « }} » attendue", "key or “}}” expected"),
            Problem::ColonExpected => tr!("« : » attendu", "“:” expected"),
            Problem::ObjectGoesOn => tr!("« , » ou « }} » attendu", "“,” or “}}” expected"),
            Problem::ArrayGoesOn => tr!("« , » ou « ] » attendu", "“,” or “]” expected"),
        }
    }
}

struct Parser<'a> {
    text: &'a str,
    s: &'a [u8],
    i: usize,
}

impl Parser<'_> {
    fn error(&self, what: Problem) -> String {
        let at = self.i;
        let what = what.said();
        tr!("{what} (octet {at})", "{what} (byte {at})")
    }

    fn peek(&self) -> Option<u8> {
        self.s.get(self.i).copied()
    }

    /// Blanks and comments (`// …` to the end of the line, `/* … */`) skipped.
    fn skip(&mut self) -> Result<(), String> {
        loop {
            while self.peek().is_some_and(|c| c.is_ascii_whitespace()) {
                self.i += 1;
            }
            let rest = &self.s[self.i..];
            if rest.starts_with(b"//") {
                self.i += rest.iter().position(|&c| c == b'\n').unwrap_or(rest.len());
            } else if rest.starts_with(b"/*") {
                let end = rest[2..]
                    .windows(2)
                    .position(|w| w == b"*/")
                    .ok_or_else(|| self.error(Problem::CommentOpen))?;
                self.i += end + 4;
            } else {
                return Ok(());
            }
        }
    }

    fn value(&mut self, depth: usize) -> Result<Node, String> {
        if depth > DEPTH {
            return Err(self.error(Problem::TooDeep));
        }
        self.skip()?;
        let start = self.i;
        let kind = match self.peek() {
            Some(b'{') => Kind::Object(self.object(depth)?),
            Some(b'[') => Kind::Array(self.array(depth)?),
            Some(b'"') => Kind::Scalar(Value::String(self.string()?)),
            Some(_) => Kind::Scalar(self.scalar()?),
            None => return Err(self.error(Problem::ValueExpected)),
        };
        Ok(Node {
            start,
            end: self.i,
            kind,
        })
    }

    fn string(&mut self) -> Result<String, String> {
        let start = self.i;
        self.i += 1;
        loop {
            match self.peek() {
                Some(b'\\') => self.i += 2,
                Some(b'"') => break,
                Some(_) => self.i += 1,
                None => return Err(self.error(Problem::StringOpen)),
            }
        }
        self.i += 1;
        serde_json::from_str(&self.text[start..self.i]).map_err(|_| self.error(Problem::BadString))
    }

    /// A number, `true`, `false` or `null`.
    fn scalar(&mut self) -> Result<Value, String> {
        let start = self.i;
        while self
            .peek()
            .is_some_and(|c| c.is_ascii_alphanumeric() || matches!(c, b'-' | b'+' | b'.'))
        {
            self.i += 1;
        }
        serde_json::from_str(&self.text[start..self.i]).map_err(|_| self.error(Problem::BadValue))
    }

    fn object(&mut self, depth: usize) -> Result<Vec<Member>, String> {
        self.i += 1;
        let mut members = Vec::new();
        loop {
            self.skip()?;
            match self.peek() {
                Some(b'}') => break,
                Some(b'"') => {}
                _ => return Err(self.error(Problem::KeyExpected)),
            }
            let key_start = self.i;
            let key = self.string()?;
            self.skip()?;
            if self.peek() != Some(b':') {
                return Err(self.error(Problem::ColonExpected));
            }
            self.i += 1;
            let value = self.value(depth + 1)?;
            members.push(Member {
                key,
                key_start,
                value,
            });
            self.skip()?;
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b'}') => break,
                _ => return Err(self.error(Problem::ObjectGoesOn)),
            }
        }
        self.i += 1;
        Ok(members)
    }

    fn array(&mut self, depth: usize) -> Result<Vec<Node>, String> {
        self.i += 1;
        let mut items = Vec::new();
        loop {
            self.skip()?;
            if self.peek() == Some(b']') {
                break;
            }
            items.push(self.value(depth + 1)?);
            self.skip()?;
            match self.peek() {
                Some(b',') => self.i += 1,
                Some(b']') => break,
                _ => return Err(self.error(Problem::ArrayGoesOn)),
            }
        }
        self.i += 1;
        Ok(items)
    }
}

/// The blanks that start the line of `at`.
pub fn line_indent(text: &str, at: usize) -> &str {
    let start = text[..at].rfind('\n').map_or(0, |n| n + 1);
    let line = &text[start..];
    let blanks = line.len() - line.trim_start_matches([' ', '\t']).len();
    &line[..blanks]
}

/// The blanks before `at` on its line, when nothing else is: `at` starts its line.
pub fn own_line_indent(text: &str, at: usize) -> Option<&str> {
    let start = text[..at].rfind('\n').map_or(0, |n| n + 1);
    let before = &text[start..at];
    before
        .chars()
        .all(|c| c == ' ' || c == '\t')
        .then_some(before)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn comments_trailing_commas_and_a_byte_order_mark_are_read_as_opencode_and_copilot_read_them() {
        let text = "\u{feff}{\n  // un\n  \"a\": [1, 2,], /* deux */\n  \"b\": { \"c\": \"x\\\"y\", },\n  \"d\": null, \"e\": -1.5e3, \"f\": true,\n}\n";
        let root = parse(text).unwrap();
        assert_eq!(
            root.to_value(),
            json!({ "a": [1, 2], "b": { "c": "x\"y" }, "d": null, "e": -1.5e3, "f": true })
        );
        let (i, b) = root.member("b").unwrap();
        assert_eq!(i, 1);
        assert_eq!(
            &text[b.key_start..b.value.end],
            "\"b\": { \"c\": \"x\\\"y\", }"
        );
        assert_eq!(line_indent(text, b.value.start), "  ");
        assert_eq!(own_line_indent(text, b.key_start), Some("  "));
        let (_, e) = root.member("e").unwrap();
        assert_eq!(own_line_indent(text, e.key_start), None);
    }

    #[test]
    fn what_is_no_json_is_refused_with_where() {
        for bad in [
            "",
            "{",
            "{\"a\" 1}",
            "{\"a\": }",
            "[1 2]",
            "{} x",
            "/* sans fin",
            "{\"a\": tru}",
            "\"x",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }
        let deep = "[".repeat(DEPTH + 2);
        assert!(parse(&deep).is_err());
        assert!(parse("  // rien d'autre\n 3 ").is_ok());
    }
}
