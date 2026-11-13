//! A process and everything it spawns (dev servers started by the Bash tool, MCP servers,
//! `cmd` → `node` chains…) live and die together.
//!
//! On Windows, a Job Object created with KILL_ON_JOB_CLOSE, so the whole tree also dies if the
//! app crashes or is killed (unless it is `release`d: an interactive terminal's, which leaves
//! the windows its user opened alone and ends only what shows none, see `terminate_windowless`).
//! On macOS (and other Unixes), the process group the child leads (see `isolate`): killed as a
//! whole when the job is terminated or dropped. A browser that must outlive it starts out of it
//! (`pty::open_browser_apart`).

/// What the processes of a job use.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct JobUsage {
    pub cpu: std::time::Duration,
    /// Working sets, in bytes.
    pub memory: u64,
    pub processes: u32,
}

#[cfg(windows)]
mod imp {
    use super::JobUsage;
    use std::collections::{HashMap, HashSet};
    use std::ffi::c_void;
    use std::time::Duration;
    use windows_sys::core::BOOL;
    use windows_sys::Win32::Foundation::{
        CloseHandle, GetLastError, ERROR_MORE_DATA, FILETIME, HANDLE, HWND, INVALID_HANDLE_VALUE,
        LPARAM,
    };
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows_sys::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, IsProcessInJob,
        JobObjectBasicAccountingInformation, JobObjectBasicProcessIdList,
        JobObjectExtendedLimitInformation, QueryInformationJobObject, SetInformationJobObject,
        TerminateJobObject, JOBOBJECT_BASIC_ACCOUNTING_INFORMATION,
        JOBOBJECT_BASIC_PROCESS_ID_LIST, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows_sys::Win32::System::ProcessStatus::{
        K32GetProcessMemoryInfo, PROCESS_MEMORY_COUNTERS,
    };
    use windows_sys::Win32::System::Threading::{
        GetProcessTimes, OpenProcess, TerminateProcess, PROCESS_QUERY_LIMITED_INFORMATION,
        PROCESS_SET_QUOTA, PROCESS_TERMINATE,
    };
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowThreadProcessId, IsWindowVisible,
    };

    pub struct Job(HANDLE);

    // SAFETY: a job handle is a kernel object handle, usable from any thread.
    unsafe impl Send for Job {}
    unsafe impl Sync for Job {}

    impl Job {
        pub fn new() -> Option<Self> {
            // SAFETY: plain Win32 calls; the handle is owned by the returned Job.
            unsafe {
                let h = CreateJobObjectW(std::ptr::null(), std::ptr::null());
                if h.is_null() {
                    return None;
                }
                let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
                let ok = SetInformationJobObject(
                    h,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                );
                if ok == 0 {
                    CloseHandle(h);
                    return None;
                }
                Some(Job(h))
            }
        }

        /// `for_child`, of an agent's process. Nothing to note (`agentgroups`, ESC-417): the Job's
        /// KILL_ON_JOB_CLOSE ends the agent's tree with the app, however the app ends (killed,
        /// crashed), as the system closes the Job's last handle.
        pub fn for_agent(child: &tokio::process::Child) -> Option<Self> {
            Self::for_child(child)
        }

        /// A job holding `child` (started by tokio) and what it starts.
        ///
        /// What `child` started before it joined the job (a `.cmd` shim's `cmd` starts its program
        /// at once) is put in the job too: such a process would escape `terminate` and `usage`.
        pub fn for_child(child: &tokio::process::Child) -> Option<Self> {
            let job = Self::new()?;
            if !job.assign_handle(child.raw_handle()?) {
                return None;
            }
            if let Some(pid) = child.id() {
                job.adopt_started_by(pid);
            }
            Some(job)
        }

        /// Puts in the job the processes `pid` started, and those they started, that are not in it
        /// already. Gone ones, ones that refuse (a job of their own that cannot nest) and ones that
        /// are not really born of `pid` (see `descendants`) are left out; returns how many joined.
        fn adopt_started_by(&self, pid: u32) -> usize {
            // Before the snapshot: a process born later joined the job as it was born, and is not
            // looked for; one that the snapshot lists and that was born later is another, which got
            // the id of one that ended.
            let before = filetime_now();
            let Some(family) = process_family() else {
                return 0;
            };
            // The handles stay open until the processes are assigned: a handle holds its process,
            // so the one whose birth was read is the one that joins.
            let mut opened: HashMap<u32, HANDLE> = HashMap::new();
            let found = descendants(&family, pid, before, |id| {
                // SAFETY: the handle is checked, and kept in `opened` or closed.
                unsafe {
                    let h = OpenProcess(
                        PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_SET_QUOTA | PROCESS_TERMINATE,
                        0,
                        id,
                    );
                    if h.is_null() {
                        return None;
                    }
                    let born = creation_time(h);
                    if born.is_some() {
                        opened.insert(id, h);
                    } else {
                        CloseHandle(h);
                    }
                    born
                }
            });
            let mut joined = 0;
            for id in found {
                if let Some(&h) = opened.get(&id) {
                    // SAFETY: a live process handle, opened above.
                    if unsafe { AssignProcessToJobObject(self.0, h) } != 0 {
                        joined += 1;
                    }
                }
            }
            for (_, h) in opened {
                // SAFETY: each handle was opened above and is closed once.
                unsafe { CloseHandle(h) };
            }
            joined
        }

        /// A job holding `child` (started in a pseudo-console) and what it starts.
        pub fn for_pty(child: &dyn portable_pty::Child) -> Option<Self> {
            let job = Self::new()?;
            job.assign_handle(child.as_raw_handle()?).then_some(job)
        }

        /// Assigns a process by handle (e.g. `tokio::process::Child::raw_handle`).
        pub fn assign_handle(&self, process: *mut c_void) -> bool {
            // SAFETY: the caller passes a live process handle.
            unsafe { AssignProcessToJobObject(self.0, process as HANDLE) != 0 }
        }

        /// Kills every process of the job.
        pub fn terminate(&self) {
            // SAFETY: valid job handle owned by self.
            unsafe {
                TerminateJobObject(self.0, 1);
            }
        }

        /// Kills the processes of the job that show no window, and leaves the others: a process
        /// with a visible top-level window (a browser, an editor) and those it started, whose own
        /// windows may be none (a browser's renderers) but which it cannot do without.
        pub fn terminate_windowless(&self) {
            let pids = self.pids();
            if pids.is_empty() {
                return;
            }
            let family = process_family().unwrap_or_default();
            let spared = spared(&pids, &visible_window_owners(), &family);
            for pid in pids.into_iter().filter(|pid| !spared.contains(pid)) {
                self.terminate_process(pid);
            }
        }

        /// Ends one process of the job; nothing if it is gone, or if its id is another process's
        /// by now (it is checked to be in the job).
        fn terminate_process(&self, pid: u32) {
            // SAFETY: the handle is checked, used for two calls and closed.
            unsafe {
                let h = OpenProcess(
                    PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE,
                    0,
                    pid,
                );
                if h.is_null() {
                    return;
                }
                let mut in_job = 0;
                if IsProcessInJob(h, self.0, &mut in_job) != 0 && in_job != 0 {
                    TerminateProcess(h, 1);
                }
                CloseHandle(h);
            }
        }

        /// Lets the processes of the job live on when it is dropped (a terminal's shell: the app
        /// closing, crashing or restarting for an update leaves a browser it started open): the
        /// job keeps holding them, but no longer kills them when its handle closes. `terminate`
        /// still does.
        pub fn release(&self) {
            // SAFETY: valid job handle owned by self, a buffer of the declared size.
            let released = unsafe {
                let info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = std::mem::zeroed();
                SetInformationJobObject(
                    self.0,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            if released == 0 {
                // SAFETY: plain call.
                let code = unsafe { GetLastError() };
                log::warn!("job: not released, it kills its processes when closed (error {code})");
            }
        }

        /// What the job's processes use: CPU time since they joined it (ended ones included),
        /// and the memory of those still running.
        pub fn usage(&self) -> Option<JobUsage> {
            // SAFETY: plain Win32 queries on our own job handle, into buffers of the declared size.
            unsafe {
                let mut acc: JOBOBJECT_BASIC_ACCOUNTING_INFORMATION = std::mem::zeroed();
                let ok = QueryInformationJobObject(
                    self.0,
                    JobObjectBasicAccountingInformation,
                    &mut acc as *mut _ as *mut c_void,
                    std::mem::size_of::<JOBOBJECT_BASIC_ACCOUNTING_INFORMATION>() as u32,
                    std::ptr::null_mut(),
                );
                if ok == 0 {
                    return None;
                }
                let ticks = (acc.TotalUserTime + acc.TotalKernelTime).max(0) as u64;
                Some(JobUsage {
                    cpu: Duration::from_nanos(ticks * 100),
                    memory: self
                        .pids()
                        .into_iter()
                        .map(|pid| self.working_set(pid))
                        .sum(),
                    processes: acc.ActiveProcesses,
                })
            }
        }

        /// The ids of the job's processes (the first thousand).
        pub fn pids(&self) -> Vec<u32> {
            const MAX: usize = 1024;
            // u64 cells: the list's header and ids need 8-byte alignment.
            let mut buf = vec![0u64; 1 + MAX];
            // SAFETY: `buf` holds a JOBOBJECT_BASIC_PROCESS_ID_LIST with room for MAX ids; the ids
            // are read through a pointer derived from the whole buffer, not from the 1-item array.
            unsafe {
                let ok = QueryInformationJobObject(
                    self.0,
                    JobObjectBasicProcessIdList,
                    buf.as_mut_ptr() as *mut c_void,
                    (buf.len() * 8) as u32,
                    std::ptr::null_mut(),
                );
                // More processes than room: the list holds the first ones.
                if ok == 0 && GetLastError() != ERROR_MORE_DATA {
                    return Vec::new();
                }
                let list = buf.as_ptr() as *const JOBOBJECT_BASIC_PROCESS_ID_LIST;
                let n = ((*list).NumberOfProcessIdsInList as usize).min(MAX);
                let ids = std::ptr::addr_of!((*list).ProcessIdList) as *const usize;
                std::slice::from_raw_parts(ids, n)
                    .iter()
                    .map(|&pid| pid as u32)
                    .collect()
            }
        }

        /// The working set of one of the job's processes, 0 when it is gone (its id may already
        /// be another process's).
        fn working_set(&self, pid: u32) -> u64 {
            // SAFETY: the handle is checked, used for two queries and closed.
            unsafe {
                let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
                if h.is_null() {
                    return 0;
                }
                let mut in_job = 0;
                let mut mem: PROCESS_MEMORY_COUNTERS = std::mem::zeroed();
                let size = std::mem::size_of::<PROCESS_MEMORY_COUNTERS>() as u32;
                let ok = IsProcessInJob(h, self.0, &mut in_job) != 0
                    && in_job != 0
                    && K32GetProcessMemoryInfo(h, &mut mem, size) != 0;
                CloseHandle(h);
                if ok {
                    mem.WorkingSetSize as u64
                } else {
                    0
                }
            }
        }
    }

    /// The `(id, parent id)` of the machine's processes, None when they cannot be listed.
    fn process_family() -> Option<Vec<(u32, u32)>> {
        let mut family: Vec<(u32, u32)> = Vec::new();
        // SAFETY: a snapshot of the machine's processes, read to its end and closed; the entry is
        // a plain struct whose size is declared as the API asks.
        unsafe {
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snapshot == INVALID_HANDLE_VALUE {
                return None;
            }
            let mut entry: PROCESSENTRY32W = std::mem::zeroed();
            entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut more = Process32FirstW(snapshot, &mut entry);
            while more != 0 {
                family.push((entry.th32ProcessID, entry.th32ParentProcessID));
                more = Process32NextW(snapshot, &mut entry);
            }
            CloseHandle(snapshot);
        }
        Some(family)
    }

    /// The ids of the processes that own a visible top-level window.
    fn visible_window_owners() -> HashSet<u32> {
        unsafe extern "system" fn note(window: HWND, owners: LPARAM) -> BOOL {
            // SAFETY: `owners` is the set `visible_window_owners` lends for the enumeration.
            let owners = unsafe { &mut *(owners as *mut HashSet<u32>) };
            // SAFETY: plain queries on the window the enumeration gives.
            unsafe {
                if IsWindowVisible(window) != 0 {
                    let mut pid = 0;
                    GetWindowThreadProcessId(window, &mut pid);
                    if pid != 0 {
                        owners.insert(pid);
                    }
                }
            }
            // Go on.
            1
        }
        let mut owners = HashSet::new();
        // SAFETY: the callback only reads the windows and fills the set, alive for the call.
        unsafe {
            EnumWindows(Some(note), &mut owners as *mut HashSet<u32> as LPARAM);
        }
        owners
    }

    /// Among the job's processes `pids`, those to leave alive: the ones in `windowed` (owners of
    /// a visible window) and every one started by them, at any depth (`family`: the `(id, parent
    /// id)` of the machine's processes). A stale parent id only spares one more.
    pub(super) fn spared(
        pids: &[u32],
        windowed: &HashSet<u32>,
        family: &[(u32, u32)],
    ) -> HashSet<u32> {
        let mut spared: HashSet<u32> = pids
            .iter()
            .copied()
            .filter(|pid| windowed.contains(pid))
            .collect();
        loop {
            let before = spared.len();
            for &(id, parent) in family {
                if pids.contains(&id) && spared.contains(&parent) {
                    spared.insert(id);
                }
            }
            if spared.len() == before {
                return spared;
            }
        }
    }

    /// Now, as a FILETIME (100 ns since 1601), the unit of the birth of a process.
    fn filetime_now() -> u64 {
        const SINCE_1601: u64 = 116_444_736_000_000_000;
        let since_1970 = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        SINCE_1601 + (since_1970.as_nanos() / 100) as u64
    }

    /// When the process `h` was born (a FILETIME), None when it cannot be read.
    fn creation_time(h: HANDLE) -> Option<u64> {
        // SAFETY: a live process handle and four plain structs to fill.
        unsafe {
            let (mut born, mut ended, mut kernel, mut user): (
                FILETIME,
                FILETIME,
                FILETIME,
                FILETIME,
            ) = std::mem::zeroed();
            (GetProcessTimes(h, &mut born, &mut ended, &mut kernel, &mut user) != 0)
                .then(|| (u64::from(born.dwHighDateTime) << 32) | u64::from(born.dwLowDateTime))
        }
    }

    /// The ids, among `family` (the `(id, parent id)` of a snapshot of the machine's processes
    /// taken at `snapshot_at`), of the processes `root` started, and those they started. `born`
    /// says when a process was born (None: gone, or out of reach). An id is reused once its process
    /// ends, so a parent id in the snapshot proves nothing by itself: a process counts only if it
    /// was born after its parent (else it is the old child of a process that had that id) and
    /// before the snapshot (else it took the id of one that ended).
    pub(super) fn descendants(
        family: &[(u32, u32)],
        root: u32,
        snapshot_at: u64,
        mut born: impl FnMut(u32) -> Option<u64>,
    ) -> Vec<u32> {
        let Some(root_born) = born(root) else {
            return Vec::new();
        };
        let mut found: Vec<u32> = Vec::new();
        let mut parents = vec![(root, root_born)];
        while let Some((parent, parent_born)) = parents.pop() {
            for &(id, of) in family {
                if of != parent || id == root || found.contains(&id) {
                    continue;
                }
                match born(id) {
                    Some(b) if b >= parent_born && b <= snapshot_at => {
                        found.push(id);
                        parents.push((id, b));
                    }
                    _ => {}
                }
            }
        }
        found
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // SAFETY: closing our own handle; with KILL_ON_JOB_CLOSE (unless released) this ends
            // the tree.
            unsafe {
                CloseHandle(self.0);
            }
        }
    }
}

