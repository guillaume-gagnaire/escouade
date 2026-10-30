use super::super::*;
use super::*;
use crate::i18n::Lang;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// What the fake harness answers.
#[derive(Clone, Copy)]
enum Answers {
    /// Never: its question waits forever.
    Never,
    /// `n` bytes.
    Bytes(usize),
}

/// A harness that is not Claude Code: only its one-shot questions are written.
struct Fake {
    answers: Answers,
    /// Set when the question it was asked is dropped (its process, were it one, killed).
    dropped: Arc<AtomicBool>,
}

impl Fake {
    fn new(answers: Answers) -> Self {
        Fake {
            answers,
            dropped: Arc::new(AtomicBool::new(false)),
        }
    }
}

/// Sets its flag once dropped.
struct OnDrop(Arc<AtomicBool>);

impl Drop for OnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::SeqCst);
    }
}

impl Engine for Fake {
    fn id(&self) -> EngineId {
        EngineId::CODEX
    }
    fn name(&self) -> &'static str {
        "Faux"
    }
    fn speaker(&self) -> &'static str {
        "Faux"
    }
    fn accounts(&self) -> registry::AccountStore {
        registry::AccountStore::ConfigDir { var: "FAUX_HOME" }
    }
    fn terminal(&self) -> Option<&'static dyn crate::engine::TerminalView> {
        None
    }
    fn instructions_room(&self) -> Option<Room> {
        None
    }
    fn subagent_relay(&self, _: Lang, _: &str, _: &str, _: &str) -> Option<String> {
        None
    }
    fn locate(&self, _: &str, _: &str) -> Option<Program> {
        None
    }
    fn tested(&self) -> registry::Tested {
        registry::Tested {
            oldest: "1.0",
            newest: "1.0",
        }
    }
    fn oldest_reason(&self, _: Lang) -> String {
        String::new()
    }
    fn update_command(&self) -> &'static str {
        ""
    }
    fn version_args(&self) -> &'static [&'static str] {
        &[]
    }
    fn caps(&self) -> registry::EngineCaps {
        unimplemented!()
    }
    fn install(&self) -> registry::Install {
        unimplemented!()
    }
    fn models(&self) -> Vec<registry::ModelDecl> {
        Vec::new()
    }
    fn efforts(&self) -> &'static [&'static str] {
        &[]
    }
    fn supports_effort(&self, _: &str) -> bool {
        false
    }
    fn supports_auto(&self, _: &str) -> bool {
        false
    }
    fn spawn(&self, _: Spawn, _: OnFrame, _: OnExit) -> Result<Process> {
        unimplemented!()
    }
    fn message(&self, _: &str, _: &[Attachment]) -> Result<Message> {
        unimplemented!()
    }
    fn translate(&self, _: &mut AgentRt, _: &Value, _: &mut Effects) {}
    fn start_lost(&self, _: &str) -> Option<StartLost> {
        None
    }
    fn one_shot<'a>(&'a self, _: OneShot<'a>) -> BoxFuture<'a, Result<String>> {
        let guard = OnDrop(self.dropped.clone());
        let answers = self.answers;
        Box::pin(async move {
            let _guard = guard;
            match answers {
                Answers::Never => std::future::pending().await,
                Answers::Bytes(n) => Ok("x".repeat(n)),
            }
        })
    }
    fn writer(&self, _: Weight) -> &'static str {
        "Faux mini"
    }
    fn grant_file(&self, _: &str, _: &str) -> Option<Vec<u8>> {
        None
    }
    fn refusal(&self) -> Option<Value> {
        None
    }
    fn escouade_launch(&self, _: &Escouade) -> mcp::Launch {
        unimplemented!()
    }
    fn declared(&self, _: &Path) -> Standing {
        unimplemented!()
    }
    fn add_declaration<'a>(
        &'a self,
        _: &'a Target,
        _: &'a Entry,
        _: &'a [&'a str],
    ) -> BoxFuture<'a, Result<()>> {
        unimplemented!()
    }
    fn remove_declaration<'a>(
        &'a self,
        _: &'a Target,
        _: &'a [&'a str],
    ) -> BoxFuture<'a, Result<()>> {
        unimplemented!()
    }
    fn manual_declaration(&self, _: bool, _: &str, _: Option<&str>, _: &Entry, _: bool) -> String {
        String::new()
    }
}

fn question(limit: Duration) -> OneShot<'static> {
    OneShot {
        program: Program::Native("faux".into()),
        env: Vec::new(),
        weight: Weight::Light,
        cwd: Path::new("."),
        system: "Réponds par un nom.",
        prompt: "Corriger le bug",
        limit,
        model: String::new(),
    }
}

