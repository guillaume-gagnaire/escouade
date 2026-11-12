//! The output of a task Claude Code runs in the background (a command, a monitor): what the
//! window shows of it while it runs and after. Two sources, the same shape: the running process
//! (`get_task_output`, which answers the end of the output, 8 KiB at most) and the file Claude Code
//! writes the whole output to as it comes (`<task>.output`, in a folder named `tasks`), which a task
//! that is over is read from, further back (256 KiB). Never any other file, never a file of another
//! machine (`safe_origin`).

use serde::Serialize;
use serde_json::Value;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// The most bytes of the output of a task that runs one gives: its end. This is also all that
/// `get_task_output` answers.
pub const MAX_OUTPUT: usize = 8 * 1024;

/// The most bytes of the output of a task that is over one gives, read from its file: enough for
/// the first error of a failed build to be in it, still a size the window takes in one go.
pub const MAX_FINISHED_OUTPUT: usize = 256 * 1024;

pub use escouade_base::paths::safe_origin;

/// The end of a background task's output (`task_output`).
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskOutput {
    /// The end of the output, cut at a character: `MAX_OUTPUT` bytes at most from the process or,
    /// for a task that runs, from the file; `MAX_FINISHED_OUTPUT` for the file of a task that is over.
    pub output: String,
    /// The size of the whole output, in bytes.
    pub total_bytes: u64,
    /// The start of the output is not in `output`.
    pub truncated: bool,
    /// Where it was read: `"process"` (the running process), `"file"` (its output file) or
    /// `"none"` (there is nothing to read: a task with no output, no file, no process).
    pub source: String,
}

impl TaskOutput {
    /// There is nothing to read.
    pub fn none() -> Self {
        Self {
            output: String::new(),
            total_bytes: 0,
            truncated: false,
            source: "none".into(),
        }
    }

    /// What Claude Code answers to `get_task_output`: `{ output, total_bytes, truncated }`. None
    /// when it does not look like that (the file is then the way to read it).
    pub fn from_process(answer: &Value) -> Option<Self> {
        let whole = answer["output"].as_str()?;
        let (output, cut) = tail(whole.as_bytes());
        Some(Self {
            total_bytes: answer["total_bytes"].as_u64().unwrap_or(whole.len() as u64),
            truncated: answer["truncated"].as_bool().unwrap_or(false) || cut,
            output,
            source: "process".into(),
        })
    }

    /// The end of the file `path`, `max` bytes at most, if it is the output file of `task`: named
    /// `<task>.output` in a folder named `tasks` (where Claude Code writes it:
    /// `<…>/<session>/tasks/<task>.output`) and an ordinary file (not a link, not a folder, nothing
    /// the system has to wait for). Anything else, or a file that cannot be read, is nothing.
    ///
    /// Whether the path leads to another machine is not asked here (`safe_origin`, of the path as
    /// the frames said it): the path of a `claude` under WSL arrives converted, to a share of ours.
    pub fn from_file(path: &Path, task: &str, max: usize) -> Self {
        Self::read_file(path, task, max).unwrap_or_else(Self::none)
    }

    fn read_file(path: &Path, task: &str, max: usize) -> Option<Self> {
        let name = format!("{task}.output");
        if path.file_name()?.to_str()? != name {
            return None;
        }
        if path.parent()?.file_name()?.to_str()? != "tasks" {
            return None;
        }
        if !std::fs::symlink_metadata(path).ok()?.file_type().is_file() {
            return None;
        }
        let mut file = std::fs::File::open(path).ok()?;
        let meta = file.metadata().ok()?;
        if !meta.is_file() {
            return None;
        }
        let len = meta.len();
        let start = len.saturating_sub(max as u64);
        file.seek(SeekFrom::Start(start)).ok()?;
        let mut bytes = Vec::new();
        file.take(max as u64).read_to_end(&mut bytes).ok()?;
        // Cut in the middle of a character: it starts at the next one.
        let skip = if start > 0 {
            bytes.iter().take_while(|b| (**b & 0xC0) == 0x80).count()
        } else {
            0
        };
        Some(Self {
            output: String::from_utf8_lossy(&bytes[skip..]).into_owned(),
            total_bytes: len,
            truncated: start > 0,
            source: "file".into(),
        })
    }
}

