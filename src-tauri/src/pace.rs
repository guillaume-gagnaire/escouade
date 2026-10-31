//! The one place that says whether an account is a good one to send work to (PR #20), meter by
//! meter, as the length of the meter's window asks:
//!
//! - A **short** window (24 h or less: the 5 hours of Claude Code and Codex) is a short pause. At
//!   the threshold the account is out until that window starts again, then it is a candidate like
//!   any other: a full 5 hours never weighs on the week.
//! - A **long** window (more than 24 h: the week, Codex's 30 days; and the month's credits of
//!   Copilot CLI, a window of the month that ends at their reset) is judged by its pace, with the
//!   projection to its reset that the status bar shows (`projected_pct`, as `projectedPct` of
//!   src/lib/accounts.ts):
//!   - at 100 % (the real limit) it is out until it starts again;
//!   - projected at 100 % or less it is good, even past the threshold: a week at 88 % that starts
//!     again in 3 hours is not to be wasted;
//!   - at `NEAR_LIMIT` (95 %) or more it goes behind the others whatever its pace: new agents do not
//!     start so near its limit while another account can take them;
//!   - projected past 100 % it goes behind the others, even under the threshold: at this pace it
//!     would not last to its reset;
//!   - without a projection (the first day, an unknown reset) the threshold decides, as before.
//!
//! The order of the accounts in the settings stays the priority: this only says which ones are
//! good, `accounts::pick_ranked` takes the first of the best. The figures both sides check are in
//! tests/fixtures/quota-pace.json.

use crate::board;
use crate::model::{QuotaMeter, RateWindow};
use chrono::{DateTime, Months};

/// The longest a window is a short pause: a day.
pub const SHORT_MINUTES: u32 = 24 * 60;

/// The use past which a long window is never good, on pace or not: an account so near its limit
/// goes behind the others (decided with the user at the review of PR #20).
pub const NEAR_LIMIT: u32 = 95;

const DAY_MS: i64 = 24 * 3_600_000;

/// How an account stands for the choice, the best first.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Standing {
    /// Every meter is good.
    Good,
    /// Only its pace is bad: a long window that would run out before its reset, none at its limit
    /// or past the threshold without a projection. After the good ones, before the others.
    Behind,
    /// A short window at the threshold, a long one at its limit (or at the threshold with no
    /// projection): out until that window starts again.
    Out,
}

/// What a meter says of its account.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeterVerdict {
    pub meter: QuotaMeter,
    pub standing: Standing,
    /// An agent running on the account leaves it: a short window at the threshold, or a long one
    /// at the threshold whose pace is not good (past 100 %, or not known).
    pub leave: bool,
    /// Where the meter would end at its reset at the pace so far, when it is known.
    pub projected: Option<f64>,
}

impl MeterVerdict {
    /// It is the pace that holds the meter back: a long window under its limit that would not last
    /// (not one held back for being near its limit, `NEAR_LIMIT`).
    pub fn by_pace(&self) -> bool {
        self.standing == Standing::Behind && self.projected.is_some_and(|p| p > 100.0)
    }
}

/// What an account's meters say of it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Verdict {
    /// The worst of its meters (good with none: a harness without a quota keeps the order alone).
    pub standing: Standing,
    /// One of its meters makes a running agent leave.
    pub leave: bool,
    /// The meter that makes it less than good (the worst, the most used of those): why new agents
    /// go elsewhere.
    pub holding: Option<MeterVerdict>,
    /// The meter that makes an agent leave (the most used): why a running agent goes elsewhere.
    pub leaving: Option<MeterVerdict>,
}

/// Whether a window is a short pause (`SHORT_MINUTES` or less); the month's credits (no length)
/// and the longer windows are judged by their pace.
pub fn is_short(minutes: Option<u32>) -> bool {
    minutes.is_some_and(|m| m <= SHORT_MINUTES)
}