#[cfg(unix)]
mod imp {
    use super::JobUsage;
    use std::time::Duration;

    /// A process group, by the id of its leader; noted (`agentgroups`) when it is an agent's.
    pub struct Job(i32, bool);

    impl Job {
        /// A job holding `child`, started by tokio after `isolate`, and what it starts.
        pub fn for_child(child: &tokio::process::Child) -> Option<Self> {
            Some(Job(child.id()?.try_into().ok()?, false))
        }

        /// `for_child`, of an agent's process: its group is noted, so that a launch after an app
        /// killed stops it (`agentgroups`, ESC-417).
        pub fn for_agent(child: &tokio::process::Child) -> Option<Self> {
            let pid: i32 = child.id()?.try_into().ok()?;
            crate::agentgroups::note(pid);
            Some(Job(pid, true))
        }

        /// A job holding `child`, started in a pseudo-console: the session it leads is also its
        /// process group.
        pub fn for_pty(child: &dyn portable_pty::Child) -> Option<Self> {
            Some(Job(child.process_id()?.try_into().ok()?, false))
        }

        /// Kills every process of the group.
        pub fn terminate(&self) {
            // SAFETY: plain syscall; a group that is already gone only yields ESRCH.
            unsafe {
                libc::killpg(self.0, libc::SIGKILL);
            }
        }

