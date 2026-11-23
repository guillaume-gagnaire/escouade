//! A process and everything it spawns (dev servers started by the Bash tool, MCP servers,
//! `cmd` → `node` chains…) live and die together.
//!
//! On Windows, a Job Object created with KILL_ON_JOB_CLOSE, so the whole tree also dies if the
//! app crashes or is killed (unless it is `release`d: an interactive terminal's, which leaves
//! the windows its user opened alone and ends only what shows none, see `terminate_windowless`).
//! On macOS and Linux (and other Unixes), the process group the child leads (see `isolate`), and
//! what its processes start apart from it, in a group or a session of its own (ESC-656): counted,
//! and killed as a whole when the job is terminated or dropped. A browser that must outlive it
//! starts out of it (`pty::open_browser_apart`). A sample of the agents' resources reads the
//! machine's processes once for all of them (`Sample`, `Job::usage_in`).

/// What the processes of a job use.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct JobUsage {
    pub cpu: std::time::Duration,
    /// In bytes, summed over the processes: their working sets on Windows, their physical
    /// footprints on macOS (the Activity Monitor's « Memory »), their resident memory elsewhere,
    /// which counts again in each process the pages of the libraries they share.
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

    /// Nothing to read once for every job: each Job Object tells its own (`Job::usage_in`).
    pub struct Sample;

    impl Sample {
        pub fn read() -> Self {
            Sample
        }
    }

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

        /// `usage`: a job knows its own processes, the reading of the machine is not needed.
        pub fn usage_in(&self, _sample: &Sample) -> Option<JobUsage> {
            self.usage()
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
    use parking_lot::Mutex;
    use std::collections::{HashMap, HashSet};
    use std::time::Duration;

    /// A process as one reading of `ps` tells it.
    #[derive(Debug, Clone, Copy, PartialEq)]
    pub(super) struct Row {
        pub pid: i32,
        pub ppid: i32,
        pub pgid: i32,
        /// Resident memory, in KiB.
        pub rss: u64,
        pub cpu: Duration,
    }

    /// One reading of the machine's processes (`ps`), read once for all the jobs a sample of the
    /// resources looks at (`Job::usage_in`). Those done but not yet reaped (zombies) are left out:
    /// they use nothing, and their children already went to another parent.
    pub struct Sample(Option<Vec<Row>>);

    impl Sample {
        pub fn read() -> Self {
            let out = std::process::Command::new("ps")
                .args(["-A", "-o", "pid=,ppid=,pgid=,stat=,rss=,time="])
                .env("LC_ALL", "C")
                .stderr(std::process::Stdio::null())
                .output();
            Sample(match out {
                Ok(out) if out.status.success() => {
                    Some(parse_ps(&String::from_utf8_lossy(&out.stdout)))
                }
                _ => None,
            })
        }
    }

    /// A process group, by the id of its leader, and what its leader started apart from it.
    ///
    /// An agent's tree is more than its group: what it starts in a session or a group of its own
    /// (a detached Node `spawn`, a command of Claude Code's Bash tool, an MCP server) leaves it.
    /// So the tree (`tree`) is its group, the descendants of its leader by their parent's id, the
    /// groups these lead, and what was seen of it before and still runs (`seen`: a descendant
    /// whose parent ended went to another parent, as on Windows it stays in the job). A process
    /// is known by its id and its start (`start_of`): a pid another process took since is never
    /// counted nor killed.
    pub struct Job {
        leader: i32,
        /// When the leader started (`u64::MAX`: it was done already; None, a test's: whatever
        /// process has its pid).
        start: Option<u64>,
        /// Noted (`agentgroups`) as an agent's.
        agent: bool,
        /// The processes of the tree seen at the last reading, and their start.
        seen: Mutex<HashMap<i32, u64>>,
    }

    impl Job {
        fn new(leader: i32, agent: bool) -> Self {
            Job {
                leader,
                // Done already (a zombie not yet reaped): a start no process will have, so that
                // the pid, once reaped and taken by another, is never the job's.
                start: Some(start_of(leader).unwrap_or(u64::MAX)),
                agent,
                seen: Mutex::new(HashMap::new()),
            }
        }

        /// A job holding `child`, started by tokio after `isolate`, and what it starts.
        pub fn for_child(child: &tokio::process::Child) -> Option<Self> {
            Some(Job::new(child.id()?.try_into().ok()?, false))
        }

        /// `for_child`, of an agent's process: its group is noted, so that a launch after an app
        /// killed stops it (`agentgroups`, ESC-417).
        pub fn for_agent(child: &tokio::process::Child) -> Option<Self> {
            let pid: i32 = child.id()?.try_into().ok()?;
            crate::agentgroups::note(pid);
            Some(Job::new(pid, true))
        }

        /// A job holding `child`, started in a pseudo-console: the session it leads is also its
        /// process group.
        pub fn for_pty(child: &dyn portable_pty::Child) -> Option<Self> {
            Some(Job::new(child.process_id()?.try_into().ok()?, false))
        }

        /// The job of `leader`, said to have started at `start` (a test's pid taken by another).
        #[cfg(test)]
        pub(super) fn claiming(leader: i32, start: Option<u64>) -> Self {
            Job {
                leader,
                start,
                agent: false,
                seen: Mutex::new(HashMap::new()),
            }
        }

        /// Kills every process of the tree (`tree`), read again just before, and the groups its
        /// processes lead. What ends a process another one may have started meanwhile is read
        /// again, a few times, until none is new.
        pub fn terminate(&self) {
            let leader = start_of(self.leader);
            let leader_runs = self.start.is_none() || (leader.is_some() && leader == self.start);
            // The leader is gone, and nothing was seen out of its group: what it started apart
            // went to another parent, out of reach. Its group alone, as before, unless its pid is
            // another process's now.
            if !leader_runs && self.seen.lock().is_empty() {
                if leader.is_none() {
                    kill_group(self.leader);
                }
                return;
            }
            let mut killed = HashSet::new();
            for _ in 0..5 {
                let Some(rows) = Sample::read().0 else {
                    if leader.is_none() || leader == self.start {
                        kill_group(self.leader);
                    }
                    return;
                };
                let tree = self.tree(&rows);
                if tree.group {
                    kill_group(self.leader);
                }
                let mut new = false;
                for row in &tree.members {
                    if !killed.insert(row.pid) {
                        continue;
                    }
                    new = true;
                    // Itself still, just before the signal.
                    let Some(start) = tree.seen.get(&row.pid) else {
                        continue;
                    };
                    if start_of(row.pid) != Some(*start) {
                        continue;
                    }
                    if row.pgid == row.pid && row.pid != self.leader {
                        kill_group(row.pid);
                    }
                    // SAFETY: plain syscall, on a process of the tree.
                    unsafe {
                        libc::kill(row.pid, libc::SIGKILL);
                    }
                }
                if !new {
                    break;
                }
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

        /// What the tree's running processes use: CPU time of those still running, and their
        /// memory (`memory`).
        pub fn usage(&self) -> Option<JobUsage> {
            self.usage_in(&Sample::read())
        }

        /// `usage`, from a reading of the machine shared with other jobs.
        pub fn usage_in(&self, sample: &Sample) -> Option<JobUsage> {
            let rows = sample.0.as_ref()?;
            let mut u = JobUsage::default();
            for row in self.tree(rows).members {
                u.processes += 1;
                u.memory += memory(&row);
                u.cpu += row.cpu;
            }
            Some(u)
        }

        /// The ids of the tree's running processes.
        pub fn pids(&self) -> Vec<u32> {
            let Some(rows) = Sample::read().0 else {
                return Vec::new();
            };
            let mut pids: Vec<u32> = self
                .tree(&rows)
                .members
                .iter()
                .filter_map(|r| r.pid.try_into().ok())
                .collect();
            pids.sort_unstable();
            pids
        }

        /// The tree in `rows`, what it saw noted for the next reading.
        fn tree(&self, rows: &[Row]) -> Tree {
            let mut seen = self.seen.lock();
            let tree = tree(rows, self.leader, self.start, &seen, start_of);
            seen.clone_from(&tree.seen);
            tree
        }
    }

    impl Drop for Job {
        fn drop(&mut self) {
            // Like KILL_ON_JOB_CLOSE: whatever the leader left running goes with it.
            self.terminate();
            if self.agent {
                crate::agentgroups::forget(self.leader);
            }
        }
    }

    /// SIGKILL to the group `pgid`.
    fn kill_group(pgid: i32) {
        // SAFETY: plain syscall; a group that is already gone only yields ESRCH.
        unsafe {
            libc::killpg(pgid, libc::SIGKILL);
        }
    }

    /// The processes of a job's tree in a reading.
    #[derive(Debug, Default)]
    pub(super) struct Tree {
        pub members: Vec<Row>,
        /// Whether the leader's group is the job's (its pid not another process's).
        pub group: bool,
        /// The members and their start, for the next reading (those whose start was told).
        pub seen: HashMap<i32, u64>,
    }

    /// The tree of the job led by `leader`, started at `start`, in `rows`: its group, the
    /// descendants of its processes by their parent's id, the groups these lead, and the
    /// processes `seen` at the last reading still running with the same start. A leader whose
    /// pid another process took is not the job's, nor is its group (a group's id is not given to
    /// a process while the group lives); `start_of` tells when a process started (None: gone).
    pub(super) fn tree(
        rows: &[Row],
        leader: i32,
        start: Option<u64>,
        seen: &HashMap<i32, u64>,
        start_of: impl Fn(i32) -> Option<u64>,
    ) -> Tree {
        let mut children: HashMap<i32, Vec<usize>> = HashMap::new();
        let mut groups: HashMap<i32, Vec<usize>> = HashMap::new();
        let mut at: HashMap<i32, usize> = HashMap::new();
        for (i, r) in rows.iter().enumerate() {
            children.entry(r.ppid).or_default().push(i);
            groups.entry(r.pgid).or_default().push(i);
            at.insert(r.pid, i);
        }
        let mut starts = HashMap::new();
        let mut queue: Vec<usize> = Vec::new();
        let now = at.get(&leader).and_then(|_| start_of(leader));
        let group = match (at.get(&leader), now) {
            // The leader runs: itself, and through it its group and descendants.
            (Some(&i), Some(s)) if start.is_none() || Some(s) == start => {
                queue.push(i);
                starts.insert(leader, s);
                true
            }
            // Its pid is another process's: neither it nor its group are the job's.
            (Some(_), Some(_)) => false,
            // It is gone: what is left of its group.
            _ => {
                queue.extend(groups.get(&leader).into_iter().flatten());
                true
            }
        };
        for (&pid, &s) in seen {
            if let Some(&i) = at.get(&pid) {
                if pid != leader && start_of(pid) == Some(s) {
                    starts.insert(pid, s);
                    queue.push(i);
                }
            }
        }
        let mut members = HashSet::new();
        while let Some(i) = queue.pop() {
            if !members.insert(i) {
                continue;
            }
            let pid = rows[i].pid;
            // Its children, and the processes of the group it leads.
            queue.extend(children.get(&pid).into_iter().flatten());
            queue.extend(groups.get(&pid).into_iter().flatten());
        }
        let mut members: Vec<usize> = members.into_iter().collect();
        members.sort_unstable();
        let members: Vec<Row> = members.into_iter().map(|i| rows[i]).collect();
        let seen = members
            .iter()
            .filter_map(|r| {
                let s = match starts.get(&r.pid) {
                    Some(&s) => s,
                    None => start_of(r.pid)?,
                };
                Some((r.pid, s))
            })
            .collect();
        Tree {
            members,
            group,
            seen,
        }
    }

    /// When `pid` started, as the system tells it; None when it is gone or done (a zombie).
    #[cfg(target_os = "linux")]
    pub(super) fn start_of(pid: i32) -> Option<u64> {
        let stat = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
        // After the program's name, which may hold spaces and parentheses: the state (field 3),
        // then the start (field 22, in clock ticks since boot).
        let mut f = stat[stat.rfind(')')? + 1..].split_whitespace();
        if matches!(f.next()?, "Z" | "X") {
            return None;
        }
        f.nth(18)?.parse().ok()
    }

    /// When `pid` started, as the system tells it; None when it is gone or done (a zombie).
    #[cfg(target_os = "macos")]
    pub(super) fn start_of(pid: i32) -> Option<u64> {
        // SAFETY: `info` is a plain C struct the call fills, of the size given.
        unsafe {
            let mut info: libc::proc_bsdinfo = std::mem::zeroed();
            let size = std::mem::size_of::<libc::proc_bsdinfo>() as libc::c_int;
            let n = libc::proc_pidinfo(
                pid,
                libc::PROC_PIDTBSDINFO,
                0,
                (&mut info as *mut libc::proc_bsdinfo).cast(),
                size,
            );
            if n != size || info.pbi_status == libc::SZOMB {
                return None;
            }
            Some(info.pbi_start_tvsec * 1_000_000 + info.pbi_start_tvusec)
        }
    }

    /// Elsewhere the system is not asked: whether it runs, from the signal 0, and no start (a
    /// pid taken by another process is then not told apart).
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    pub(super) fn start_of(pid: i32) -> Option<u64> {
        // SAFETY: signal 0 only checks that the process exists.
        (unsafe { libc::kill(pid, 0) } == 0).then_some(0)
    }

    /// The memory of a process: on macOS its physical footprint, as the Activity Monitor's
    /// « Memory » column shows it (its own memory, compressed included, without the pages of
    /// the libraries it shares with others); elsewhere, and when macOS does not tell it, its
    /// resident memory (`rss`, which counts the shared pages in each process that maps them).
    fn memory(row: &Row) -> u64 {
        #[cfg(target_os = "macos")]
        if let Some(bytes) = footprint(row.pid) {
            return bytes;
        }
        row.rss * 1024
    }

    /// The physical footprint of `pid` (`proc_pid_rusage`, as `footprint(1)` reads it).
    #[cfg(target_os = "macos")]
    fn footprint(pid: i32) -> Option<u64> {
        // SAFETY: `info` is a plain C struct of the flavor asked, which the call fills.
        unsafe {
            let mut info: libc::rusage_info_v2 = std::mem::zeroed();
            let r = libc::proc_pid_rusage(
                pid,
                libc::RUSAGE_INFO_V2,
                (&mut info as *mut libc::rusage_info_v2).cast(),
            );
            (r == 0).then_some(info.ri_phys_footprint)
        }
    }

    /// The `pid ppid pgid stat rss time` lines of `ps`, zombies left out.
    pub(super) fn parse_ps(out: &str) -> Vec<Row> {
        out.lines()
            .filter_map(|line| {
                let mut f = line.split_whitespace();
                let pid = f.next()?.parse().ok()?;
                let ppid = f.next()?.parse().ok()?;
                let pgid = f.next()?.parse().ok()?;
                if f.next()?.starts_with('Z') {
                    return None;
                }
                let rss = f.next()?.parse().unwrap_or(0);
                let cpu = f.next().and_then(parse_cpu_time).unwrap_or_default();
                Some(Row {
                    pid,
                    ppid,
                    pgid,
                    rss,
                    cpu,
                })
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

    pub struct Sample;

    impl Sample {
        pub fn read() -> Self {
            Sample
        }
    }

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
        pub fn usage_in(&self, _sample: &Sample) -> Option<super::JobUsage> {
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

pub use imp::{Job, Sample};

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
    use super::imp::{parse_cpu_time, parse_ps, start_of, tree, Row};
    use super::*;
    use std::collections::HashMap;
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

    /// A machine: init (1); the app (50); its agent (100) and the agent's child (101) in the
    /// agent's group; a grandchild (102) that took a session of its own, a process (103) of that
    /// session whose parent ended (now 1's), a child of 102 (104) that leads a group of its own,
    /// and 104's child (105); a zombie of the agent's group (106); a stranger child of 1 (200) and
    /// one of the app's (201), the agent's sibling.
    const PS: &str = "\
    1     0     1 Ss     100   0:01.00
   50     1    50 Ss     200   0:03.00
  100    50   100 S     1000   0:01.00
  101   100   100 S+      24   0:00.50
  102   101   102 Ss     500   0:02.00
  103     1   102 S      300   0:00.25
  104   102   104 Ss      10   0:00.00
  105   104   104 S        5   1:00.00
  106   100   100 Z        0   0:00.00
  200     1   200 Ss   99999   9:00.00
  201    50   201 S    99999   9:00.00
 junk
";

    /// Every process of `PS` started at ten times its pid.
    fn started(pid: i32) -> Option<u64> {
        Some(pid as u64 * 10)
    }

    fn pids(rows: &[Row]) -> Vec<i32> {
        rows.iter().map(|r| r.pid).collect()
    }

    #[test]
    fn reads_the_processes_of_ps_without_the_zombies() {
        let rows = parse_ps(PS);
        assert_eq!(
            pids(&rows),
            vec![1, 50, 100, 101, 102, 103, 104, 105, 200, 201]
        );
        assert_eq!(
            rows[3],
            Row {
                pid: 101,
                ppid: 100,
                pgid: 100,
                rss: 24,
                cpu: Duration::from_millis(500),
            }
        );
    }

    #[test]
    fn the_tree_is_the_group_the_descendants_and_the_groups_they_lead() {
        let rows = parse_ps(PS);
        let t = tree(&rows, 100, Some(1000), &HashMap::new(), started);
        // Not init, the app, the zombie nor the strangers (one of the same parent as 103).
        assert_eq!(pids(&t.members), vec![100, 101, 102, 103, 104, 105]);
        assert!(t.group);
        assert_eq!(t.seen.get(&103), Some(&1030));
        let rss: u64 = t.members.iter().map(|r| r.rss).sum();
        assert_eq!(rss, 1000 + 24 + 500 + 300 + 10 + 5);
        let cpu: Duration = t.members.iter().map(|r| r.cpu).sum();
        assert_eq!(cpu, Duration::from_millis(63_750));
    }

    #[test]
    fn a_leader_whose_pid_another_process_took_is_not_the_jobs_nor_its_group() {
        let rows = parse_ps(PS);
        // The job's leader started at 7: 100 is another process now.
        let t = tree(&rows, 100, Some(7), &HashMap::new(), started);
        assert!(t.members.is_empty());
        assert!(!t.group);
    }

    #[test]
    fn a_leader_gone_leaves_its_group_and_what_descends_from_it() {
        let rows: Vec<Row> = parse_ps(PS).into_iter().filter(|r| r.pid != 100).collect();
        let t = tree(&rows, 100, Some(1000), &HashMap::new(), started);
        assert_eq!(pids(&t.members), vec![101, 102, 103, 104, 105]);
        assert!(t.group);
    }

    #[test]
    fn what_was_seen_stays_in_the_tree_while_it_runs_as_it_was_seen() {
        // 200 was seen in the tree (its parent ended since), 201 too but its pid went to
        // another process since: it started at 1, not 2010.
        let seen = HashMap::from([(200, 2000), (201, 1)]);
        let t = tree(&parse_ps(PS), 100, Some(1000), &seen, started);
        assert_eq!(pids(&t.members), vec![100, 101, 102, 103, 104, 105, 200]);
        assert!(!t.seen.contains_key(&201));
    }

    #[test]
    fn a_process_gone_meanwhile_is_not_noted() {
        // 105 ended between the reading and its start's.
        let t = tree(&parse_ps(PS), 100, Some(1000), &HashMap::new(), |pid| {
            (pid != 105).then(|| pid as u64 * 10)
        });
        assert!(t.members.iter().any(|r| r.pid == 105));
        assert!(!t.seen.contains_key(&105));
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

    /// Whether `pid` runs (a zombie, done but not yet reaped, does not).
    fn runs(pid: i32) -> bool {
        let out = std::process::Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .unwrap();
        let stat = String::from_utf8_lossy(&out.stdout);
        out.status.success() && !stat.trim().is_empty() && !stat.trim().starts_with('Z')
    }

    /// Waits, at most 5 s, until `pid` no longer runs.
    fn ends(pid: i32) -> bool {
        for _ in 0..250 {
            if !runs(pid) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        false
    }

    /// Kills, when a test ends whatever its end, the processes it started that still run as
    /// they started (a pid another process took since is left alone): one left would hold
    /// nextest.
    struct Reap(Vec<(i32, Option<u64>)>);
    impl Reap {
        fn new(pids: &[i32]) -> Self {
            Reap(pids.iter().map(|&p| (p, start_of(p))).collect())
        }
    }
    impl Drop for Reap {
        fn drop(&mut self) {
            for &(pid, start) in &self.0 {
                if start.is_some() && start_of(pid) == start {
                    // SAFETY: plain syscall, on a process this test started.
                    unsafe {
                        libc::kill(pid, libc::SIGKILL);
                    }
                }
            }
        }
    }

    /// An agent that starts, as a detached Node `spawn` or a command of Claude Code's Bash tool
    /// does, a program in a session of its own (`setsid`, by perl: macOS has no `setsid`
    /// command), which starts `sleep 60` and lives `lives` seconds: the pids of both, once the
    /// first left the agent's group.
    async fn agent_with_a_session_apart(lives: u32) -> (tokio::process::Child, Job, i32, i32) {
        let mut cmd = tokio::process::Command::new("sh");
        let apart = format!(
            r#"perl -MPOSIX -e '$| = 1; POSIX::setsid() or die; my $k = fork(); if ($k == 0) {{ exec("sleep", "60") }} print "$$ $k\n"; sleep {lives}' </dev/null 2>/dev/null &
wait"#
        );
        cmd.args(["-c", &apart])
            .stdout(std::process::Stdio::piped());
        isolate(&mut cmd);
        let mut child = cmd.spawn().unwrap();
        let job = Job::for_agent(&child).unwrap();
        let mut out = tokio::io::BufReader::new(child.stdout.take().unwrap());
        let mut line = String::new();
        tokio::io::AsyncBufReadExt::read_line(&mut out, &mut line)
            .await
            .unwrap();
        let mut pids = line.split_whitespace().map(|p| p.parse::<i32>().unwrap());
        let (apart, its_child) = (pids.next().unwrap(), pids.next().unwrap());
        (child, job, apart, its_child)
    }

    /// A stranger: `sleep 60`, child of the test as the agent is, in a group of its own.
    fn stranger() -> std::process::Child {
        let mut cmd = std::process::Command::new("sleep");
        cmd.arg("60");
        std::os::unix::process::CommandExt::process_group(&mut cmd, 0);
        cmd.spawn().unwrap()
    }

    #[tokio::test]
    async fn an_agent_counts_what_it_starts_in_a_session_apart() {
        let (_child, job, apart, its_child) = agent_with_a_session_apart(60).await;
        let _reap = Reap::new(&[apart, its_child]);
        let mut other = stranger();
        // SAFETY: plain syscall.
        assert_eq!(
            unsafe { libc::getpgid(apart) },
            apart,
            "perl leads its group"
        );
        let pids = job.pids();
        assert!(
            pids.contains(&(apart as u32)) && pids.contains(&(its_child as u32)),
            "{pids:?} lacks {apart} or {its_child}"
        );
        assert!(!pids.contains(&other.id()), "{pids:?} holds the stranger");
        // sh, perl and sleep, in one reading shared with other jobs: the stranger's has it alone.
        let sample = Sample::read();
        let u = job.usage_in(&sample).unwrap();
        assert_eq!(u.processes, 3);
        assert!(u.memory > 0);
        let theirs = Job::claiming(other.id() as i32, start_of(other.id() as i32));
        assert_eq!(theirs.usage_in(&sample).unwrap().processes, 1);
        std::mem::forget(theirs);
        assert_eq!(job.usage().unwrap().processes, 3);
        other.kill().unwrap();
        other.wait().unwrap();
    }

    #[tokio::test]
    async fn stopping_an_agent_stops_what_it_started_in_a_session_apart_and_nothing_else() {
        let (mut child, job, apart, its_child) = agent_with_a_session_apart(60).await;
        let _reap = Reap::new(&[apart, its_child]);
        let mut other = stranger();
        job.terminate();
        child.wait().await.unwrap();
        assert!(ends(apart), "{apart} survived the agent");
        assert!(ends(its_child), "{its_child} survived the agent");
        assert!(runs(other.id() as i32), "the stranger was killed");
        other.kill().unwrap();
        other.wait().unwrap();
    }

    #[tokio::test]
    async fn what_an_agent_started_apart_stays_its_own_once_its_parent_ended() {
        // perl ends after 2 s: its `sleep` goes to another parent, out of the agent's descent.
        let (mut child, job, apart, its_child) = agent_with_a_session_apart(2).await;
        let _reap = Reap::new(&[apart, its_child]);
        assert!(job.pids().contains(&(its_child as u32)));
        assert!(ends(apart));
        let pids = job.pids();
        assert!(
            pids.contains(&(its_child as u32)),
            "{pids:?} lost {its_child}"
        );
        drop(job);
        child.wait().await.unwrap();
        assert!(ends(its_child), "{its_child} survived the agent");
    }

    #[test]
    fn a_job_whose_pid_another_process_took_counts_and_kills_nothing() {
        let mut other = stranger();
        let pid = other.id() as i32;
        // The job's leader started at another time: `other` took its pid since.
        let job = Job::claiming(pid, start_of(pid).map(|s| s + 1));
        assert_eq!(job.usage().unwrap().processes, 0);
        assert!(job.pids().is_empty());
        job.terminate();
        drop(job);
        std::thread::sleep(Duration::from_millis(100));
        assert!(runs(pid), "the stranger was killed");
        other.kill().unwrap();
        other.wait().unwrap();
    }
}
