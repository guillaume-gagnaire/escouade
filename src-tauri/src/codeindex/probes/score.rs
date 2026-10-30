//! The scores of the probes (ESC-294): what is right in an answer, what it costs to read.
//!
//! - An expected answer is a list of items; an item is found when one of its alternatives
//!   (`path`, `path:line` or `path:from-to`, joined by `|`) is: a result of the same path whose line
//!   falls in the range (any line for a whole file).
//! - Precision: the results that hit an item, over the results. Recall: the items found, over the
//!   items. The rank: the place (from 1) of the first result that hits one.
//! - Tokens: about four bytes each (`tokens_of`), the same count for the index and for grep.

use super::super::tokens_of;
use serde::Serialize;

/// One way of being right: a file, whole (`from` 0) or lines `from..=to` of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Alt {
    pub path: String,
    pub from: u32,
    pub to: u32,
}

/// A result, for the scores: where it points. Line 0 for a whole file (a file that imports).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Item {
    pub path: String,
    pub line: u32,
    /// The last line it covers: `line` for a place, the end of a symbol given whole (tokensave's
    /// `line`..`end_line`, ESC-304).
    pub end: u32,
}

impl Item {
    pub fn new(path: &str, line: u32) -> Self {
        Self::span(path, line, line)
    }

    pub fn span(path: &str, line: u32, end: u32) -> Self {
        Self {
            path: norm(path),
            line,
            end: end.max(line),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Score {
    pub results: usize,
    /// Results that hit an expected item.
    pub good: usize,
    pub expected: usize,
    /// Expected items found.
    pub found: usize,
    pub precision: f64,
    pub recall: f64,
    /// The rank (from 1) of the first good result.
    pub first_good: Option<usize>,
}

fn norm(path: &str) -> String {
    path.trim()
        .replace('\\', "/")
        .trim_start_matches("./")
        .to_string()
}

/// `a.rs:10-20|b.rs:3|c.md`: the alternatives of an expected item.
pub fn parse_expect(text: &str) -> Vec<Alt> {
    text.split('|')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(parse_alt)
        .collect()
}

fn parse_alt(s: &str) -> Alt {
    let whole = |p: &str| Alt {
        path: norm(p),
        from: 0,
        to: 0,
    };
    let Some((path, lines)) = s.rsplit_once(':') else {
        return whole(s);
    };
    let (a, b) = lines.split_once('-').unwrap_or((lines, lines));
    match (a.trim().parse::<u32>(), b.trim().parse::<u32>()) {
        (Ok(from), Ok(to)) if from > 0 && to >= from => Alt {
            path: norm(path),
            from,
            to,
        },
        _ => whole(s),
    }
}

/// Whether `item` is this alternative: a line of it falls in the range.
pub fn hits(alt: &Alt, item: &Item) -> bool {
    alt.path == item.path && (alt.from == 0 || (item.line <= alt.to && item.end >= alt.from))
}

/// For each result, whether it hits an expected item.
pub fn good_flags(expect: &[Vec<Alt>], items: &[Item]) -> Vec<bool> {
    items
        .iter()
        .map(|i| expect.iter().flatten().any(|a| hits(a, i)))
        .collect()
}

/// The scores of `items` against `expect`. An expected answer that is empty says nothing must come
/// out: recall is whole, precision is whole when nothing came out and nought else.
pub fn score(expect: &[Vec<Alt>], items: &[Item]) -> Score {
    let flags = good_flags(expect, items);
    let good = flags.iter().filter(|g| **g).count();
    let found = expect
        .iter()
        .filter(|alts| items.iter().any(|i| alts.iter().any(|a| hits(a, i))))
        .count();
    let precision = if items.is_empty() {
        if expect.is_empty() {
            1.0
        } else {
            0.0
        }
    } else {
        good as f64 / items.len() as f64
    };
    let recall = if expect.is_empty() {
        1.0
    } else {
        found as f64 / expect.len() as f64
    };
    Score {
        results: items.len(),
        good,
        expected: expect.len(),
        found,
        precision,
        recall,
        first_good: flags.iter().position(|g| *g).map(|p| p + 1),
    }
}

/// Estimated tokens of a text: the app's count (about four bytes each).
pub fn tokens(text: &str) -> u64 {
    tokens_of(text.len() as u64)
}

/// How many results an agent reads to conclude. A question of one answer: up to the first good one
/// (all, when none is). A question of several (the callers, what a change touches): all, since
/// each must be checked.
pub fn read_count(expected: usize, flags: &[bool]) -> usize {
    if expected <= 1 {
        flags
            .iter()
            .position(|g| *g)
            .map(|p| p + 1)
            .unwrap_or(flags.len())
    } else {
        flags.len()
    }
}

/// What an agent reads to conclude: `radius` lines around each of the first `max` results (the top
/// of a file for a whole file), the windows of one file merged. In the order of the results.
pub fn windows(items: &[Item], radius: u32, max: usize) -> Vec<(String, u32, u32)> {
    let mut out: Vec<(String, u32, u32)> = items
        .iter()
        .take(max)
        .map(|i| match i.line {
            0 => (i.path.clone(), 1, 2 * radius + 1),
            n => (i.path.clone(), n.saturating_sub(radius).max(1), n + radius),
        })
        .collect();
    // Windows of one file that touch are one, until none do.
    loop {
        let mut joined = false;
        let mut merged: Vec<(String, u32, u32)> = Vec::new();
        for w in out {
            if let Some(m) = merged
                .iter_mut()
                .find(|m| m.0 == w.0 && w.1 <= m.2 + 1 && w.2 + 1 >= m.1)
            {
                m.1 = m.1.min(w.1);
                m.2 = m.2.max(w.2);
                joined = true;
            } else {
                merged.push(w);
            }
        }
        out = merged;
        if !joined {
            return out;
        }
    }
}

/// What `cost` saves against `against`, in percent (negative when it costs more).
pub fn economy(cost: u64, against: u64) -> i64 {
    if against == 0 {
        return 0;
    }
    (against as i64 - cost as i64) * 100 / against as i64
}
