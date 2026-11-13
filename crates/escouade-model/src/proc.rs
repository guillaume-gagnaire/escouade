//! Running a child process and collecting what it prints, without depending on the end of its
//! pipes.
//!
//! A pipe only ends once every process holding its write end has closed it. On macOS, std makes
//! its pipes with `pipe()` then `FIOCLEX`, two steps with no `pipe2`: a process started at that
//! very moment (the fake `claude` of a test, a terminal, a dev server) inherits the write end of
//! another command's pipe, and waiting for the end of that pipe lasts as long as it lives.

use std::io;
use std::process::{ExitStatus, Output, Stdio};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::process::{Child, Command};
use tokio::time::{sleep_until, timeout, Instant};

/// The creation flag that starts a console program of Windows without a window of its own.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Once the process has ended, how long its pipes may stay silent before what was read is taken
/// as the whole output: a program it started and left running can hold them open.
const EOF_GRACE: Duration = Duration::from_secs(2);

/// Runs `cmd` (nothing to read on its input, its two outputs in pipes) and gives what it printed.
/// Past `limit` it is killed and the error is `TimedOut`. A pipe a descendant keeps open after the
/// process ended does not hold it back (see `wait_output`).
pub async fn output_within(mut cmd: Command, limit: Duration) -> io::Result<Output> {
    cmd.stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    let child = cmd.spawn()?;
    // The timeout drops the future and with it the child, which `kill_on_drop` kills.
    match timeout(limit, wait_output(child)).await {
        Ok(out) => out,
        Err(_) => Err(io::Error::new(
            io::ErrorKind::TimedOut,
            format!("still not done after {} s", limit.as_secs()),
        )),
    }
}

/// How many descriptors `no_inherited_fds` goes through at most: the soft limit can be absurd (a
/// container's is often a billion), and std's pipes are the lowest numbers free anyway.
#[cfg(unix)]
const MAX_FDS_MARKED: libc::c_int = 1 << 16;

/// Makes the process `cmd` starts leave behind the descriptors the app has open (every one but its
/// standard three): a long-lived child (`claude`, a test run) would otherwise hold on to the write
/// end of the pipe of a `git` that another thread is running at that moment, and the end of that
/// pipe would wait for it. Nothing to do outside Unix: a handle is not inherited there unless it was
/// made to be. The descriptors are marked close-on-exec, not closed: std tells its parent that
/// `exec` failed through a pipe of its own, which is only valid if it stays open until then.
pub fn no_inherited_fds(cmd: &mut Command) {
    #[cfg(unix)]
    {
        // Read here rather than in the closure, which runs between `fork` and `exec` and so may
        // only call what is async-signal-safe.
        let limit = unsafe { libc::getdtablesize() }.clamp(3, MAX_FDS_MARKED);
        // SAFETY: the closure runs in the child between `fork` and `exec`, where nothing may
        // allocate or take a lock. It only calls `fcntl` (a bare system call, async-signal-safe) on
        // integers, on a range computed beforehand, and ignores its failure (`EBADF`: a number
        // that is not open).
        unsafe {
            cmd.pre_exec(move || {
                for fd in 3..limit {
                    libc::fcntl(fd, libc::F_SETFD, libc::FD_CLOEXEC);
                }
                Ok(())
            });
        }
    }
    #[cfg(not(unix))]
    let _ = cmd;
}

/// What `child` (its outputs in pipes) prints until it ends, like tokio's `wait_with_output`, but
/// without waiting for the end of the pipes once it has: they are then given `EOF_GRACE` to end,
/// counted from the last data they delivered, so that all of a big output is read, and a pipe
/// that stays open without a word is let go.
pub async fn wait_output(mut child: Child) -> io::Result<Output> {
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let (mut out, mut err) = (Vec::new(), Vec::new());
    // Once the process has ended: its status, and when the pipes are given up on if they are silent.
    let mut ended: Option<(ExitStatus, Instant)> = None;
    let status = loop {
        let give_up = match ended {
            Some((status, _)) if stdout.is_none() && stderr.is_none() => break status,
            Some((_, give_up)) => Some(give_up),
            None => None,
        };
        // Biased: what the pipes still hold is read before they are given up on.
        tokio::select! {
            biased;
            status = child.wait(), if ended.is_none() => {
                ended = Some((status?, Instant::now() + EOF_GRACE));
            }
            n = fill(&mut stdout, &mut out), if stdout.is_some() => {
                if n == 0 {
                    stdout = None;
                } else {
                    renew(&mut ended);
                }
            }
            n = fill(&mut stderr, &mut err), if stderr.is_some() => {
                if n == 0 {
                    stderr = None;
                } else {
                    renew(&mut ended);
                }
            }
            // The expression is evaluated even with the branch disabled: no `unwrap` here.
            _ = sleep_until(give_up.unwrap_or_else(Instant::now)), if give_up.is_some() => {
                if let Some((status, _)) = ended {
                    break status;
                }
            }
        }
    };
    Ok(Output {
        status,
        stdout: out,
        stderr: err,
    })
}

