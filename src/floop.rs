#![allow(dead_code)]

use tokio::task::JoinHandle;
use tokio::runtime::{Runtime, Builder as RuntimeBuilder};
use parking_lot::{Mutex,MutexGuard};
use std::{mem};
use std::sync::Arc;

use crate::server::FastServer;

/// An async-loop emulator providing a mutex over FAST operations.
pub struct FastLoop {
	data: Mutex<FastLoopData>
}

struct FastLoopData {
	runtime: Arc<Runtime>,
	callback_task: Option<JoinHandle<()>>
}

/// Create a new FastLoop ready for use.
#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_new(srv_ptr: *mut FastServer) -> *mut FastLoop {
	let srv = unsafe { &*srv_ptr };

	let inner = FastLoopData {
		runtime:  srv.0.clone(),
		callback_task: None
	};

	let floop = Box::new(FastLoop {
		data: Mutex::new(inner)
	});

	Box::leak(floop)
}

#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_free(floop_ptr: *mut FastLoop) {
	let floop = unsafe { Box::from_raw(floop_ptr) };

	// Abort any callback that's running
	let mut inner = floop.data.lock();
	if let Some(thr) = &inner.callback_task {
		thr.abort();
		inner.callback_task = None;
	}

	// floop gets dropped
}

#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_lock(floop_ptr: *mut FastLoop) {
	let guard = unsafe { (&*floop_ptr).data.lock() };

	mem::forget(guard);
}

#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_unlock(floop_ptr: *mut FastLoop) {
	let floop = unsafe { &*floop_ptr };
	unsafe { floop.data.force_unlock() };
}
