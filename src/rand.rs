//! Derived from <https://github.com/madsim-rs/madsim/blob/main/madsim/src/sim/rand.rs>

use std::{
    borrow::BorrowMut,
    fs::File,
    io::{self, Read},
    sync::{Arc, Mutex, MutexGuard, OnceLock},
};

use rand::{Rng, rngs::StdRng};

static RNG_CELL: OnceLock<Arc<Mutex<StdRng>>> = OnceLock::new();

pub fn set_rng(rng: StdRng) {
    RNG_CELL
        .set(Arc::new(Mutex::new(rng)))
        .expect("Single init")
}

pub fn try_rng() -> Option<MutexGuard<'static, StdRng>> {
    RNG_CELL.get().map(|m| m.lock().expect("RNG lock"))
}

pub fn get_rng() -> MutexGuard<'static, StdRng> {
    try_rng().expect("RNG init")
}

fn fill_with_dev_urandom(dest: &mut [u8]) -> io::Result<()> {
    let mut file = File::open("/dev/urandom")?;
    file.read_exact(dest)?;
    Ok(())
}

#[unsafe(no_mangle)]
#[inline(never)]
unsafe extern "C" fn getrandom(buf: *mut u8, buflen: usize, _flags: u32) -> isize {
    // <https://man7.org/linux/man-pages/man2/getrandom.2.html>
    if !buf.is_null() && buflen > 0 {
        let dest = unsafe { std::slice::from_raw_parts_mut(buf, buflen) };
        match try_rng() {
            Some(mut rng) => {
                rng.borrow_mut().fill_bytes(dest);
            }
            None => {
                // for call sites (e.g. test runner, getting random seed if not set in env var)
                // before the test has set the RNG.
                if fill_with_dev_urandom(dest).is_err() {
                    return -1;
                }
            }
        }
        buflen as isize
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
#[cfg(target_os = "macos")]
#[inline(never)]
unsafe extern "C" fn CCRandomGenerateBytes(buf: *mut u8, buflen: usize) -> i32 {
    // For Mac OS
    // - <https://blog.xoria.org/randomness-on-apple-platforms/>
    // - <https://linear.app/streamstore/issue/S2-597/fix-non-determinism-in-dst>
    if unsafe { getrandom(buf, buflen, 0) } as i32 != -1 {
        0
    } else {
        -1
    }
}

#[unsafe(no_mangle)]
#[inline(never)]
unsafe extern "C" fn getentropy(buf: *mut u8, buflen: usize) -> i32 {
    // <https://man7.org/linux/man-pages/man3/getentropy.3.html>
    if buflen > 256 {
        return -1;
    }
    match unsafe { getrandom(buf, buflen, 0) } {
        -1 => -1,
        _ => 0,
    }
}