/// `cmd.output()` for the tests, with stdout and stderr in temporary files instead of pipes: a
/// file has no end to wait for, so a program started at the same moment (a fake `claude`) that
/// inherits it cannot hold `cmd` back.
#[cfg(any(test, feature = "testing"))]
pub fn output_in_files(cmd: &mut std::process::Command) -> io::Result<Output> {
    use std::sync::atomic::{AtomicU32, Ordering};
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let base = format!(
        "ccm-output-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    );
    let (out_file, err_file) = (
        std::env::temp_dir().join(format!("{base}.out")),
        std::env::temp_dir().join(format!("{base}.err")),
    );
    let mut run = || -> io::Result<Output> {
        let status = cmd
            .stdin(Stdio::null())
            .stdout(std::fs::File::create(&out_file)?)
            .stderr(std::fs::File::create(&err_file)?)
            .status()?;
        Ok(Output {
            status,
            stdout: std::fs::read(&out_file)?,
            stderr: std::fs::read(&err_file)?,
        })
    };
    let out = run();
    let _ = std::fs::remove_file(&out_file);
    let _ = std::fs::remove_file(&err_file);
    out
}

/// Appends what `pipe` delivers to `buf`: how many bytes, 0 at its end (or if it fails).
async fn fill<R: AsyncRead + Unpin>(pipe: &mut Option<R>, buf: &mut Vec<u8>) -> usize {
    match pipe {
        Some(p) => p.read_buf(buf).await.unwrap_or(0),
        None => 0,
    }
}

/// Data came after the end of the process: the pipes have a new `EOF_GRACE`.
fn renew(ended: &mut Option<(ExitStatus, Instant)>) {
    if let Some((_, give_up)) = ended {
        *give_up = Instant::now() + EOF_GRACE;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use std::time::Instant;

    fn command(program: &str, args: &[&str]) -> Command {
        let mut cmd = Command::new(program);
        cmd.args(args);
        cmd
    }

    /// Prints `out`, then ends and leaves a program running for 30 s, which holds the pipe open.
    fn leaves_a_grandchild() -> Command {
        #[cfg(unix)]
        return command("sh", &["-c", "echo out; sleep 30 & exit 0"]);
        // Not `ping`: the terminals' tests count the machine's `ping` processes.
        #[cfg(windows)]
        return command(
            "cmd",
            &[
                "/c",
                "echo out& start /b powershell -NoProfile -Command Start-Sleep -Seconds 30",
            ],
        );
    }

    /// Does nothing for 30 s.
    fn lasts_30_seconds() -> Command {
        #[cfg(unix)]
        return command("sleep", &["30"]);
        #[cfg(windows)]
        return command(
            "powershell",
            &["-NoProfile", "-Command", "Start-Sleep -Seconds 30"],
        );
    }

    /// Prints the file.
    fn prints(file: &Path) -> Command {
        let file = file.to_string_lossy();
        #[cfg(unix)]
        return command("cat", &[&file]);
        #[cfg(windows)]
        return command("cmd", &["/c", "type", &file]);
    }

    #[test]
    fn a_pipe_a_grandchild_keeps_open_does_not_hold_the_output_back() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let start = Instant::now();
        let out = rt
            .block_on(output_within(
                leaves_a_grandchild(),
                Duration::from_secs(60),
            ))
            .unwrap();
        let elapsed = start.elapsed();
        // On Windows tokio reads a pipe from a thread of the runtime, stuck until the grandchild
        // lets go of the pipe: the runtime must not wait for it as it goes.
        rt.shutdown_background();
        assert!(out.status.success());
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "out");
        assert!(elapsed < Duration::from_secs(10), "{elapsed:?}");
    }

    #[tokio::test]
    async fn a_command_past_its_limit_is_killed_and_reported_as_timed_out() {
        let start = Instant::now();
        let err = output_within(lasts_30_seconds(), Duration::from_secs(1))
            .await
            .unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::TimedOut, "{err}");
        assert!(
            start.elapsed() < Duration::from_secs(5),
            "{:?}",
            start.elapsed()
        );
    }

    #[tokio::test]
    async fn five_megabytes_come_back_whole() {
        let dir = escouade_base::testing::test_dir("proc-five-megabytes");
        let file = dir.join("big.txt");
        let text: String = (0..580_000).map(|i| format!("{i:08}\n")).collect();
        assert!(text.len() > 5_000_000);
        std::fs::write(&file, &text).unwrap();
        let out = output_within(prints(&file), Duration::from_secs(60))
            .await
            .unwrap();
        assert!(out.status.success());
        assert!(out.stderr.is_empty());
        assert_eq!(out.stdout.len(), text.len());
        assert!(out.stdout == text.as_bytes(), "the output differs");
    }

    #[test]
    fn the_tests_commands_read_their_outputs_from_files() {
        #[cfg(unix)]
        let mut cmd = std::process::Command::new("sh");
        #[cfg(unix)]
        cmd.args(["-c", "echo out; echo err >&2; exit 3"]);
        #[cfg(windows)]
        let mut cmd = std::process::Command::new("cmd");
        #[cfg(windows)]
        cmd.args(["/c", "echo out& echo err 1>&2& exit 3"]);
        let out = output_in_files(&mut cmd).unwrap();
        assert_eq!(out.status.code(), Some(3));
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "out");
        assert_eq!(String::from_utf8_lossy(&out.stderr).trim(), "err");
    }

    #[tokio::test]
    async fn the_exit_code_and_both_outputs_are_kept_apart() {
        #[cfg(unix)]
        let cmd = command("sh", &["-c", "echo out; echo err >&2; exit 3"]);
        #[cfg(windows)]
        let cmd = command("cmd", &["/c", "echo out& echo err 1>&2& exit 3"]);
        let out = output_within(cmd, Duration::from_secs(60)).await.unwrap();
        assert_eq!(out.status.code(), Some(3));
        assert_eq!(String::from_utf8_lossy(&out.stdout).trim(), "out");
        assert_eq!(String::from_utf8_lossy(&out.stderr).trim(), "err");
    }

    /// A pipe made the way std makes its own before it marks them close-on-exec: both ends are
    /// inherited by whatever is started while they are open. Returns `(read, write)`.
    #[cfg(unix)]
    fn inheritable_pipe() -> (libc::c_int, libc::c_int) {
        let mut fds = [0 as libc::c_int; 2];
        assert_eq!(unsafe { libc::pipe(fds.as_mut_ptr()) }, 0);
        (fds[0], fds[1])
    }

    /// Set in the copy of the test binary that runs the test below alone.
    #[cfg(unix)]
    const ALONE: &str = "CCM_PROC_TEST_ALONE";

    /// A long-lived child must not keep what the app has open when it is started: with a pipe's
    /// write end inherited, the app reading the pipe would not see its end until the child dies.
    ///
    /// The pipe is open (and inheritable) for the few milliseconds between its creation and the
    /// close of our write end: any test of the process starting a long-lived program in that
    /// window holds it too and the read waits for that one. So the test runs again alone, in a
    /// copy of the test binary where nothing else is started (as nextest does for every test).
    #[cfg(unix)]
    #[test]
    fn a_long_lived_child_does_not_inherit_the_open_descriptors() {
        const NAME: &str = "a_long_lived_child_does_not_inherit_the_open_descriptors";
        if std::env::var_os(ALONE).is_none() {
            // The test's name from the root of the crate: `proc::tests::…`.
            let path = module_path!().split_once("::").map_or("", |(_, rest)| rest);
            let mut again = std::process::Command::new(std::env::current_exe().unwrap());
            again
                .args([&format!("{path}::{NAME}"), "--exact", "--test-threads=1"])
                .env(ALONE, "1");
            let out = output_in_files(&mut again).unwrap();
            let said = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            // "1 passed": a name that matches nothing would also end well.
            assert!(
                out.status.success() && said.contains("1 passed"),
                "run alone: {said}"
            );
            return;
        }
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let (n, waited) = rt.block_on(async {
            let (read_end, write_end) = inheritable_pipe();
            let mut cmd = command("sleep", &["5"]);
            cmd.kill_on_drop(true);
            no_inherited_fds(&mut cmd);
            let spawned = cmd.spawn();
            unsafe { libc::close(write_end) };
            let mut child = spawned.unwrap();
            let start = Instant::now();
            let mut byte = 0u8;
            let n = unsafe { libc::read(read_end, (&mut byte as *mut u8).cast(), 1) };
            let waited = start.elapsed();
            unsafe { libc::close(read_end) };
            child.kill().await.unwrap();
            (n, waited)
        });
        assert_eq!(n, 0, "the pipe has an end, nothing was written");
        assert!(
            waited < Duration::from_secs(1),
            "the read waited {waited:?} for the child to let go of the pipe"
        );
    }

    /// Descriptors are marked, not closed: std tells the parent that `exec` failed through a pipe
    /// of its own, which the child still writes to after the closure has run.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_program_that_cannot_start_is_still_reported_by_spawn() {
        let mut cmd = command("/nonexistent/ccm-no-such-program", &[]);
        no_inherited_fds(&mut cmd);
        let err = cmd.spawn().unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::NotFound, "{err}");
    }

    /// The grace after the end of the process only cuts a pipe that stays silent: a program the
    /// command left behind that goes on writing (here for 3 s, more than the grace) is read to its end.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_pipe_that_goes_on_receiving_data_after_the_exit_is_read_to_its_end() {
        let cmd = command(
            "sh",
            &[
                "-c",
                "(for i in 1 2 3 4 5; do sleep 0.6; echo late$i; done) & exit 0",
            ],
        );
        let out = output_within(cmd, Duration::from_secs(60)).await.unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "late1\nlate2\nlate3\nlate4\nlate5\n"
        );
    }
}