/// The last `MAX_OUTPUT` bytes of `bytes` as text, from a character's start; true when the start
/// was left out.
fn tail(bytes: &[u8]) -> (String, bool) {
    if bytes.len() <= MAX_OUTPUT {
        return (String::from_utf8_lossy(bytes).into_owned(), false);
    }
    let cut = &bytes[bytes.len() - MAX_OUTPUT..];
    let skip = cut.iter().take_while(|b| (**b & 0xC0) == 0x80).count();
    (String::from_utf8_lossy(&cut[skip..]).into_owned(), true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn the_answer_of_claude_code_is_the_end_of_the_output_as_it_says() {
        let out = TaskOutput::from_process(
            &json!({ "output": "tick 1\n", "total_bytes": 7, "truncated": false }),
        )
        .unwrap();
        assert_eq!(
            (
                out.output.as_str(),
                out.total_bytes,
                out.truncated,
                out.source.as_str()
            ),
            ("tick 1\n", 7, false, "process")
        );
        // The start left out, and a size it counted itself.
        let out = TaskOutput::from_process(
            &json!({ "output": "end\n", "total_bytes": 90_000, "truncated": true }),
        )
        .unwrap();
        assert_eq!((out.total_bytes, out.truncated), (90_000, true));
        // Missing fields are what the text says.
        let out = TaskOutput::from_process(&json!({ "output": "é" })).unwrap();
        assert_eq!((out.total_bytes, out.truncated), (2, false));
        // Not an answer of that shape: no output.
        assert_eq!(TaskOutput::from_process(&json!({})), None);
        assert_eq!(TaskOutput::from_process(&json!({ "output": 3 })), None);
        assert_eq!(TaskOutput::from_process(&Value::Null), None);
    }

    #[test]
    fn an_answer_longer_than_8_kib_is_cut_to_its_end_at_a_character() {
        let long = format!("{}x", "é".repeat(20_000));
        let out = TaskOutput::from_process(&json!({ "output": long })).unwrap();
        assert!(out.truncated && out.output.len() == 8_191);
        assert!(out.output.starts_with('é') && out.output.ends_with("éx"));
        assert_eq!(out.total_bytes, 40_001);
    }

    /// The task's file in a folder named `tasks`, as Claude Code writes it.
    fn task_file(test: &str, task: &str, content: &[u8]) -> std::path::PathBuf {
        let dir = crate::paths::test_dir(test).join("claude").join("tasks");
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(format!("{task}.output"));
        std::fs::write(&file, content).unwrap();
        file
    }

    #[test]
    fn a_file_is_read_only_in_a_folder_named_tasks_and_named_after_its_task() {
        let file = task_file("taskout-folder", "b1", b"out\n");
        let out = TaskOutput::from_file(&file, "b1", MAX_OUTPUT);
        assert_eq!(
            (out.source.as_str(), out.output.as_str(), out.total_bytes),
            ("file", "out\n", 4)
        );
        // The same file anywhere else is nothing: another folder, a deeper one, the folder's name
        // in another case, none at all.
        let root = file.parent().unwrap().parent().unwrap().to_path_buf();
        for folder in ["task", "taskz", "Tasks", "tmp", "tasks.d"] {
            let dir = root.join(folder);
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join("b1.output"), "never").unwrap();
            let out = TaskOutput::from_file(&dir.join("b1.output"), "b1", MAX_OUTPUT);
            assert_eq!(out, TaskOutput::none(), "{folder}");
        }
        let deeper = root.join("tasks").join("sub");
        std::fs::create_dir_all(&deeper).unwrap();
        std::fs::write(deeper.join("b1.output"), "never").unwrap();
        assert_eq!(
            TaskOutput::from_file(&deeper.join("b1.output"), "b1", MAX_OUTPUT),
            TaskOutput::none()
        );
        // A file with no folder: a bare name.
        assert_eq!(
            TaskOutput::from_file(Path::new("b1.output"), "b1", MAX_OUTPUT),
            TaskOutput::none()
        );
        // Not the task's name, in the right folder.
        std::fs::write(file.with_file_name("other.output"), "never").unwrap();
        for name in ["other.output", "b1.output.txt", "b1.txt"] {
            let out = TaskOutput::from_file(&file.with_file_name(name), "b1", MAX_OUTPUT);
            assert_eq!(out, TaskOutput::none(), "{name}");
        }
        // A folder named like the file is no file.
        std::fs::create_dir_all(file.with_file_name("d1.output")).unwrap();
        assert_eq!(
            TaskOutput::from_file(&file.with_file_name("d1.output"), "d1", MAX_OUTPUT),
            TaskOutput::none()
        );
    }

    #[test]
    fn a_file_is_read_by_its_end_up_to_the_bound_it_is_given_cut_at_a_character() {
        // 300 000 bytes of two- and three-byte characters.
        let text: String = "é€".repeat(60_000);
        assert_eq!(text.len(), 300_000);
        let file = task_file("taskout-bound", "b1", text.as_bytes());
        let finished = TaskOutput::from_file(&file, "b1", MAX_FINISHED_OUTPUT);
        assert_eq!(
            (
                finished.source.as_str(),
                finished.total_bytes,
                finished.truncated
            ),
            ("file", 300_000, true)
        );
        // 262 144 bytes, less the character the cut falls in: it starts at the next one.
        assert!(finished.output.len() <= MAX_FINISHED_OUTPUT);
        assert!(finished.output.len() > MAX_FINISHED_OUTPUT - 3);
        assert!(!finished.output.contains('\u{FFFD}'));
        assert!(text.ends_with(&finished.output));
        assert!(finished.output.starts_with(['é', '€']));
        // The live bound is 8 KiB, the same way.
        let live = TaskOutput::from_file(&file, "b1", MAX_OUTPUT);
        assert!(live.truncated && live.total_bytes == 300_000);
        assert!(live.output.len() <= MAX_OUTPUT && live.output.len() > MAX_OUTPUT - 3);
        assert!(text.ends_with(&live.output) && !live.output.contains('\u{FFFD}'));
        // Exactly the bound is whole; a byte over is not.
        let exact = task_file(
            "taskout-bound-exact",
            "b2",
            &vec![b'a'; MAX_FINISHED_OUTPUT],
        );
        let out = TaskOutput::from_file(&exact, "b2", MAX_FINISHED_OUTPUT);
        assert_eq!(
            (out.output.len(), out.total_bytes, out.truncated),
            (MAX_FINISHED_OUTPUT, MAX_FINISHED_OUTPUT as u64, false)
        );
        let over = task_file(
            "taskout-bound-over",
            "b3",
            &vec![b'a'; MAX_FINISHED_OUTPUT + 1],
        );
        let out = TaskOutput::from_file(&over, "b3", MAX_FINISHED_OUTPUT);
        assert_eq!(
            (out.output.len(), out.total_bytes, out.truncated),
            (MAX_FINISHED_OUTPUT, MAX_FINISHED_OUTPUT as u64 + 1, true)
        );
        // 100 KiB is whole under the bound of a finished task, and not under the live one.
        let hundred = task_file("taskout-bound-100", "b4", &vec![b'a'; 100 * 1024]);
        let out = TaskOutput::from_file(&hundred, "b4", MAX_FINISHED_OUTPUT);
        assert_eq!((out.output.len(), out.truncated), (100 * 1024, false));
        let out = TaskOutput::from_file(&hundred, "b4", MAX_OUTPUT);
        assert_eq!((out.output.len(), out.truncated), (MAX_OUTPUT, true));
    }

    #[test]
    fn the_bounds_are_8_kib_for_the_live_output_and_256_kib_for_a_finished_task() {
        assert_eq!(MAX_OUTPUT, 8 * 1024);
        assert_eq!(MAX_FINISHED_OUTPUT, 256 * 1024);
    }
}
