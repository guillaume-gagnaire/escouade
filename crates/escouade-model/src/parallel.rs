//! How many tickets of a Kanban run at once (ESC-620): the one bound of « En parallèle », that the board, the MCP
//! (`start_ticket`, `set_board`), the window, the policies (`projects.maxParallel`, the managed device's and the team's
//! `.escouade.yml`) and the remote API all go by. Nothing else writes `clamp(1, 6)`.
//!
//! - **On a desktop**, up to [`DESKTOP_MAX`]; **on a server** (`escouade-server`, and a server's space seen from a
//!   client: its core answers), up to [`SERVER_MAX`].
//! - **Beyond [`WARN_ABOVE`]**, the window says what this machine holds ([`Machine::fits`]: its cores, its memory, and
//!   the room an agent takes, measured on Escouade's own repository: [`AGENT_MB`], [`AGENT_CORES`]) and that the
//!   account's quota goes faster: an indication, never a refusal.
//! - **A ceiling** may be imposed (`projects.maxParallel`, rank 5 or 6 of the locks): the bound is then the lower.

use serde::Serialize;

/// The fewest tickets at once.
pub const MIN: u32 = 1;
/// The most on a desktop.
pub const DESKTOP_MAX: u32 = 16;
/// The most on a server.
pub const SERVER_MAX: u32 = 64;
/// Beyond it, the window says what the machine holds and that the quota goes faster.
pub const WARN_ABOVE: u32 = 6;

/// The memory an agent of a Kanban takes on average, in MB, measured on this repository (ESC-620, Windows 11, 32 cores
/// and 64 GB, 2026-11-22), the private memory of each process tree sampled every 2 s: Claude Code itself (about 0.3 GB),
/// a compile of Escouade (`npx tauri build --debug --no-bundle`, its crate `escouade` made again: 4.7 GB on average,
/// 14 to 17 GB at its peak) and an end-to-end test (the app and its WebView: 0.6 GB on average, 1 GB at its peak):
/// 6 GB, rounded up. The peaks of several agents seldom meet.
pub const AGENT_MB: u64 = 6_144;
/// The cores an agent keeps busy on average over the same runs (their CPU time over their wall time: 1.2), rounded up.
pub const AGENT_CORES: u32 = 2;

/// The most tickets at once on this kind of machine, before any ceiling.
pub fn limit(server: bool) -> u32 {
    if server {
        SERVER_MAX
    } else {
        DESKTOP_MAX
    }
}

/// The most tickets at once: the machine's limit, lowered by an imposed ceiling (never below [`MIN`]).
pub fn max(server: bool, ceiling: Option<u32>) -> u32 {
    let limit = limit(server);
    ceiling.map_or(limit, |c| c.min(limit)).max(MIN)
}

/// `n` within the bound: what the board starts by.
pub fn clamp(n: u32, server: bool, ceiling: Option<u32>) -> u32 {
    n.clamp(MIN, max(server, ceiling))
}

/// Whether `n` may be set as is (`set_board`, the window): within [`MIN`] and [`max`].
pub fn allowed(n: u32, server: bool, ceiling: Option<u32>) -> bool {
    (MIN..=max(server, ceiling)).contains(&n)
}

/// What a machine has, for the warning beyond [`WARN_ABOVE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Machine {
    pub cores: u32,
    /// Its physical memory, in MB (0 when it could not be read).
    pub memory_mb: u64,
}

impl Machine {
    /// This machine.
    pub fn this() -> Self {
        Machine {
            cores: std::thread::available_parallelism().map_or(1, |n| n.get() as u32),
            memory_mb: memory_mb().unwrap_or(0),
        }
    }

    /// About how many agents that compile and run tests it holds at once: what its memory and its cores both allow
    /// (its cores alone when its memory is not known), one at least.
    pub fn fits(&self) -> u32 {
        let by_cores = self.cores / AGENT_CORES;
        let by_memory = if self.memory_mb == 0 {
            u32::MAX
        } else {
            u32::try_from(self.memory_mb / AGENT_MB).unwrap_or(u32::MAX)
        };
        by_cores.min(by_memory).max(1)
    }
}

