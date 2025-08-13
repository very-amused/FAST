use tokio::task;
use std::collections::VecDeque;

use crate::sys::FastStreamSettings;

pub struct FastStream {
	join_handle: Option<task::JoinHandle<()>>,
	buffer: FastStreamBuffer
}

struct FastStreamBuffer {
	data: VecDeque<u8>, // buffer of raw PCM data to consume
	frame_size: u32 // sample_size * n_channels, our minimum read unit
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
	
	// Create stream
	let stream = Box::new(FastStream {
		join_handle: None,
		buffer
	});
	Box::leak(stream)
}

pub extern "C" fn FastStream_free(stream_ptr: *mut FastStream) {
	let stream = unsafe { Box::from_raw(stream_ptr) };
	if let Some(join_handle) = stream.join_handle {
		join_handle.abort();
	}

	// stream gets dropped
}
