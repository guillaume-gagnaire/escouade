//! `escouade-audit`: `verify` checks an export of the team's audit (signatures, chains, checkpoints: `docs/TEAM-AUDIT.md`
//! §5.4), without the network; `events` lists the closed events and their severity; `never` says what never goes out.

use std::io::Read as _;
use std::process::ExitCode;

use escouade_audit::verify::{self, Keys, Place};
use escouade_audit::{never, Fact};

const USAGE: &str = "usage:
  escouade-audit verify [--keys KEYS.json] [--checkpoints CHECKPOINTS.jsonl] EXPORT.jsonl...   (- reads the standard input)
  escouade-audit events
  escouade-audit never

verify answers 0 when the export is whole, 1 when it found a problem, 2 when it could not read what it was given.";

fn read(path: &str) -> Result<String, String> {
    if path == "-" {
        let mut s = String::new();
        std::io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| format!("the standard input: {e}"))?;
        return Ok(s);
    }
    std::fs::read_to_string(path).map_err(|e| format!("{path}: {e}"))
}

fn lines(path: &str) -> Result<Vec<(Place, String)>, String> {
    Ok(read(path)?
        .lines()
        .enumerate()
        .map(|(i, l)| {
            (
                Place {
                    file: path.to_owned(),
                    line: i + 1,
                },
                l.to_owned(),
            )
        })
        .collect())
}

fn run_verify(args: &[String]) -> Result<bool, String> {
    let mut keys = None;
    let mut checkpoints = Vec::new();
    let mut exports = Vec::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--keys" => {
                let path = args.get(i + 1).ok_or("--keys: a file")?;
                keys = Some(Keys::parse(&read(path)?).map_err(|e| format!("{path}: {e}"))?);
                i += 2;
            }
            "--checkpoints" => {
                let path = args.get(i + 1).ok_or("--checkpoints: a file")?;
                checkpoints.extend(lines(path)?);
                i += 2;
            }
            other if other.starts_with("--") && other != "-" => {
                return Err(format!("unknown option {other}"))
            }
            path => {
                exports.extend(lines(path)?);
                i += 1;
            }
        }
    }
    if exports.is_empty() {
        return Err("an export to verify".to_owned());
    }
    if keys.is_none() {
        eprintln!("no --keys: the signatures are not checked, the chains are");
    }
    let report = verify::verify(exports, keys.as_ref(), checkpoints);
    println!("{report}");
    Ok(report.ok())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("verify") => match run_verify(&args[1..]) {
            Ok(true) => ExitCode::SUCCESS,
            Ok(false) => ExitCode::from(1),
            Err(e) => {
                eprintln!("escouade-audit verify: {e}\n\n{USAGE}");
                ExitCode::from(2)
            }
        },
        Some("events") => {
            for name in Fact::NAMES {
                println!("{name}\t{}", Fact::severity_of(name).unwrap_or(0));
            }
            ExitCode::SUCCESS
        }
        Some("never") => {
            for line in never::NEVER {
                println!("- {line}");
            }
            ExitCode::SUCCESS
        }
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}
