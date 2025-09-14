#![allow(dead_code)]
use tokio::{spawn, task, time};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};
use std::borrow::BorrowMut;
use std::ffi::c_void;
use std::io;
use std::os::raw::c_int;
use std::ptr::null_mut;
use std::sync::{Arc, Mutex};
use crossbeam_queue::ArrayQueue;

use crate::sys::{self, FastStreamSettings, FastStream_write_callback};
use crate::thread_flag::ThreadFlag;
use crate::userdata::Userdata;

pub mod callback;

/// An audio sink for FAST
pub struct FastStream {
	runtime: tokio::runtime::Runtime,
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

struct FastStreamBuffer {
	data: ArrayQueue<u8>, // Ring buffer of raw PCM data to consume
	frame_size: usize, // sample_size * n_channels, our minimum read unit
	sample_rate: usize// Numer of audio frames to read per second
}

/// Initialize a Tokio runtime capable of powering a FastStream
fn new_runtime() -> io::Result<Runtime>{
	RuntimeBuilder::new_multi_thread()
		.worker_threads(2) // stream_task + callback_task
		.enable_all()
		.build()
}

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_new(settings: *const FastStreamSettings) -> *mut FastStream {
	// Create buffer
	let frame_size: usize = unsafe {
		((*settings).sample_size as usize) * (*settings).n_channels as usize
	};
	let sample_rate: usize = unsafe { (*settings).sample_rate as usize };
	let buf_size: usize = unsafe {
		(frame_size * sample_rate * (*settings).buffer_ms as usize) / 1000
	};
	let buffer = FastStreamBuffer {
		data: ArrayQueue::new(buf_size),
		frame_size,
		sample_rate
	};

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
		buffer,

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
	const READ_INTERVAL_MS: u64 = 10;
	const READ_INTERVAL_DURATION: time::Duration = time::Duration::from_millis(READ_INTERVAL_MS); // i.e reads of 441 frames/10ms @ 44.1khz
	let mut interval = time::interval(READ_INTERVAL_DURATION);
	// CRITICAL: align ticks to play/pause event
	interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

	// Compute read size (bytes) for each tick
	let read_size: usize = (buffer.sample_rate / (1000 / READ_INTERVAL_MS as usize)) * buffer.frame_size;

	let mut n_ticks: usize = 0;

	// Request write of [write_size] when we have the space in our buffer
	let write_size = read_size * 2;
	let write_cap = buffer.data.capacity() - buffer.data.len();
	if write_cap >= write_size {
		let n_bytes = write_size * (write_cap / write_size);

		// Clunkiest shit ever
		if let Ok(_guard) = stream.callback_lock.try_lock() {
			stream.callback_task = Some(spawn(async move {
				let stream = unsafe { (*stream_ptr.0).borrow_mut() };
				let _guard = stream.callback_lock.lock();

				if let Some(write_cb) = stream.write_cb {
					unsafe { write_cb(stream_ptr.into(), n_bytes, stream.write_cb_userdata.0) };
				}
			}));
		}
	}

	// Event loop
	loop {
		// Wait for tick or pause signal
		tokio::select! {
			_ = interval.tick() => if !stream.paused.get() {
				n_ticks += 1;
				handle_reads(stream, read_size).await;
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
	for _ in 0..read_size {
		if buffer.data.pop() == None  && !cfg!(debug_assertions) {
			eprintln!("Read error: FastStream buffer is empty");
		}
	}
}

// Handle writes (via callback) for each tick
async fn handle_writes(stream_ptr: FastStreamPtr, read_size: usize) {
	let stream = unsafe { (*stream_ptr.0).borrow_mut() };
	let buffer = &mut stream.buffer;

	// Request write of [write_size] when we have the space in our buffer
	let write_size = read_size * 2;
	let write_cap = buffer.data.capacity() - buffer.data.len();
	if write_cap < write_size {
		return;
	}

	// Compute size for this write
	let n_bytes = write_size * (write_cap / write_size);

	// Clunkiest shit ever
	if let Ok(_guard) = stream.callback_lock.try_lock() {
		// FIXME: find way to seamlessly move guard to callback_task
		stream.callback_task = Some(spawn(async move {
			let stream = unsafe { (*stream_ptr.0).borrow_mut() };
			let _guard = _guard;

			if let Some(write_cb) = stream.write_cb {
				unsafe { write_cb(stream_ptr.into(), n_bytes, stream.write_cb_userdata.0) };
			}
		}));
	}
}