        /// A group knows no windows: all of it. What shows one and must stay (the browser of a
        /// sign-in) was started out of the group (`pty::open_browser_apart`).
        pub fn terminate_windowless(&self) {
            self.terminate();
        }

        /// Nothing to release: a group is killed when the job is dropped, so a terminal whose
        /// shell may end leaving what its user detached has no job on Unix (see `pty`).
        pub fn release(&self) {}

        /// What the group's running processes use (CPU time of those still running, resident
        /// memory), read from `ps`.
        pub fn usage(&self) -> Option<JobUsage> {
            let out = std::process::Command::new("ps")
                .args(["-A", "-o", "pgid=,rss=,time="])
                .stderr(std::process::Stdio::null())
                .output()
                .ok()?;
            out.status.success().then_some(())?;
            Some(parse_ps(&String::from_utf8_lossy(&out.stdout), self.0))
        }

        /// The ids of the group's running processes, read from `ps`.
        pub fn pids(&self) -> Vec<u32> {
            let Ok(out) = std::process::Command::new("ps")
                .args(["-A", "-o", "pgid=,pid="])
                .stderr(std::process::Stdio::null())
                .output()
            else {
                return Vec::new();
            };
            parse_pids(&String::from_utf8_lossy(&out.stdout), self.0)
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // Like KILL_ON_JOB_CLOSE: whatever the leader left running goes with it.
            self.terminate();
            if self.1 {
                crate::agentgroups::forget(self.0);
            }
        }
    }

