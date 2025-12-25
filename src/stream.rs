#![allow(dead_code)]
use tokio::{spawn, task, time};
use tokio::runtime::Runtime;
use std::borrow::BorrowMut;
use std::ffi::c_void;
use std::mem;
use std::os::raw::c_int;
use std::ptr::null_mut;
use std::sync::Arc;

mod callback;
mod buffer;

use crate::floop::{FastLoop, FastLoopPtr, FastLoop_lock, FastLoop_unlock};
use crate::sys::{self, FastStreamSettings, FastStream_write_callback};
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
		FastStream_routine(FastStreamPtr(stream_ptr))));

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
			FastStream_routine(FastStreamPtr(stream_ptr)));
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

async fn FastStream_routine(stream_ptr: FastStreamPtr) {
	let stream = unsafe { (*stream_ptr.0).borrow_mut() };
	let buffer = &mut stream.buffer;

	// Set up interval to read frames on
	const READ_INTERVAL_MS: u64 = FastStreamBuffer::read_interval_ms();
	const READ_INTERVAL_DURATION: time::Duration = time::Duration::from_millis(READ_INTERVAL_MS); // i.e reads of 441 frames/10ms @ 44.1khz
	let mut interval = time::interval(READ_INTERVAL_DURATION);
	// CRITICAL: align ticks to play/pause event
	interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

	// Compute read size (bytes) for each tick
	let read_size = buffer.read_size();

	let mut n_ticks: usize = 0;

	// Event loop
	loop {
		// Wait for tick or pause signal
		tokio::select! {
			_ = interval.tick() => if !stream.paused.get() {
				handle_reads(stream, read_size).await;
				handle_writes(stream_ptr, read_size).await;
				n_ticks += 1;
				//eprintln!("{} ticks elapsed ({} bytes read)\n", n_ticks, n_ticks * read_size);
			},
			pause = stream.paused.get_new() => if pause {
				// Wait until unpaused
				while stream.paused.get_new().await == true {}
			}
		}
	}
}

// Handle reads for each tick
async fn handle_reads(stream: &mut FastStream, read_size: usize) {
	let buffer = &mut stream.buffer;

	// Read {read_size} bytes each tick
	if let Err(e) = buffer.read(read_size) {
		eprintln!("Read error: {}", e);
	}
}

// Handle writes (via callback) for each tick
async fn handle_writes(stream_ptr: FastStreamPtr, read_size: usize) {
	let stream = unsafe { &mut (*stream_ptr.0) };
	let buffer = &mut stream.buffer;

	// We request writes of 50ms of audio at a time
	// The user can then write up to 50ms each time in the callback
	let write_size = read_size * 5;
	let write_cap = buffer.write_capacity();
	if write_cap < write_size {
		return;
	}

	// Compute size for this write
	let n_bytes = write_size * (write_cap / write_size);

	// Spawn callback
	let floop = unsafe { &mut *stream.floop.0 };
	floop.run_callback(move || {
		if let Some(write_cb) = stream.write_cb {
			unsafe { write_cb(stream_ptr.into(), n_bytes, stream.write_cb_userdata.0) };
		}
	});
}
