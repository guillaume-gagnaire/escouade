//! The processor time of the calling thread: what a budget counts when a loaded machine must not eat it (the turn of a
//! WASM plugin, ESC-650). A thread the system leaves waiting for a processor spends none.

use std::time::Duration;

/// The processor time the calling thread spent so far (user and system), or none when the system does not say it.
pub fn thread() -> Option<Duration> {
    imp::thread()
}

#[cfg(windows)]
mod imp {
    use std::time::Duration;
    use windows_sys::Win32::Foundation::FILETIME;
    use windows_sys::Win32::System::Threading::{GetCurrentThread, GetThreadTimes};

    pub fn thread() -> Option<Duration> {
        let zero = FILETIME {
            dwLowDateTime: 0,
            dwHighDateTime: 0,
        };
        let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
        // SAFETY: the pseudo-handle of the calling thread, and four FILETIME alive for the call.
        let ok = unsafe {
            GetThreadTimes(
                GetCurrentThread(),
                &mut created,
                &mut exited,
                &mut kernel,
                &mut user,
            )
        };
        if ok == 0 {
            return None;
        }
        // In units of 100 ns.
        let units = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
        Some(Duration::from_nanos(
            (units(kernel) + units(user)).saturating_mul(100),
        ))
    }
}

#[cfg(unix)]
mod imp {
    use std::time::Duration;

    pub fn thread() -> Option<Duration> {
        let mut t = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        // SAFETY: a timespec alive for the call.
        if unsafe { libc::clock_gettime(libc::CLOCK_THREAD_CPUTIME_ID, &mut t) } != 0 {
            return None;
        }
        Some(Duration::new(t.tv_sec as u64, t.tv_nsec as u32))
    }
}

#[cfg(not(any(windows, unix)))]
mod imp {
    pub fn thread() -> Option<std::time::Duration> {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A thread that works spends processor time; one that sleeps spends almost none, however long it sleeps.
    #[test]
    fn a_thread_that_works_spends_its_time_and_one_that_sleeps_does_not() {
        let start = thread().expect("this system says it");
        let mut x = 0u64;
        let began = std::time::Instant::now();
        while began.elapsed() < Duration::from_millis(200) {
            x = x.wrapping_mul(31).wrapping_add(7);
        }
        std::hint::black_box(x);
        let worked = thread().unwrap() - start;
        assert!(worked > Duration::from_millis(1), "{worked:?}");
        let before = thread().unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let slept = thread().unwrap() - before;
        assert!(slept < Duration::from_millis(100), "{slept:?}");
    }
}
