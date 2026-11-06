//! Derived from <https://github.com/madsim-rs/madsim/blob/main/madsim/src/sim/time/system_time.rs>

use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

static USE_SIM_CLOCKS: AtomicBool = AtomicBool::new(false);

/// Scopes checking for Tokio runtime.
///
/// Otherwise `tokio::runtime::Handle::try_current()` as process is tearing down can result in a
/// SIGTRAP: "global allocator may not use TLS"
pub struct SimClocksGuard(());

impl SimClocksGuard {
    pub fn init() -> Self {
        USE_SIM_CLOCKS.store(true, Ordering::Release);
        Self(())
    }
}

impl Drop for SimClocksGuard {
    fn drop(&mut self) {
        USE_SIM_CLOCKS.store(false, Ordering::Release);
    }
}

/// Based on innards of [std::time::Instant], which [tokio::time::Instant] wraps.
#[derive(Debug, Copy, Clone)]
struct StdTimespec {
    tv_sec: u64,
    tv_nsec: u32,
}

impl From<StdTimespec> for libc::timespec {
    fn from(value: StdTimespec) -> Self {
        Self {
            tv_sec: value.tv_sec as i64,
            tv_nsec: value.tv_nsec as libc::c_long,
        }
    }
}

fn turmoil_since_epoch() -> libc::timespec {
    let since_epoch = turmoil::since_epoch().unwrap_or(Duration::ZERO);
    libc::timespec {
        tv_sec: since_epoch.as_secs() as i64,
        tv_nsec: since_epoch.subsec_nanos() as libc::c_long,
    }
}

unsafe fn tokio_instant_now() -> libc::timespec {
    let instant = tokio::time::Instant::now();
    let ts: StdTimespec = unsafe { std::mem::transmute(instant) };
    ts.into()
}

#[unsafe(no_mangle)]
#[inline(never)]
unsafe extern "C" fn clock_gettime(
    clockid: libc::clockid_t,
    tp: *mut libc::timespec,
) -> libc::c_int {
    // <https://man7.org/linux/man-pages/man3/clock_gettime.3.html>
    if USE_SIM_CLOCKS.load(Ordering::Acquire) && tokio::runtime::Handle::try_current().is_ok() {
        let timespec = match clockid {
            libc::CLOCK_REALTIME => Some(turmoil_since_epoch()),
            #[cfg(target_os = "linux")]
            libc::CLOCK_REALTIME_COARSE => Some(turmoil_since_epoch()),
            libc::CLOCK_MONOTONIC | libc::CLOCK_MONOTONIC_RAW => {
                Some(unsafe { tokio_instant_now() })
            }
            #[cfg(target_os = "linux")]
            libc::CLOCK_MONOTONIC_COARSE | libc::CLOCK_BOOTTIME => {
                Some(unsafe { tokio_instant_now() })
            }
            #[cfg(target_os = "macos")]
            libc::CLOCK_UPTIME_RAW => Some(unsafe { tokio_instant_now() }),
            _ => {
                eprintln!(
                    "Unsupported clock, real implementation will be used -- clock_gettime({clockid:?})"
                );
                None
            }
        };
        if let Some(timespec) = timespec {
            unsafe { tp.write(timespec) };
            return 0;
        }
    }

    // In the course of a DST, a few notable callers of clock_gettime still need handling here:
    //  - mimalloc
    //  - tracing
    //  - fastrand uses the current time as an input to its hasher, even if setting a seed
    // While the latter two should be deterministic in our use (when and how many times invoked),
    // mimalloc might not be. If it were deterministic, we could use some atomic counter as a
    // spoofed clock, but for now, simply returning the Unix epoch seems fine.
    unsafe {
        tp.write(libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        });
    }
    0
}
