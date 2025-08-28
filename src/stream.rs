#![allow(dead_code)]
use tokio::task;
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};
use std::borrow::{BorrowMut};
use std::collections::VecDeque;
use std::io;
use std::os::raw::c_int;
use std::sync::Mutex;

use crate::sys::FastStreamSettings;

/// An audio sink for FAST
pub struct FastStream {
	runtime: tokio::runtime::Runtime,
	/// Thread that consumes audio frames
	stream_task: Option<task::JoinHandle<()>>,
	/// Thread that runs callback routines
	callback_task: Option<task::JoinHandle<()>>,

	/// Buffer that
	/// - [stream_task] reads from
	/// - [callback_task] writes to
	buffer: FastStreamBuffer,

	// Mutices
	callback_lock: Mutex<()> // Lock to ensure we run one callback at a time. Must be acquired when
													 // spawning a callback task
}

struct FastStreamBuffer {
	data: VecDeque<u8>, // buffer of raw PCM data to consume
	frame_size: u32 // sample_size * n_channels, our minimum read unit
}

struct FastStreamPtr(*mut FastStream);
unsafe impl Send for FastStreamPtr {}

/// Initialize a Tokio runtime capable of powering a FastStream
fn new_runtime() -> io::Result<Runtime>{
	RuntimeBuilder::new_multi_thread()
		.worker_threads(2) // stream_task + callback_task
		.enable_all()
		.build()
}

pub extern "C" fn FastStream_new(settings: *const FastStreamSettings) -> *mut FastStream {
	// Create buffer
	let frame_size: u32 = unsafe {
		((*settings).sample_size as u32) * (*settings).n_channels
	};
	let buf_size: usize = unsafe {
		((frame_size * (*settings).sample_rate * (*settings).buffer_ms) as usize) / 1000
	};
	let buffer = FastStreamBuffer {
		data: VecDeque::with_capacity(buf_size),
		frame_size
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
		stream_task: None,
		callback_task: None,
		buffer,
		callback_lock: Mutex::new(())
	});
	Box::leak(stream)
}

pub extern "C" fn FastStream_free(stream_ptr: *mut FastStream) {
	let stream = unsafe { Box::from_raw(stream_ptr) };
	if let Some(join_handle) = stream.stream_task {
		join_handle.abort();
	}
	if let Some(join_handle) = stream.callback_task {
		join_handle.abort();
	}

	// stream gets dropped
}

pub extern "C" fn FastStream_start(stream_ptr: *mut FastStream) -> c_int {
	// Spawn stream task
	let stream = unsafe { (*stream_ptr).borrow_mut() };
	let handle = stream.runtime.spawn(
		FastStream_routine(FastStreamPtr(stream_ptr)));
	stream.stream_task = Some(handle);

	return 0;
}

async fn FastStream_routine(stream_ptr: FastStreamPtr) {
	let stream = unsafe { (*stream_ptr.0).borrow_mut() };

	// TODO: set up clock and read from stream.buffer
}
