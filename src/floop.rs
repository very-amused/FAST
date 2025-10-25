#![allow(dead_code)]

use tokio::task::JoinHandle;
use tokio::runtime::{Runtime, Builder as RuntimeBuilder};
use parking_lot::{Mutex,MutexGuard};
use std::{mem};
use std::sync::Arc;

use crate::server::FastServer;

/// An async-loop emulator providing a mutex over FAST operations.
pub struct FastLoop {
	runtime: Arc<Runtime>,
	lock: Mutex<()>, // lock that ensures only 1 callback is run at a time
	callback_task: Option<JoinHandle<()>>
}


impl FastLoop {
	pub fn run_callback<F>(&mut self, callback: F)
	where F: FnOnce() -> () + Send + 'static {
		todo!()
	}
}

/// Create a new FastLoop ready for use.
#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_new(srv_ptr: *mut FastServer) -> *mut FastLoop {
	let srv = unsafe { &*srv_ptr };

	let floop = Box::new(FastLoop {
		runtime: srv.0.clone(),
		lock: Mutex::new(()),
		callback_task: None
	});

	Box::leak(floop)
}

#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_free(floop_ptr: *mut FastLoop) {
	let mut floop = unsafe { Box::from_raw(floop_ptr) };

	// Abort any callback that's running
	let guard = floop.lock.lock();
	if let Some(thr) = floop.callback_task {
		thr.abort();
		floop.callback_task = None;
	}

	// floop gets dropped
}

#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_lock(floop_ptr: *mut FastLoop) {
	let guard = unsafe { (&*floop_ptr).lock.lock() };

	mem::forget(guard);
}

#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_unlock(floop_ptr: *mut FastLoop) {
	let floop = unsafe { &*floop_ptr };
	unsafe { floop.lock.force_unlock() };
}
