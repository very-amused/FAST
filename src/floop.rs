#![allow(dead_code)]

use tokio::task::JoinHandle;
use tokio::runtime::{Runtime as TokioRuntime, Builder as RuntimeBuilder};
use parking_lot::Mutex;
use std::{io,mem};
use std::sync::Arc;

/// An async-loop emulator providing a mutex over FAST operations.
pub struct FastLoop {
	runtime: Arc<TokioRuntime>,
	lock: Mutex<()>,

	// Callback handling
	// Thread for an active callback routine. Only 1 at a time!
	// When a callback is spawned, call [callback_wait] to await its JoinHandle before unlocking the
	// loop.
	callback_task: Option<JoinHandle<()>>
}

/// Create a new FastLoop ready for use.
#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_new() -> *mut FastLoop {
	// Initialize Tokio async runtime
	let runtime = match new_runtime() {
		Ok(rt) => rt,
		Err(err) => {
			eprintln!("Failed to initialize Tokio runtime: {}", err);
			return std::ptr::null_mut();
		}
	};

	let floop = Box::new(FastLoop {
		runtime: Arc::new(runtime),
		lock: Mutex::new(()),
		callback_task: None
	});

	Box::leak(floop)
}

/// Initialize a Tokio runtime capable of powering a FastStream
fn new_runtime() -> io::Result<TokioRuntime>{
	RuntimeBuilder::new_multi_thread()
		.worker_threads(4) // stream_task + callback_task + growing room
		.enable_all()
		.build()
}

#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_free(floop_ptr: *mut FastLoop) {
	let _floop = unsafe { Box::from_raw(floop_ptr) };

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
