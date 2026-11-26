//! The canonical JSON of RFC 8785 (JCS): what is signed and what is chained (§5.2). Objects' members sorted by the
//! UTF-16 code units of their names, no white space, strings in their shortest escape, numbers as ECMAScript writes
//! them (`Number.prototype.toString`).

use serde_json::Value;

/// Why a value has no canonical form.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum JcsError {
    #[error("a number that is not finite")]
    NotFinite,
}

/// The canonical bytes of `v`.
pub fn canonical(v: &Value) -> Result<Vec<u8>, JcsError> {
    let mut out = String::new();
    write(v, &mut out)?;
    Ok(out.into_bytes())
}

/// The canonical form of a JSON text. Its numbers are read by Rust's own parser, correctly rounded (serde_json's
/// default parser may miss the last bit of a long decimal, as `333333333.33333329` of RFC 8785 §3.2.2.3); a name given
/// twice is refused (§3.1).
pub fn canonical_text(json: &str) -> Result<String, String> {
    let mut p = Parser { s: json, at: 0 };
    let v = p.value()?;
    p.space();
    if p.at != json.len() {
        return Err(format!("text after the value at {}", p.at));
    }
    canonical(&v)
        .map(|b| String::from_utf8(b).expect("canonical JSON is UTF-8"))
        .map_err(|e| e.to_string())
}

struct Parser<'a> {
    s: &'a str,
    at: usize,
}

impl Parser<'_> {
    fn space(&mut self) {
        while self.s[self.at..].starts_with([' ', '\t', '\n', '\r']) {
            self.at += 1;
        }
    }

    fn eat(&mut self, c: char) -> Result<(), String> {
        self.space();
        if self.s[self.at..].starts_with(c) {
            self.at += 1;
            Ok(())
        } else {
            Err(format!("« {c} » expected at {}", self.at))
        }
    }

    fn string(&mut self) -> Result<String, String> {
        self.space();
        let start = self.at;
        let b = self.s.as_bytes();
        if b.get(self.at) != Some(&b'"') {
            return Err(format!("a string expected at {start}"));
        }
        let mut i = self.at + 1;
        while i < b.len() && b[i] != b'"' {
            i += if b[i] == b'\\' { 2 } else { 1 };
        }
        if i >= b.len() {
            return Err(format!("a string not closed at {start}"));
        }
        self.at = i + 1;
        serde_json::from_str(&self.s[start..self.at]).map_err(|e| e.to_string())
    }

    fn value(&mut self) -> Result<Value, String> {
        self.space();
        let rest = &self.s[self.at..];
        match rest.chars().next() {
            Some('{') => {
                self.at += 1;
                let mut map = serde_json::Map::new();
                self.space();
                if self.s[self.at..].starts_with('}') {
                    self.at += 1;
                    return Ok(Value::Object(map));
                }
                loop {
                    let k = self.string()?;
                    self.eat(':')?;
                    let v = self.value()?;
                    if map.insert(k.clone(), v).is_some() {
                        return Err(format!("the name « {k} » given twice"));
                    }
                    self.space();
                    if self.s[self.at..].starts_with(',') {
                        self.at += 1;
                    } else {
                        self.eat('}')?;
                        return Ok(Value::Object(map));
                    }
                }
            }
            Some('[') => {
                self.at += 1;
                let mut items = Vec::new();
                self.space();
                if self.s[self.at..].starts_with(']') {
                    self.at += 1;
                    return Ok(Value::Array(items));
                }
                loop {
                    items.push(self.value()?);
                    self.space();
                    if self.s[self.at..].starts_with(',') {
                        self.at += 1;
                    } else {
                        self.eat(']')?;
                        return Ok(Value::Array(items));
                    }
                }
            }
            Some('"') => self.string().map(Value::String),
            _ => {
                for (word, v) in [
                    ("null", Value::Null),
                    ("true", Value::Bool(true)),
                    ("false", Value::Bool(false)),
                ] {
                    if rest.starts_with(word) {
                        self.at += word.len();
                        return Ok(v);
                    }
                }
                let n = rest
                    .find(|c: char| {
                        !(c.is_ascii_digit() || matches!(c, '-' | '+' | '.' | 'e' | 'E'))
                    })
                    .unwrap_or(rest.len());
                let x: f64 = rest[..n]
                    .parse()
                    .map_err(|_| format!("a value expected at {}", self.at))?;
                self.at += n;
                serde_json::Number::from_f64(x)
                    .map(Value::Number)
                    .ok_or_else(|| "a number that is not finite".to_owned())
            }
        }
    }
}