/// The physical memory, in MB.
#[cfg(windows)]
fn memory_mb() -> Option<u64> {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut status = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..unsafe { std::mem::zeroed() }
    };
    // SAFETY: `status` is a MEMORYSTATUSEX whose dwLength is set, as the call asks.
    let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
    (ok != 0).then_some(status.ullTotalPhys / (1024 * 1024))
}

/// The physical memory, in MB: `MemTotal` of `/proc/meminfo` (in kB).
#[cfg(any(target_os = "linux", target_os = "android"))]
fn memory_mb() -> Option<u64> {
    let text = std::fs::read_to_string("/proc/meminfo").ok()?;
    meminfo_mb(&text)
}

/// `MemTotal` of a `/proc/meminfo`, in MB.
#[cfg_attr(
    not(any(target_os = "linux", target_os = "android", test)),
    allow(dead_code)
)]
fn meminfo_mb(text: &str) -> Option<u64> {
    let line = text.lines().find(|l| l.starts_with("MemTotal:"))?;
    let kb: u64 = line.split_whitespace().nth(1)?.parse().ok()?;
    Some(kb / 1024)
}

/// The physical memory, in MB: `hw.memsize` (in bytes).
#[cfg(any(target_os = "macos", target_os = "ios"))]
fn memory_mb() -> Option<u64> {
    let mut bytes: u64 = 0;
    let mut len = std::mem::size_of::<u64>();
    // SAFETY: the name is a C string; `bytes` and `len` are the buffer of a u64 and its size, as sysctlbyname asks.
    let rc = unsafe {
        libc::sysctlbyname(
            c"hw.memsize".as_ptr(),
            (&mut bytes as *mut u64).cast(),
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    (rc == 0).then_some(bytes / (1024 * 1024))
}

#[cfg(not(any(
    windows,
    target_os = "linux",
    target_os = "android",
    target_os = "macos",
    target_os = "ios"
)))]
fn memory_mb() -> Option<u64> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixteen_on_a_desktop_sixty_four_on_a_server() {
        assert_eq!(max(false, None), 16);
        assert_eq!(max(true, None), 64);
        assert_eq!(clamp(20, false, None), 16);
        assert_eq!(clamp(20, true, None), 20);
        assert_eq!(clamp(100, true, None), 64);
        assert_eq!(clamp(0, true, None), 1);
        assert!(allowed(16, false, None) && !allowed(17, false, None));
        assert!(allowed(64, true, None) && !allowed(65, true, None));
        assert!(!allowed(0, true, None));
    }

    #[test]
    fn a_ceiling_lowers_the_bound_never_raises_it() {
        assert_eq!(max(false, Some(8)), 8);
        assert_eq!(max(true, Some(8)), 8);
        assert_eq!(max(false, Some(40)), 16);
        assert_eq!(max(true, Some(0)), 1);
        assert_eq!(clamp(12, true, Some(8)), 8);
        assert!(!allowed(9, true, Some(8)));
    }

    #[test]
    fn what_a_machine_holds() {
        // 32 cores and 128 GB: the cores say 16, the memory 21.
        let big = Machine {
            cores: 32,
            memory_mb: 128 * 1024,
        };
        assert_eq!(big.fits(), 16);
        // 16 cores and 16 GB: the memory says 2.
        let small = Machine {
            cores: 16,
            memory_mb: 16 * 1024,
        };
        assert_eq!(small.fits(), 2);
        // Memory unknown: the cores alone; one at least.
        assert_eq!(
            Machine {
                cores: 8,
                memory_mb: 0
            }
            .fits(),
            4
        );
        assert_eq!(
            Machine {
                cores: 1,
                memory_mb: 1024
            }
            .fits(),
            1
        );
    }

    #[test]
    fn the_memory_of_proc_meminfo() {
        let text = "MemTotal:       32768000 kB\nMemFree:         1000 kB\n";
        assert_eq!(meminfo_mb(text), Some(32000));
        assert_eq!(meminfo_mb("nothing"), None);
        assert!(Machine::this().cores >= 1);
    }
}
