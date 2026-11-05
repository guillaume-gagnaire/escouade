//! The HTML of Azure Boards' fields (a work item's description, its acceptance criteria) as markdown.

/// An HTML text (Azure Boards' fields) as markdown: paragraphs and line breaks, headings, lists
/// (`- `, `1. `, a checkbox's `[ ]`), bold and code; the other tags left out, the entities read.
pub fn html_to_markdown(html: &str) -> String {
    let mut out = String::new();
    // The lists the text is in: ordered or not, and their items so far.
    let mut lists: Vec<(bool, usize)> = Vec::new();
    let mut rest = html;
    let mut skip = false;
    while !rest.is_empty() {
        let Some(open) = rest.find('<') else {
            if !skip {
                push_text(&mut out, rest);
            }
            break;
        };
        if !skip {
            push_text(&mut out, &rest[..open]);
        }
        let after = &rest[open + 1..];
        let Some(close) = after.find('>') else {
            push_text(&mut out, &rest[open..]);
            break;
        };
        let tag = &after[..close];
        rest = &after[close + 1..];
        let ending = tag.starts_with('/');
        let name: String = tag
            .trim_start_matches('/')
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_ascii_lowercase();
        match (name.as_str(), ending) {
            ("script" | "style", e) => skip = !e,
            ("br" | "div" | "tr", _) => line_break(&mut out, 1),
            ("p", _) => line_break(&mut out, 2),
            (h, false)
                if h.len() == 2 && h.starts_with('h') && h.as_bytes()[1].is_ascii_digit() =>
            {
                line_break(&mut out, 2);
                let level = (h.as_bytes()[1] - b'0').clamp(1, 6) as usize;
                out.push_str(&"#".repeat(level));
                out.push(' ');
            }
            (h, true) if h.len() == 2 && h.starts_with('h') && h.as_bytes()[1].is_ascii_digit() => {
                line_break(&mut out, 2)
            }
            ("ul" | "ol", false) => {
                lists.push((name == "ol", 0));
                line_break(&mut out, 1);
            }
            ("ul" | "ol", true) => {
                lists.pop();
                line_break(&mut out, 1);
            }
            ("li", false) => {
                line_break(&mut out, 1);
                let depth = lists.len().max(1);
                out.push_str(&"  ".repeat(depth - 1));
                match lists.last_mut() {
                    Some((true, n)) => {
                        *n += 1;
                        out.push_str(&format!("{n}. "));
                    }
                    _ => out.push_str("- "),
                }
            }
            ("li", true) => line_break(&mut out, 1),
            ("input", false) if tag.to_ascii_lowercase().contains("checkbox") => {
                out.push_str(if tag.to_ascii_lowercase().contains("checked") {
                    "[x] "
                } else {
                    "[ ] "
                });
            }
            ("b" | "strong", _) => out.push_str("**"),
            ("code", _) => out.push('`'),
            _ => {}
        }
    }
    let mut lines: Vec<&str> = Vec::new();
    for line in out.lines().map(str::trim_end) {
        if line.trim().is_empty() && lines.last().is_none_or(|l| l.trim().is_empty()) {
            continue;
        }
        lines.push(line);
    }
    while lines.last().is_some_and(|l| l.trim().is_empty()) {
        lines.pop();
    }
    lines.join("\n")
}

/// Ends the line, with at least `n - 1` empty lines after it (none at the start).
fn line_break(out: &mut String, n: usize) {
    out.truncate(out.trim_end_matches(' ').len());
    if out.is_empty() {
        return;
    }
    let ends = out.len() - out.trim_end_matches('\n').len();
    for _ in ends..n {
        out.push('\n');
    }
}

/// A text of HTML, its entities read and its runs of spaces one space (none at a line's start).
fn push_text(out: &mut String, text: &str) {
    let text = entities(text);
    for c in text.chars() {
        if c.is_whitespace() && c != '\u{a0}' {
            if !(out.is_empty() || out.ends_with([' ', '\n'])) {
                out.push(' ');
            }
        } else if c == '\u{a0}' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
}

/// The entities of HTML read: named ones (`&amp;`, `&nbsp;`…) and numbered (`&#39;`, `&#x2019;`).
fn entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(i) = rest.find('&') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        let end = after.find(';').filter(|&e| e <= 10);
        let read = end.and_then(|e| {
            let name = &after[..e];
            let c = match name {
                "amp" => '&',
                "lt" => '<',
                "gt" => '>',
                "quot" => '"',
                "apos" => '\'',
                "nbsp" => '\u{a0}',
                n if n.starts_with("#x") || n.starts_with("#X") => {
                    char::from_u32(u32::from_str_radix(&n[2..], 16).ok()?)?
                }
                n if n.starts_with('#') => char::from_u32(n[1..].parse().ok()?)?,
                _ => return None,
            };
            Some((c, e))
        });
        match read {
            Some((c, e)) => {
                out.push(c);
                rest = &after[e + 1..];
            }
            None => {
                out.push('&');
                rest = after;
            }
        }
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn html_reads_as_markdown() {
        let html = "<div>Le paiement&nbsp;plante &amp; l&#39;app <b>quitte</b>.</div>\
            <div><br></div><h2>Étapes</h2><ol><li>Ouvrir   le panier</li><li>Payer</li></ol>\
            <p>Voir <code>pay.ts</code> &lt;ici&gt;</p><ul><li><input type=\"checkbox\" checked> fait\
            <ul><li>dessous</li></ul></li><li><input type=\"checkbox\"> à faire</li></ul>\
            <style>.x{}</style>&unknown; &#x2019;";
        assert_eq!(
            html_to_markdown(html),
            "Le paiement plante & l'app **quitte**.\n\n## Étapes\n\n1. Ouvrir le panier\n2. Payer\n\n\
             Voir `pay.ts` <ici>\n\n- [x] fait\n  - dessous\n- [ ] à faire\n&unknown; ’"
        );
        assert_eq!(html_to_markdown(""), "");
        assert_eq!(html_to_markdown("texte seul"), "texte seul");
    }
}
