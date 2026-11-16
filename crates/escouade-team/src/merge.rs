//! The three-way merge of a text, line by line (TS-6, `docs/TEAM-SERVER.md` §4.3): a ticket's description changed on
//! two devices from the same base. Each side's changes against the base are kept when they touch different lines;
//! two sides that change the same lines differently are a conflict, and the card offers « Garder le mien » and
//! « Prendre le sien ».

/// The longest a side may be for the merge to be tried (lines): past it, the conflict is shown as it is.
pub const MAX_LINES: usize = 4000;

/// For each line of `base`, the line of `other` it is matched to by their longest common subsequence.
fn matches(base: &[&str], other: &[&str]) -> Vec<Option<usize>> {
    let (n, m) = (base.len(), other.len());
    let mut lcs = vec![0u32; (n + 1) * (m + 1)];
    let at = |i: usize, j: usize| i * (m + 1) + j;
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[at(i, j)] = if base[i] == other[j] {
                lcs[at(i + 1, j + 1)] + 1
            } else {
                lcs[at(i + 1, j)].max(lcs[at(i, j + 1)])
            };
        }
    }
    let mut out = vec![None; n];
    let (mut i, mut j) = (0, 0);
    while i < n && j < m {
        if base[i] == other[j] {
            out[i] = Some(j);
            i += 1;
            j += 1;
        } else if lcs[at(i + 1, j)] >= lcs[at(i, j + 1)] {
            i += 1;
        } else {
            j += 1;
        }
    }
    out
}

/// Merges `ours` and `theirs`, both changed from `base`: the merged text, or None when they change the same lines in
/// two ways (or are too long to try).
pub fn merge3(base: &str, ours: &str, theirs: &str) -> Option<String> {
    if ours == theirs || theirs == base {
        return Some(ours.to_string());
    }
    if ours == base {
        return Some(theirs.to_string());
    }
    let b: Vec<&str> = base.split_inclusive('\n').collect();
    let o: Vec<&str> = ours.split_inclusive('\n').collect();
    let t: Vec<&str> = theirs.split_inclusive('\n').collect();
    if b.len().max(o.len()).max(t.len()) > MAX_LINES {
        return None;
    }
    let mo = matches(&b, &o);
    let mt = matches(&b, &t);
    let mut out = String::new();
    let (mut i, mut a, mut c) = (0usize, 0usize, 0usize);
    loop {
        // The next line of the base both sides kept: what lies before it is a chunk to merge.
        let stable = (i..b.len())
            .find(|&j| matches!((mo[j], mt[j]), (Some(x), Some(y)) if x >= a && y >= c));
        let (bj, oj, tj) = match stable {
            Some(j) => (j, mo[j]?, mt[j]?),
            None => (b.len(), o.len(), t.len()),
        };
        let (bb, oo, tt) = (&b[i..bj], &o[a..oj], &t[c..tj]);
        let chunk = if oo == bb {
            tt
        } else if tt == bb || oo == tt {
            oo
        } else {
            return None;
        };
        out.extend(chunk.iter().copied());
        match stable {
            Some(j) => {
                out.push_str(b[j]);
                i = j + 1;
                a = oj + 1;
                c = tj + 1;
            }
            None => return Some(out),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn changes_on_different_lines_are_both_kept() {
        let base = "un\ndeux\ntrois\nquatre\n";
        let ours = "un\nDEUX\ntrois\nquatre\n";
        let theirs = "un\ndeux\ntrois\nquatre\ncinq\n";
        assert_eq!(
            merge3(base, ours, theirs).unwrap(),
            "un\nDEUX\ntrois\nquatre\ncinq\n"
        );
        assert_eq!(
            merge3(base, theirs, ours).unwrap(),
            "un\nDEUX\ntrois\nquatre\ncinq\n"
        );
    }

    #[test]
    fn the_same_lines_changed_two_ways_do_not_merge() {
        let base = "un\ndeux\ntrois\n";
        assert_eq!(merge3(base, "un\nDEUX\ntrois\n", "un\nZWEI\ntrois\n"), None);
    }

    #[test]
    fn one_side_unchanged_or_both_the_same_take_the_other() {
        let base = "a\nb\n";
        assert_eq!(merge3(base, base, "a\nc\n").unwrap(), "a\nc\n");
        assert_eq!(merge3(base, "x\n", base).unwrap(), "x\n");
        assert_eq!(merge3(base, "a\nc\n", "a\nc\n").unwrap(), "a\nc\n");
    }

    #[test]
    fn lines_removed_on_one_side_and_added_on_the_other() {
        let base = "a\nb\nc\nd\n";
        let ours = "a\nc\nd\n";
        let theirs = "a\nb\nc\nd\ne\n";
        assert_eq!(merge3(base, ours, theirs).unwrap(), "a\nc\nd\ne\n");
        // An empty base: both sides wrote a text of their own.
        assert_eq!(merge3("", "x\n", "y\n"), None);
    }
}