/// The verdict on the account's meters at `now`, `threshold` being the percentage at which a short
/// window (or a long one without a projection) is out: the use past which an account takes no new
/// agent while another can (`board::pick_threshold`), or the one an agent leaves at. The only
/// place that judges an account's quota for a choice.
pub fn judge(meters: &[QuotaMeter], threshold: u32, now: i64) -> Verdict {
    let verdicts: Vec<MeterVerdict> = meters
        .iter()
        .map(|m| judge_meter(m, threshold, now))
        .collect();
    let most_used = |a: &&MeterVerdict, b: &&MeterVerdict| a.meter.pct.total_cmp(&b.meter.pct);
    let holding = verdicts
        .iter()
        .filter(|v| v.standing != Standing::Good)
        // The worst, and the most used of equals.
        .max_by(|a, b| a.standing.cmp(&b.standing).then_with(|| most_used(a, b)))
        .copied();
    let leaving = verdicts
        .iter()
        .filter(|v| v.leave)
        .max_by(most_used)
        .copied();
    Verdict {
        standing: verdicts
            .iter()
            .map(|v| v.standing)
            .max()
            .unwrap_or(Standing::Good),
        leave: leaving.is_some(),
        holding,
        leaving,
    }
}

/// What one meter says at `now`.
pub fn judge_meter(meter: &QuotaMeter, threshold: u32, now: i64) -> MeterVerdict {
    let rate = meter.rate();
    // Until its end (and the margin an agent's resume waits after it): a window whose end is
    // unknown or past holds nothing back.
    let at_threshold = board::over_threshold(&rate, threshold, now);
    let projected = projected_pct(meter.minutes, &rate, now);
    let (standing, leave) = if is_short(meter.minutes) {
        if at_threshold {
            (Standing::Out, true)
        } else {
            (Standing::Good, false)
        }
    } else if board::over_threshold(&rate, 100, now) {
        (Standing::Out, true)
    } else {
        let near = board::over_threshold(&rate, NEAR_LIMIT, now);
        match projected {
            Some(p) if p > 100.0 => (Standing::Behind, at_threshold),
            None if at_threshold => (Standing::Out, true),
            // On pace (or not known yet) but near its limit: behind the others, and an agent at the
            // threshold leaves.
            _ if near => (Standing::Behind, at_threshold),
            _ => (Standing::Good, false),
        }
    };
    MeterVerdict {
        meter: *meter,
        standing,
        leave,
        projected,
    }
}

/// Where a window would end if it went on at the pace used so far: its percentage spread over the
/// time gone, to its whole length. The month's credits (`minutes` none) are a window of the month
/// that ends at their reset. None for a window whose end is unknown or past, nor while it is too
/// early (`too_early`). As `projectedPct` of src/lib/accounts.ts.
pub fn projected_pct(minutes: Option<u32>, w: &RateWindow, now: i64) -> Option<f64> {
    let end = w.resets_at.filter(|end| *end != 0)?;
    let length = match minutes {
        Some(m) => i64::from(m) * 60_000,
        None => end - month_before(end),
    };
    let left = end - now;
    if left <= 0 {
        return None;
    }
    let gone = length.min(length - left);
    if too_early(gone, length) {
        return None;
    }
    Some(w.pct * length as f64 / gone as f64)
}

/// Too early to project, `gone` into a window of `length`. One of a day or so waits for a tenth of
/// itself (30 minutes of a 5 h window, a turn or two would take it far past 100 %; 2 h 24 of a
/// day); a longer one (a week's, a month's) for a whole day, for the nights to count: a morning's
/// work spread over a week as if it went on day and night would make the start of every week bad.
fn too_early(gone: i64, length: i64) -> bool {
    let wait = if length < 2 * DAY_MS {
        length as f64 * 0.1
    } else {
        DAY_MS as f64
    };
    (gone as f64) < wait
}

/// A projection as it is written: « 112 », « 999+ » past that (early in a window, a few turns go
/// far); as `fProjection` of src/lib/accounts.ts (which adds the %).
pub fn written(pct: f64) -> String {
    let pct = pct.round();
    if pct > 999.0 {
        "999+".into()
    } else {
        format!("{pct}")
    }
}