    /// Sums the `pgid rss time` lines of `ps` for group `pgid`.
    pub(super) fn parse_ps(out: &str, pgid: i32) -> JobUsage {
        let mut u = JobUsage::default();
        for line in out.lines() {
            let mut f = line.split_whitespace();
            let (Some(g), Some(rss), Some(time)) = (f.next(), f.next(), f.next()) else {
                continue;
            };
            if g.parse::<i32>().ok() != Some(pgid) {
                continue;
            }
            u.processes += 1;
            u.memory += rss.parse::<u64>().unwrap_or(0) * 1024;
            u.cpu += parse_cpu_time(time).unwrap_or_default();
        }
        u
    }

    /// The ids of the `pgid pid` lines of `ps` for group `pgid`.
    pub(super) fn parse_pids(out: &str, pgid: i32) -> Vec<u32> {
        out.lines()
            .filter_map(|line| {
                let mut f = line.split_whitespace();
                let (g, pid) = (f.next()?, f.next()?);
                (g.parse::<i32>().ok()? == pgid)
                    .then(|| pid.parse::<u32>().ok())
                    .flatten()
            })
            .collect()
    }

    /// `ps` CPU times:`[[dd-]hh:]mm:ss[.cc]` (Linux `00:01:02`, macOS `1:02.03`).
    pub(super) fn parse_cpu_time(s: &str) -> Option<Duration> {
        let (days, rest) = match s.split_once('-') {
            Some((d, r)) => (d.parse::<u64>().ok()?, r),
            None => (0, s),
        };
        let mut secs = 0f64;
        for part in rest.split(':') {
            secs = secs * 60.0 + part.parse::<f64>().ok()?;
        }
        Some(Duration::from_secs_f64(secs + days as f64 * 86_400.0))
    }
}

