#![allow(dead_code)]

use tokio::task::JoinHandle;
use tokio::runtime::Runtime;
use parking_lot::Mutex;
use std::mem;
use std::sync::Arc;

use crate::server::FastServer;

/// An async-loop emulator providing a mutex over FAST operations.
pub struct FastLoop {
	pub runtime: Arc<Runtime>,
	lock: Mutex<()>, // lock that ensures only 1 callback is run at a time
	callback_task: Option<JoinHandle<()>>
}

// Don't try this at home
#[derive(Clone, Copy)]
pub struct FastLoopPtr(pub *mut FastLoop);
unsafe impl Send for FastLoopPtr {}

impl FastLoop {
	pub fn run_callback<F>(&mut self, callback: F)
	where F: FnOnce() -> () + Send + 'static{
		// Lock the loop
		let guard = self.lock.lock();
		
		// move guard into the spawned callback
		mem::forget(guard);
		// get a mutex ref w/o borrow checking
		let mutex = unsafe { (&self.lock as *const Mutex<()>).as_ref() }.unwrap();
		self.callback_task = Some(self.runtime.spawn_blocking(move || {
			callback();
			unsafe { mutex.force_unlock() };
		}));
	}
}

/// Create a new FastLoop ready for use.
#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_new(srv_ptr: *mut FastServer) -> *mut FastLoop {
	let srv = unsafe { &*srv_ptr };

	let floop = Box::new(FastLoop {
		runtime: srv.runtime.clone(),
		lock: Mutex::new(()),
		callback_task: None
	});

	Box::leak(floop)
}

#[unsafe(no_mangle)]
pub extern "C" fn FastLoop_free(floop_ptr: *mut FastLoop) {
	let floop = unsafe { Box::from_raw(floop_ptr) };

	drop(floop)
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