/// The same day and time a month before `ts`, in UTC (the credits come back on the 1st, 00:00
/// UTC); the last day of that month when it is shorter (March 31st gives February 28th, not
/// March 3rd).
fn month_before(ts: i64) -> i64 {
    DateTime::from_timestamp_millis(ts)
        .and_then(|d| d.checked_sub_months(Months::new(1)))
        .map_or(ts - 30 * DAY_MS, |d| d.timestamp_millis())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    /// The figures the window (src/lib/accounts.test.ts) checks too.
    const CASES: &str = include_str!("../../tests/fixtures/quota-pace.json");

    #[derive(Deserialize)]
    struct Cases {
        now: String,
        projection: Vec<Projection>,
        verdict: Vec<Scenario>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Projection {
        name: String,
        #[serde(flatten)]
        meter: Meter,
        now: String,
        expected: Option<f64>,
    }

    #[derive(Deserialize)]
    struct Scenario {
        name: String,
        threshold: u32,
        now: Option<String>,
        meters: Vec<Meter>,
        standing: String,
        leave: bool,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Meter {
        minutes: Option<u32>,
        pct: f64,
        resets_in_min: Option<i64>,
        resets_at: Option<String>,
    }

    fn at(iso: &str) -> i64 {
        DateTime::parse_from_rfc3339(iso)
            .unwrap()
            .timestamp_millis()
    }

    impl Meter {
        fn at(&self, now: i64) -> QuotaMeter {
            QuotaMeter {
                minutes: self.minutes,
                pct: self.pct,
                resets_at: match (self.resets_in_min, &self.resets_at) {
                    (Some(min), _) => Some(now + min * 60_000),
                    (None, Some(iso)) => Some(at(iso)),
                    (None, None) => None,
                },
            }
        }
    }

    fn cases() -> Cases {
        serde_json::from_str(CASES).unwrap()
    }

    #[test]
    fn the_projection_goes_by_the_figures_the_window_checks() {
        for c in cases().projection {
            let now = at(&c.now);
            let m = c.meter.at(now);
            let got = projected_pct(m.minutes, &m.rate(), now);
            match (got, c.expected) {
                (None, None) => {}
                (Some(g), Some(e)) => assert!((g - e).abs() < 0.005, "{}: {g} for {e}", c.name),
                (g, e) => panic!("{}: {g:?} for {e:?}", c.name),
            }
        }
    }

    #[test]
    fn the_verdict_goes_by_the_figures_the_window_checks() {
        let cases = cases();
        for s in cases.verdict {
            let now = at(s.now.as_deref().unwrap_or(&cases.now));
            let meters: Vec<QuotaMeter> = s.meters.iter().map(|m| m.at(now)).collect();
            let v = judge(&meters, s.threshold, now);
            let standing = match v.standing {
                Standing::Good => "good",
                Standing::Behind => "behind",
                Standing::Out => "out",
            };
            assert_eq!(standing, s.standing, "{}", s.name);
            assert_eq!(v.leave, s.leave, "{}", s.name);
        }
    }

    #[test]
    fn a_window_is_short_up_to_a_day_and_the_credits_are_long() {
        assert!(is_short(Some(300)));
        assert!(is_short(Some(1440)));
        assert!(!is_short(Some(1441)));
        assert!(!is_short(Some(10_080)));
        assert!(!is_short(None));
    }

    #[test]
    fn what_holds_an_account_back_is_the_worst_meter_and_what_makes_an_agent_leave_the_most_used() {
        let now = at("2026-10-30T12:00:00Z");
        let meter = |minutes, pct, min: i64| QuotaMeter {
            minutes: Some(minutes),
            pct,
            resets_at: Some(now + min * 60_000),
        };
        // A week behind (210 %), a 5 h at the threshold: it is the 5 h that holds the account back.
        let v = judge(&[meter(10_080, 60.0, 7200), meter(300, 96.0, 100)], 95, now);
        assert_eq!(v.standing, Standing::Out);
        assert_eq!(v.holding.map(|h| h.meter.minutes), Some(Some(300)));
        assert_eq!(v.leaving.map(|h| h.meter.minutes), Some(Some(300)));
        // The pace alone: the week holds it back, and says where it would end.
        let v = judge(&[meter(10_080, 60.0, 7200), meter(300, 10.0, 100)], 95, now);
        assert_eq!(v.standing, Standing::Behind);
        let held = v.holding.unwrap();
        assert!(held.by_pace());
        assert!((held.projected.unwrap() - 210.0).abs() < 1e-9);
        assert!(!v.leave && v.leaving.is_none());
    }
}
