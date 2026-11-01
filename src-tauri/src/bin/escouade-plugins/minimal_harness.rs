//! The harness of the demonstration plugin (`escouade-plugins minimal-harness`): the minimal fake engine of the
//! conformance bench (`engine/minimal/`) as a program of its own, which the core launches from the description the
//! plugin gives (INT-9). One JSON a line on its input (`{"say": text}`, `{"answer": {"request", "value"}}`,
//! `{"interrupt": true}`), one frame a line on its output, in the protocol `agent/minimal.rs` lists. Its message picks
//! the scenario as the in-process fake's does, with three more: « plantage » (its frames make the plugin crash once,
//! mid-turn), « plantage-toujours » (each time), « flux <n> » (n lines of text at once, each stamped, to measure), « rythme <n> » (the same, one every 5 ms).
//!
//! `--session <id>` resumes a session; `--once <prompt>` answers a one-shot question and exits.

use serde_json::{json, Value};
use std::io::{BufRead, Write};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

enum Input {
    Say(String),
    Answer { request: String, value: Value },
    Interrupt,
}

enum Cut {
    Interrupted,
    Closed,
}

struct Script {
    session: String,
    cwd: String,
    input: Receiver<Input>,
    n: u32,
}

pub fn run() -> i32 {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let value = |name: &str| {
        args.iter()
            .position(|a| a == name)
            .and_then(|i| args.get(i + 1))
            .cloned()
    };
    if let Some(prompt) = value("--once") {
        println!("Réponse courte : {prompt}");
        return 0;
    }
    let session = value("--session").unwrap_or_else(|| {
        let n = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or_default();
        format!("min-{n:x}")
    });
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            let Ok(v) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let input = if let Some(text) = v["say"].as_str() {
                Input::Say(text.to_string())
            } else if v["interrupt"] == true {
                Input::Interrupt
            } else if let Some(request) = v["answer"]["request"].as_str() {
                Input::Answer {
                    request: request.to_string(),
                    value: v["answer"]["value"].clone(),
                }
            } else {
                continue;
            };
            if tx.send(input).is_err() {
                break;
            }
        }
    });
    let cwd = std::env::current_dir()
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_default();
    Script {
        session,
        cwd,
        input: rx,
        n: 0,
    }
    .play();
    0
}

fn out(frame: Value) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{frame}");
    let _ = stdout.flush();
}

fn micros() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_micros())
        .unwrap_or_default()
}

impl Script {
    fn play(mut self) {
        while let Ok(input) = self.input.recv() {
            if let Input::Say(text) = input {
                match self.turn(&text) {
                    Ok(()) => {
                        out(json!({ "type": "end", "usage": { "input": 120, "output": 30 } }))
                    }
                    Err(Cut::Interrupted) => out(json!({ "type": "end", "interrupted": true })),
                    Err(Cut::Closed) => return,
                }
            }
        }
    }

    fn say(&self, text: &str) {
        out(json!({ "type": "say", "text": text }));
    }

    fn id(&mut self, prefix: &str) -> String {
        self.n += 1;
        format!("{prefix}-{}", self.n)
    }

    fn wait(&self, request: &str, within: Duration) -> Result<Option<Value>, Cut> {
        let deadline = Instant::now() + within;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.input.recv_timeout(left) {
                Ok(Input::Answer { request: r, value }) if r == request => return Ok(Some(value)),
                Ok(Input::Interrupt) => return Err(Cut::Interrupted),
                Ok(_) => {}
                Err(RecvTimeoutError::Timeout) => return Ok(None),
                Err(RecvTimeoutError::Disconnected) => return Err(Cut::Closed),
            }
        }
    }

    fn turn(&mut self, text: &str) -> Result<(), Cut> {
        out(json!({ "type": "start", "session": self.session }));
        if text.contains("raisonnement") {
            out(json!({ "type": "thinking", "text": "Deux pistes ; la seconde est la bonne." }));
            self.say("Réponse réfléchie.");
        } else if text.contains("commande") {
            let (call, request) = (self.id("call"), self.id("ask"));
            out(
                json!({ "type": "call", "id": call, "tool": "shell", "input": { "command": "rm -rf build" } }),
            );
            out(json!({ "type": "ask", "request": request, "call": call }));
            let answer = self
                .wait(&request, Duration::from_secs(60))?
                .unwrap_or_default();
            if answer["allow"] == true {
                out(json!({ "type": "done", "call": call, "output": "build supprimé" }));
                self.say("Commande exécutée.");
            } else {
                out(json!({ "type": "done", "call": call, "output": "refusée", "error": true }));
                self.say("Compris.");
            }
        } else if text.contains("fichier") {
            let call = self.id("call");
            out(
                json!({ "type": "call", "id": call, "tool": "patch", "input": { "path": "src/app.ts", "diff": "-const a = 1;\n+const a = 2;" } }),
            );
            out(json!({ "type": "done", "call": call, "output": "1 fichier modifié" }));
            self.say("Fichier modifié.");
        } else if text.contains("question") {
            let (call, request) = (self.id("call"), self.id("question"));
            out(json!({
                "type": "question", "request": request, "call": call,
                "text": "Quelle base de données ?", "choices": ["PostgreSQL", "SQLite"],
            }));
            let answers = self
                .wait(&request, Duration::from_secs(60))?
                .unwrap_or_default();
            let chosen: Vec<&str> = answers
                .as_object()
                .into_iter()
                .flat_map(|o| o.values().filter_map(Value::as_str))
                .collect();
            self.say(&format!("Choix retenu : {}", chosen.join(", ")));
        } else if text.contains("lent") {
            self.say("Je commence…");
            self.wait("", Duration::from_secs(30))?;
        } else if text.contains("sous-agent") {
            let call = self.id("call");
            out(
                json!({ "type": "call", "id": call, "tool": "agent", "input": { "task": "Chercher la cause du bug" } }),
            );
            out(
                json!({ "type": "say", "text": "Cause trouvée : le filtre refuse les PDF.", "parent": call }),
            );
            out(json!({ "type": "done", "call": call, "output": "Le filtre refuse les PDF." }));
            self.say("Rapport reçu.");
        } else if text.contains("plantage") {
            // A tool under way when the plugin crashes: the core closes it at the end of the turn.
            let call = self.id("call");
            out(
                json!({ "type": "call", "id": call, "tool": "shell", "input": { "command": "make" } }),
            );
            self.say("Avant le plantage.");
            let flag = std::path::Path::new(&self.cwd).join("plantage.flag");
            out(json!({ "type": "crash", "flag": flag, "always": text.contains("toujours") }));
            // Long enough for the plugin to start again: its frames wait in the core meanwhile.
            std::thread::sleep(Duration::from_millis(300));
            self.say("Après le plantage.");
        } else if let Some(n) = text
            .strip_prefix("rythme ")
            .and_then(|n| n.trim().parse::<u32>().ok())
        {
            // At a pace: a line every 5 ms, as a model writes.
            for _ in 0..n {
                self.say(&micros().to_string());
                std::thread::sleep(Duration::from_millis(5));
            }
        } else if let Some(n) = text
            .strip_prefix("flux ")
            .and_then(|n| n.trim().parse::<u32>().ok())
        {
            for _ in 0..n {
                self.say(&micros().to_string());
            }
        } else {
            self.say(&format!("Bonjour, tu as dit : {text}"));
        }
        Ok(())
    }
}