#[cfg(not(any(windows, unix)))]
mod imp {
    pub struct Job;

    impl Job {
        pub fn for_child(_child: &tokio::process::Child) -> Option<Self> {
            None
        }
        pub fn for_agent(_child: &tokio::process::Child) -> Option<Self> {
            None
        }
        pub fn for_pty(_child: &dyn portable_pty::Child) -> Option<Self> {
            None
        }
        pub fn terminate(&self) {}
        pub fn terminate_windowless(&self) {}
        pub fn release(&self) {}
        pub fn usage(&self) -> Option<super::JobUsage> {
            None
        }
        pub fn pids(&self) -> Vec<u32> {
            Vec::new()
        }
    }
}

/// Prepares `cmd` so that `Job::for_child` can hold the process and what it starts: on Unix, the
/// child leads a process group of its own.
pub fn isolate(cmd: &mut tokio::process::Command) {
    #[cfg(unix)]
    cmd.process_group(0);
    #[cfg(not(unix))]
    let _ = cmd;
}

pub use imp::Job;

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    /// The `PATH` of a test of the workspace holds every folder a build script gives the linker, those of
    /// whisper-rs-sys's cmake tree among them (46): past the 8191 characters cmd.exe reads. They go, once, as the app's
    /// `paths::trim_test_path` does.
    fn trim_test_path() {
        static ONCE: std::sync::Once = std::sync::Once::new();
        ONCE.call_once(|| {
            let Some(path) = std::env::var_os("PATH") else {
                return;
            };
            let kept: Vec<std::path::PathBuf> = std::env::split_paths(&path)
                .filter(|p| !p.to_string_lossy().contains("whisper-rs-sys-"))
                .collect();
            if let Ok(joined) = std::env::join_paths(kept) {
                // Edition 2021: safe; done once, before the test spawns anything.
                std::env::set_var("PATH", joined);
            }
        });
    }
    use std::process::Command;

    fn alive(pid: u32) -> bool {
        let out = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&pid.to_string())
    }

    #[test]
    fn dropping_the_job_kills_the_process() {
        let mut child = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Start-Sleep 60"])
            .spawn()
            .unwrap();
        let job = Job::new().unwrap();
        assert!(job.assign_handle(std::os::windows::io::AsRawHandle::as_raw_handle(&child)));
        drop(job);
        child.wait().unwrap();
        assert!(!alive(child.id()));
    }

    #[test]
    fn a_released_job_kills_nothing_when_it_is_dropped() {
        // Not `ping`: the terminals' tests count the machine's `ping` processes.
        let mut child = Command::new("powershell")
            .args(["-NoProfile", "-Command", "Start-Sleep 30"])
            .spawn()
            .unwrap();
        let job = Job::new().unwrap();
        assert!(job.assign_handle(std::os::windows::io::AsRawHandle::as_raw_handle(&child)));
        job.release();
        drop(job);
        std::thread::sleep(std::time::Duration::from_millis(500));
        let survived = child.try_wait().unwrap().is_none();
        child.kill().unwrap();
        child.wait().unwrap();
        assert!(survived, "the released job killed its process");
    }

    #[test]
    fn the_processes_with_a_window_and_the_ones_they_started_are_spared() {
        let windowed = std::collections::HashSet::from([20, 99]);
        // 20 shows a window, 30 is its helper (no window) and 31 its helper's; 10 and 40 show
        // none and are not its own; 50 shows none but is not in the job; 99 shows a window, not in
        // the job.
        let family = [(10, 1), (20, 10), (30, 20), (31, 30), (40, 10), (50, 20)];
        let spared = super::imp::spared(&[10, 20, 30, 31, 40], &windowed, &family);
        let mut spared: Vec<u32> = spared.into_iter().collect();
        spared.sort();
        assert_eq!(spared, vec![20, 30, 31]);
        // No window at all: nothing is spared.
        let none = std::collections::HashSet::new();
        assert!(super::imp::spared(&[10, 20], &none, &family).is_empty());
    }

    /// `descendants` over a snapshot of `(id, parent)` and the births of the processes in it.
    fn adopted(family: &[(u32, u32)], root: u32, at: u64, births: &[(u32, u64)]) -> Vec<u32> {
        let mut found = super::imp::descendants(family, root, at, |id| {
            births.iter().find(|b| b.0 == id).map(|b| b.1)
        });
        found.sort();
        found
    }

    #[test]
    fn the_processes_born_of_the_child_are_adopted_with_theirs() {
        // 10 is the child, 20 its own, 30 theirs; 99 is another's.
        let family = [(10, 1), (20, 10), (30, 20), (99, 5)];
        let births = [(10, 100), (20, 150), (30, 160), (99, 40)];
        assert_eq!(adopted(&family, 10, 200, &births), vec![20, 30]);
    }

    #[test]
    fn a_process_born_before_the_child_is_not_its_own_whatever_its_parent_id_says() {
        // 40 was the child of a process that had the id 10 before the child: it is a stranger (and
        // so is 41, its own), though the snapshot names the child as its parent.
        let family = [(10, 1), (20, 10), (40, 10), (41, 40)];
        let births = [(10, 100), (20, 150), (40, 50), (41, 60)];
        assert_eq!(adopted(&family, 10, 200, &births), vec![20]);
        // One born before its parent, under a genuine descendant, is no more a descendant.
        let family = [(10, 1), (20, 10), (31, 20)];
        let births = [(10, 100), (20, 150), (31, 120)];
        assert_eq!(adopted(&family, 10, 200, &births), vec![20]);
    }

    #[test]
    fn a_process_born_after_the_snapshot_is_not_the_one_it_listed() {
        // 50 took the id of a child of 10 that ended after the snapshot.
        let family = [(10, 1), (20, 10), (50, 10)];
        let births = [(10, 100), (20, 150), (50, 300)];
        assert_eq!(adopted(&family, 10, 200, &births), vec![20]);
    }

    #[test]
    fn a_process_gone_or_out_of_reach_is_left_out_with_its_own() {
        let family = [(10, 1), (20, 10), (30, 20), (60, 10)];
        // 20 cannot be read: neither it nor 30 (its child) is adopted; 60 is.
        let births = [(10, 100), (30, 160), (60, 170)];
        assert_eq!(adopted(&family, 10, 200, &births), vec![60]);
        // The child itself unreadable: nothing is adopted.
        assert_eq!(adopted(&family, 10, 200, &[(20, 150)]), Vec::<u32>::new());
    }

    #[tokio::test]
    async fn a_program_the_process_started_before_it_joined_the_job_is_held_too() {
        // What a `.cmd` shim does: `cmd` starts its program at once, maybe before the job exists.
        // The program says it runs, by a file; `cmd` waits for it to end.
        trim_test_path();
        let up = std::env::temp_dir().join(format!("ccm-job-adopt-{}", std::process::id()));
        let _ = std::fs::remove_file(&up);
        let mut cmd = tokio::process::Command::new("cmd");
        cmd.args([
            "/c",
            "node",
            "-e",
            "require('fs').writeFileSync(process.argv[1],'x');setTimeout(function(){},30000)",
        ])
        .arg(&up)
        .kill_on_drop(true);
        let mut child = cmd.spawn().unwrap();
        for _ in 0..400 {
            if up.exists() {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        }
        assert!(up.exists(), "the program did not start");
        let job = Job::for_child(&child).unwrap();
        // `cmd` and its program (node may also have a helper): not just the process itself.
        assert!(job.usage().unwrap().processes >= 2, "{:?}", job.pids());
        job.terminate();
        child.wait().await.unwrap();
        let _ = std::fs::remove_file(&up);
    }

    /// Kills its child when it is dropped, whatever way the test ends (a failed assertion, a job
    /// that could not be made): a process that never stops is not left running.
    struct KillOnDrop(std::process::Child);

    impl Drop for KillOnDrop {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    #[test]
    fn the_guard_kills_its_child_when_a_test_ends_without_reaching_its_cleanup() {
        let pid = {
            let child = KillOnDrop(
                Command::new("powershell")
                    .args(["-NoProfile", "-Command", "Start-Sleep 60"])
                    .spawn()
                    .unwrap(),
            );
            child.0.id()
        };
        assert!(!alive(pid));
    }

    #[test]
    fn tells_the_cpu_time_and_memory_of_its_processes() {
        let mut child = KillOnDrop(
            Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    // Busy without end: the test waits for the Job's CPU time to pass a second, and
                    // a script that stops by the process's own may be over before the Job shows it.
                    // It dies with the Job, dropped at the end of the test (or with the guard).
                    "while ($true) {}",
                ])
                .spawn()
                .unwrap(),
        );
        let job = Job::new().unwrap();
        assert!(job.assign_handle(std::os::windows::io::AsRawHandle::as_raw_handle(&child.0)));
        let mut usage = job.usage().unwrap();
        // Two minutes: a loaded CI runner gave it 0.4 s of CPU in 30 s.
        for _ in 0..1200 {
            if usage.cpu > std::time::Duration::from_secs(1) {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(100));
            usage = job.usage().unwrap();
        }
        assert!(usage.cpu > std::time::Duration::from_secs(1), "{usage:?}");
        assert_eq!(usage.processes, 1);
        assert!(usage.memory > 10 * 1024 * 1024, "{usage:?}");
        drop(job);
        child.0.wait().unwrap();
    }
}

