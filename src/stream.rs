#![allow(dead_code)]
use tokio::sync::oneshot;
use tokio::{task, time};
use tokio::runtime::{Builder as RuntimeBuilder, Runtime};
use std::borrow::BorrowMut;
use std::io;
use std::os::raw::c_int;
use std::sync::{Condvar, Mutex};
use crossbeam_queue::ArrayQueue;

use crate::sys::FastStreamSettings;
use crate::thread_flag::ThreadFlag;

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

	/// Thread that runs callback routines
	callback_task: Option<task::JoinHandle<()>>,


	// Mutices
	callback_lock: Mutex<()> // Lock to ensure we run one callback at a time. Must be acquired when
													 // spawning a callback task
}

struct FastStreamBuffer {
	data: ArrayQueue<u8>, // Ring buffer of raw PCM data to consume
	frame_size: usize, // sample_size * n_channels, our minimum read unit
	sample_rate: usize// Numer of audio frames to read per second
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
		paused: Mutex::new(true),

		callback_task: None,
		callback_lock: Mutex::new(())
	});
	Box::leak(stream)
}

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
	let buffer = &mut stream.buffer;

	// Set up interval to read frames on
	const READ_INTERVAL_MS: u64 = 10;
	const READ_INTERVAL_DURATION: time::Duration = time::Duration::from_millis(READ_INTERVAL_MS); // i.e reads of 441 frames/10ms @ 44.1khz
	let mut interval = time::interval(READ_INTERVAL_DURATION);
	// CRITICAL: align ticks to play/pause event
	interval.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

	// Compute read size (bytes) for each tick
	let read_size: usize = (buffer.sample_rate / (1000 / READ_INTERVAL_MS as usize)) * buffer.frame_size;

	// Event loop
'evt_loop:
	loop {
	/* fixme
		interval.tick().await;
		// Wait for tick or pause signal
		tokio::select! {
			_ = interval.tick() => {
				// Read {read_size} bytes each tick
				for _ in 0..read_size {
					if buffer.data.pop() == None {
						eprintln!("Read error: FastStream buffer is empty");
					}
				}
			},
			Ok(play) = &mut stream.play.1 => if !play {
				// Pause signal received
				{
					let mut paused = stream.paused.lock().unwrap();
					*paused = true;
					stream.paused_cv.notify_all();
				}
				while let Ok(resume) = (&mut stream.play.1).await {
					if resume {
						let mut paused = stream.paused.lock().unwrap();
						*paused = false;
						stream.paused_cv.notify_all();
						continue 'evt_loop;
					}
				}
			}
		}
	*/
	}
}

pub extern "C" fn FastStream_play(stream_ptr: *mut FastStream, play: bool) -> c_int {
	let stream = unsafe { (*stream_ptr).borrow_mut() };
	let paused = stream.paused.lock().unwrap();

	if play {
		// Debounce play signals
		if !*paused {
			return 0;
		}

		// FIXME:
		stream.play.0.send(true);
	}

	todo!()
}
