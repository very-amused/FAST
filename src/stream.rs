#![allow(dead_code)]
use tokio::{task, time};
use tokio::runtime::Runtime;
use std::borrow::BorrowMut;
use std::ffi::{c_uchar, c_void};
use std::os::raw::c_int;
use std::ptr::null_mut;
use std::slice;
use std::sync::Arc;

mod buffer;
mod error;

use crate::floop::{FastLoop, FastLoopPtr, FastLoop_lock, FastLoop_unlock};
use crate::sys::{self, FastStream_write_callback, FastStreamSettings};
use crate::thread_flag::ThreadFlag;
use crate::userdata::Userdata;
use buffer::FastStreamBuffer;

/// An audio sink for FAST
pub struct FastStream {
	floop: FastLoopPtr,
	runtime: Arc<Runtime>, // Runtime responsible for stream_task, held for Arc

	/// Buffer that
	/// - [stream_task] reads from
	/// - [callback_task] writes to
	buffer: FastStreamBuffer,

	/// Thread that consumes audio frames
	stream_task: Option<task::JoinHandle<()>>,
	paused: ThreadFlag<bool>, // Controls + indicates whether the stream is paused

	// Callback for writing audio bytes to [buffer]
	write_cb: FastStream_write_callback,
	write_cb_userdata: Userdata
}

#[derive(Clone, Copy)]
struct FastStreamPtr(*mut FastStream);
unsafe impl Send for FastStreamPtr {}

// Conversion between opaque and non-opaque FastStream pointers
impl From<FastStreamPtr> for *mut sys::FastStream {
	fn from(value: FastStreamPtr) -> Self {
		value.0 as *mut c_void as *mut sys::FastStream
	}
}
impl Into<FastStreamPtr> for *mut sys::FastStream {
	fn into(self) -> FastStreamPtr {
		FastStreamPtr(self as *mut c_void as *mut FastStream)
	}
}


#[unsafe(no_mangle)]
pub extern "C" fn FastStream_new(loop_ptr: *mut FastLoop, settings_ptr: *const FastStreamSettings) -> *mut FastStream {
	let floop = unsafe { &*loop_ptr };
	let settings = unsafe { &*settings_ptr };

	// Create stream
	let stream = Box::new(FastStream {
		floop: FastLoopPtr(loop_ptr),
		runtime: floop.runtime.clone(),
		buffer: FastStreamBuffer::new(settings),

		stream_task: None,
		paused: ThreadFlag::new(true),

		write_cb: None,
		write_cb_userdata: Userdata(null_mut())
	});

	// Start stream_task in a paused state
	let stream_ptr: *mut FastStream = Box::leak(stream);
	let stream = unsafe { &mut *stream_ptr };
	stream.stream_task = Some(stream.runtime.spawn(
		FastStream_loop(FastStreamPtr(stream_ptr))));

	stream_ptr
}

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_free(stream_ptr: *mut FastStream) {
	let mut stream = unsafe { Box::from_raw(stream_ptr) };
	if let Some(thr) = stream.stream_task {
		thr.abort();
		stream.stream_task = None;
	}

	drop(stream)
}

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_play(stream_ptr: *mut FastStream, play: bool) -> c_int {
	// Spawn stream task if needed (should not be needed in general)
	let stream = unsafe { (*stream_ptr).borrow_mut() };
	if stream.stream_task.is_none() {
		let handle = stream.runtime.spawn(
			FastStream_loop(FastStreamPtr(stream_ptr)));
		stream.stream_task = Some(handle);
	}

	FastLoop_lock(stream.floop.0);
	{
		let paused = stream.paused.get();

		// Debounce play signals
		if (play && !paused) || (!play && paused) {
			return 0;
		}

		stream.runtime.block_on(stream.paused.set(!play));
	}
	FastLoop_unlock(stream.floop.0);

	return 0;
}

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_set_write_cb(stream_ptr: *mut FastStream, cb: FastStream_write_callback, userdata: *mut c_void) {
	let stream = unsafe { (*stream_ptr).borrow_mut() };

	stream.write_cb_userdata.0 = userdata;
	stream.write_cb = cb;
}

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_begin_write(stream_ptr: *const FastStream, n_bytes_ptr: *mut usize) -> c_int {
	let stream = unsafe { &*stream_ptr };
	let n_bytes = unsafe { &mut *n_bytes_ptr };

	// If the caller has nothing to write but is still preparing to write, that's an error they should handle
	if *n_bytes == 0 {
		return 1;
	}

	// Get buffer write capacity
	let write_cap = stream.buffer.write_capacity();
	// If the caller is going to overflow our buffer, reduce their write size
	if write_cap < *n_bytes {
		*n_bytes = write_cap;
	}

	return 0;
}

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_write(stream_ptr: *mut FastStream, src: *const c_uchar, n: usize) -> c_int {
	let stream = unsafe { &mut *stream_ptr };

	// Construct slice and write to our stream's buffer
	let data = unsafe { slice::from_raw_parts(src, n) };
	if let Err(e) = stream.buffer.write(data) {
		eprintln!("FastStream write error: {}", e);
		return 1;
	}

	return 0;
}

async fn FastStream_loop(stream_ptr: FastStreamPtr) {
	let stream = unsafe { (*stream_ptr.0).borrow_mut() };

	// Set up interval to read frames on
	const READ_INTERVAL_MS: u64 = FastStreamBuffer::read_interval_ms();
	const READ_INTERVAL_DURATION: time::Duration = time::Duration::from_millis(READ_INTERVAL_MS); // i.e reads of 441 frames/10ms @ 44.1khz
	let mut interval = time::interval(READ_INTERVAL_DURATION);
	// CRITICAL: align ticks to play/pause event
	interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

	// Event loop
	loop {
		// Wait for tick or pause signal
		tokio::select! {
			_ = interval.tick() => if !stream.paused.get() {
				handle_reads(stream).await;
				handle_writes(stream).await;
			},
			pause = stream.paused.get_new() => if pause {
				// Wait until unpaused
				while stream.paused.get_new().await == true {}
			}
		}
	}
}

// Handle reads for each tick
async fn handle_reads(stream: &mut FastStream) {
	let read_size = stream.buffer.read_size();
	if let Err(e) = stream.buffer.read(read_size) {
		eprintln!("FastStream_loop->handle_reads: {}", e);
	}
}

// Handle writes (via callback) for each tick
async fn handle_writes(stream: &mut FastStream) {
	let stream_ptr = FastStreamPtr(std::ptr::from_mut(stream));
	let n_bytes = stream.buffer.write_capacity();
	// We want to write half as often as we read (~50 writes/s)
	if n_bytes < 2 * stream.buffer.read_size() {
		return;
	}

	// Run callback on the FastLoop (which handles locking)
	let floop = unsafe { &mut *stream.floop.0 };
	floop.run_callback(move || {
		let stream = unsafe { &mut *stream_ptr.0 }; // callback lifetime
		if let Some(write_cb) = stream.write_cb {
			unsafe {
				let userdata = stream.write_cb_userdata.0;
				write_cb(stream_ptr.into(), n_bytes, userdata);
			}
		}
	});
}