#[cfg(all(test, unix))]
mod unix_tests {
    use super::imp::{parse_cpu_time, parse_pids, parse_ps};
    use super::*;
    use std::time::Duration;

    #[test]
    fn reads_the_cpu_times_of_ps() {
        assert_eq!(
            parse_cpu_time("1:02.50"),
            Some(Duration::from_millis(62_500))
        );
        assert_eq!(parse_cpu_time("01:00:03"), Some(Duration::from_secs(3603)));
        assert_eq!(
            parse_cpu_time("2-00:00:00"),
            Some(Duration::from_secs(172_800))
        );
        assert_eq!(parse_cpu_time("n/a"), None);
    }

    #[test]
    fn sums_the_processes_of_the_group() {
        let out = " 42  1000 0:01.00\n 42  24 0:00.50\n 7 99999 9:00.00\n";
        let u = parse_ps(out, 42);
        assert_eq!(u.processes, 2);
        assert_eq!(u.memory, 1024 * 1024);
        assert_eq!(u.cpu, Duration::from_millis(1500));
    }

    #[test]
    fn lists_the_pids_of_the_group() {
        let out = " 42  100\n 42  101\n 7  102\n junk\n 42 x\n";
        assert_eq!(parse_pids(out, 42), vec![100, 101]);
    }

    #[tokio::test]
    async fn dropping_the_job_kills_the_whole_group() {
        let mut cmd = tokio::process::Command::new("sh");
        // The grandchild outlives its parent, which echoes its pid.
        cmd.args(["-c", "sleep 60 & echo $!; wait"])
            .stdout(std::process::Stdio::piped());
        isolate(&mut cmd);
        let mut child = cmd.spawn().unwrap();
        let job = Job::for_child(&child).unwrap();
        let mut out = tokio::io::BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        tokio::io::AsyncBufReadExt::read_line(&mut out, &mut line)
            .await
            .unwrap();
        let grandchild: i32 = line.trim().parse().unwrap();
        assert!(job.usage().unwrap().processes >= 2);
        drop(job);
        child.wait().await.unwrap();
        for _ in 0..100 {
            // SAFETY: signal 0 only checks that the process exists.
            if unsafe { libc::kill(grandchild, 0) } != 0 {
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("grandchild {grandchild} survived the job");
    }
}