#[test]
fn the_texts_go_to_the_harness_chosen_while_it_is_found_else_to_claude_haiku_else_to_one_found() {
    // Nothing chosen: Claude Code when it is found, wherever it is listed.
    assert_eq!(
        writer(None, &[EngineId::CLAUDE, EngineId::CODEX]),
        Some(EngineId::CLAUDE)
    );
    assert_eq!(
        writer(None, &[EngineId::CODEX, EngineId::CLAUDE]),
        Some(EngineId::CLAUDE)
    );
    // Chosen and found.
    assert_eq!(
        writer(Some(EngineId::CODEX), &[EngineId::CLAUDE, EngineId::CODEX]),
        Some(EngineId::CODEX)
    );
    // Chosen but gone: as if nothing was.
    assert_eq!(
        writer(Some(EngineId::CODEX), &[EngineId::CLAUDE]),
        Some(EngineId::CLAUDE)
    );
    // No Claude Code: the first harness found.
    assert_eq!(
        writer(None, &[EngineId::CODEX, EngineId::OPENCODE]),
        Some(EngineId::CODEX)
    );
    assert_eq!(
        writer(
            Some(EngineId::CURSOR),
            &[EngineId::CODEX, EngineId::OPENCODE]
        ),
        Some(EngineId::CODEX)
    );
    // None found.
    assert_eq!(writer(Some(EngineId::CLAUDE), &[]), None);
}

#[tokio::test]
async fn a_harness_that_never_answers_is_dropped_at_its_limit_90_seconds_by_default() {
    assert_eq!(LIMIT, Duration::from_secs(90));
    let fake = Fake::new(Answers::Never);
    let err = ask(&fake, question(Duration::from_secs(1)))
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("Faux mini") && err.contains(" 1 s"), "{err}");
    // Its question is dropped: a process of its would be killed with all it started.
    assert!(fake.dropped.load(Ordering::SeqCst));
}

#[tokio::test]
async fn the_limit_holds_for_any_harness_and_its_answer_is_bounded() {
    let fake = Fake::new(Answers::Never);
    let start = Instant::now();
    assert!(ask(&fake, question(Duration::from_millis(200)))
        .await
        .is_err());
    assert!(start.elapsed() < Duration::from_secs(10));
    assert!(fake.dropped.load(Ordering::SeqCst));
    // An answer up to the bound is taken, one past it refused.
    let fits = Fake::new(Answers::Bytes(ANSWER_MAX));
    assert_eq!(ask(&fits, question(LIMIT)).await.unwrap().len(), ANSWER_MAX);
    let over = Fake::new(Answers::Bytes(ANSWER_MAX + 1));
    let err = ask(&over, question(LIMIT)).await.unwrap_err().to_string();
    assert!(err.contains("Faux mini"), "{err}");
}

#[tokio::test]
async fn a_reading_stops_at_its_bound() {
    // Endless: never read to its end.
    assert_eq!(
        read_bounded(tokio::io::repeat(b'x'), 1000).await.unwrap(),
        None
    );
    let exact = read_bounded(&b"abc"[..], 3).await.unwrap();
    assert_eq!(exact.as_deref(), Some(&b"abc"[..]));
    assert_eq!(read_bounded(&b"abcd"[..], 3).await.unwrap(), None);
}

/// A program of the system and its arguments that run `script` (cmd on Windows, sh elsewhere).
fn shell(script: &str) -> (Program, Vec<String>) {
    if cfg!(windows) {
        let args = ["/d", "/c", script].map(String::from).to_vec();
        (Program::Native("cmd.exe".into()), args)
    } else {
        let args = ["-c", script].map(String::from).to_vec();
        (Program::Native("/bin/sh".into()), args)
    }
}

#[tokio::test]
async fn a_process_that_says_too_much_is_killed_and_one_that_never_reads_its_question_still_answers(
) {
    let cwd = std::env::temp_dir();
    // Endless output: killed past the bound, refused.
    let endless = if cfg!(windows) {
        "for /l %i in (0,0,1) do @echo xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
    } else {
        "yes xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
    };
    let (program, args) = shell(endless);
    let endless = run(&program, &cwd, &args, &[], "", "Faux mini");
    let err = tokio::time::timeout(Duration::from_secs(60), endless)
        .await
        .expect("killed once past the bound")
        .unwrap_err()
        .to_string();
    assert!(err.contains("Faux mini"), "{err}");
    // A question bigger than a pipe holds, never read: the answer comes all the same.
    let (program, args) = shell("echo ok");
    let big = "x".repeat(4 << 20);
    let out = tokio::time::timeout(
        Duration::from_secs(60),
        run(&program, &cwd, &args, &[], &big, "Faux mini"),
    )
    .await
    .expect("not held by its question")
    .unwrap();
    assert_eq!(String::from_utf8_lossy(&out).trim(), "ok");
}
