#![allow(dead_code)]
use tokio::{spawn, task, time};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};
use std::borrow::BorrowMut;
use std::ffi::c_void;
use std::{io, mem};
use std::os::raw::c_int;
use std::ptr::null_mut;
use std::sync::Arc;
use parking_lot::Mutex;

mod callback;
mod buffer;

use crate::sys::{self, FastStreamSettings, FastStream_write_callback};
use crate::thread_flag::ThreadFlag;
use crate::userdata::Userdata;
use buffer::FastStreamBuffer;

/// An audio sink for FAST
pub struct FastStream {
	runtime: Arc<Runtime>, // We need to hold this to keep stream_task valid
	/// Buffer that
	/// - [stream_task] reads from
	/// - [callback_task] writes to
	buffer: FastStreamBuffer,

	/// Thread that consumes audio frames
	stream_task: Option<task::JoinHandle<()>>,
	paused: ThreadFlag<bool>, // Controls + indicates whether the stream is paused



	// Callbacks
	/// Thread that runs callback routines
	callback_task: Option<task::JoinHandle<()>>,
	/// Lock to ensure we run one callback at a time
	/// Must be acquired when spawning a callback task
	callback_lock: Mutex<()>,
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

/// Initialize a Tokio runtime capable of powering a FastStream
fn new_runtime() -> io::Result<Runtime>{
	RuntimeBuilder::new_multi_thread()
		.worker_threads(2) // stream_task + callback_task
		.enable_all()
		.build()
}

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_new(settings_ptr: *const FastStreamSettings) -> *mut FastStream {
	let settings = unsafe { &*settings_ptr };

	// Initialize Tokio async runtime
	let runtime = match new_runtime() {
		Ok(rt) => rt,
		Err(err) => {
			eprintln!("Failed to initialize Tokio runtime: {}", err);
			return std::ptr::null_mut();
		}
	};

	// Create stream
	let stream = Box::new(FastStream {
		runtime,
		buffer: FastStreamBuffer::new(settings),

		stream_task: None,
		paused: ThreadFlag::new(true),

		callback_task: None,
		callback_lock: Mutex::new(()),
		write_cb: None,
		write_cb_userdata: Userdata(null_mut())
	});
	Box::leak(stream)
}

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_free(stream_ptr: *mut FastStream) {
	let stream = unsafe { Box::from_raw(stream_ptr) };
	if let Some(thr) = stream.stream_task {
		thr.abort();
	}
	if let Some(thr) = stream.callback_task {
		thr.abort();
	}

	// stream gets dropped
}

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_start(stream_ptr: *mut FastStream) -> c_int {
	// Spawn stream task
	let stream = unsafe { (*stream_ptr).borrow_mut() };
	let handle = stream.runtime.spawn(
		FastStream_routine(FastStreamPtr(stream_ptr)));
	stream.stream_task = Some(handle);

	// Start consuming audio by unpausing
	return FastStream_play(stream_ptr, true);
}

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_play(stream_ptr: *mut FastStream, play: bool) -> c_int {
	let stream = unsafe { (*stream_ptr).borrow_mut() };
	let paused = stream.paused.get();

	// Debounce play signals
	if (play && !paused) || (!play && paused) {
		return 0;
	}

	stream.runtime.block_on(stream.paused.set(!play));
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
				eprintln!("{} ticks elapsed ({} bytes read)\n", n_ticks, n_ticks * read_size);
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

	// Clunkiest shit ever
	if let Some(_guard) = stream.callback_lock.try_lock() {

		// move guard into the spawned callback
		mem::forget(_guard);
		stream.callback_task = Some(spawn(async move {
			let stream = unsafe { &mut *stream_ptr.0 };
			let _guard = unsafe { stream.callback_lock.make_guard_unchecked() };

			if let Some(write_cb) = stream.write_cb {
				unsafe { write_cb(stream_ptr.into(), n_bytes, stream.write_cb_userdata.0) };
			}
		}));
	}
}