fn write(v: &Value, out: &mut String) -> Result<(), JcsError> {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&number(n.as_f64().ok_or(JcsError::NotFinite)?)?),
        Value::String(s) => string(s, out),
        Value::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(item, out)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            let mut members: Vec<(&String, &Value)> = map.iter().collect();
            members.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            out.push('{');
            for (i, (k, item)) in members.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                string(k, out);
                out.push(':');
                write(item, out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

/// A number as ECMAScript writes it (ECMA-262, `Number::toString`), from the shortest digits that read back to it.
pub fn number(x: f64) -> Result<String, JcsError> {
    if !x.is_finite() {
        return Err(JcsError::NotFinite);
    }
    if x == 0.0 {
        return Ok("0".to_owned());
    }
    if x < 0.0 {
        return number(-x).map(|s| format!("-{s}"));
    }
    let mut buf = ryu::Buffer::new();
    let shortest = buf.format_finite(x);
    let (mantissa, exp) = match shortest.split_once('e') {
        Some((m, e)) => (m, e.parse::<i32>().expect("ryu's exponent")),
        None => (shortest, 0),
    };
    let (int, frac) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    let mut digits: String = format!("{int}{frac}");
    // x = 0.digits × 10^n
    let mut n = int.len() as i32 + exp;
    while digits.starts_with('0') {
        digits.remove(0);
        n -= 1;
    }
    while digits.ends_with('0') {
        digits.pop();
    }
    let k = digits.len() as i32;
    let s = if k <= n && n <= 21 {
        format!("{digits}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{digits}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let sign = if e < 0 { '-' } else { '+' };
        if k == 1 {
            format!("{digits}e{sign}{}", e.abs())
        } else {
            format!("{}.{}e{sign}{}", &digits[..1], &digits[1..], e.abs())
        }
    };
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 8785, appendix B: the numbers, by their IEEE 754 bits.
    #[test]
    fn the_numbers_of_rfc_8785_appendix_b() {
        let cases: [(u64, &str); 24] = [
            (0x0000000000000000, "0"),
            (0x8000000000000000, "0"),
            (0x0000000000000001, "5e-324"),
            (0x8000000000000001, "-5e-324"),
            (0x7fefffffffffffff, "1.7976931348623157e+308"),
            (0xffefffffffffffff, "-1.7976931348623157e+308"),
            (0x4340000000000000, "9007199254740992"),
            (0xc340000000000000, "-9007199254740992"),
            (0x4430000000000000, "295147905179352830000"),
            (0x44b52d02c7e14af5, "9.999999999999997e+22"),
            (0x44b52d02c7e14af6, "1e+23"),
            (0x44b52d02c7e14af7, "1.0000000000000001e+23"),
            (0x444b1ae4d6e2ef4e, "999999999999999700000"),
            (0x444b1ae4d6e2ef4f, "999999999999999900000"),
            (0x444b1ae4d6e2ef50, "1e+21"),
            (0x3eb0c6f7a0b5ed8c, "9.999999999999997e-7"),
            (0x3eb0c6f7a0b5ed8d, "0.000001"),
            (0x41b3de4355555553, "333333333.3333332"),
            (0x41b3de4355555554, "333333333.33333325"),
            (0x41b3de4355555555, "333333333.3333333"),
            (0x41b3de4355555556, "333333333.3333334"),
            (0x41b3de4355555557, "333333333.33333343"),
            (0xbecbf647612f3696, "-0.0000033333333333333333"),
            (0x43143ff3c1cb0959, "1424953923781206.2"),
        ];
        for (bits, want) in cases {
            assert_eq!(number(f64::from_bits(bits)).unwrap(), want, "{bits:016x}");
        }
        assert_eq!(number(f64::NAN), Err(JcsError::NotFinite));
        assert_eq!(number(f64::INFINITY), Err(JcsError::NotFinite));
    }
}
