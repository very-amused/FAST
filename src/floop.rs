#![allow(dead_code)]

use parking_lot::Mutex;
use std::mem;

/// An async-loop emulator providing a mutex over FAST operations.
pub struct FastLoop {
	lock: Mutex<()>
}

/// Create a new FastLoop ready for use.
#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_new() -> *mut FastLoop {
	let floop = Box::new(FastLoop {
		lock: Mutex::new(())
	});
	Box::leak(floop)
}

#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_free(floop_ptr: *mut FastLoop) {
	let floop = unsafe { Box::from_raw(floop_ptr) };

	// floop gets dropped
}

#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_lock(floop_ptr: *mut FastLoop) {
	let floop = unsafe { &*floop_ptr };
	let guard = floop.lock.lock();

	mem::forget(guard);
}

#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_unlock(floop_ptr: *mut FastLoop) {
	let floop = unsafe { &*floop_ptr };
	unsafe { floop.lock.force_unlock() };
}
