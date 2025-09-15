use std::slice;
use std::{io::Write};
use std::ffi::{c_uchar, c_int};

use crossbeam_queue::ArrayQueue;

use crate::sys::FastStreamSettings;

use super::FastStream;

pub struct FastStreamBuffer {
	data: ArrayQueue<u8>, // Ring buffer of raw PCM data to consume
	pub frame_size: usize, // sample_size * n_channels, our minimum read unit
	pub sample_rate: usize// Numer of audio frames to read per second
}

impl FastStreamBuffer {
	pub fn new(settings: &FastStreamSettings) -> Self {
		// Compute frame size, sample rate, and total buffer size
		let frame_size: usize = (settings.sample_size as usize) * (settings.n_channels as usize);
		let sample_rate: usize = settings.sample_rate as usize;
		let buf_size: usize = (frame_size * sample_rate * (settings.buffer_ms as usize)) / 1000;

		FastStreamBuffer {
				data: ArrayQueue::new(buf_size),
				frame_size,
				sample_rate
			}
	}

	pub const fn read_interval_ms() -> u64 {
		10
	}

	// Get the ideal read size for a tick occuring every [read_interval_ms] milliseconds
	pub fn read_size(&self) -> usize {
		(self.sample_rate / (1000 / Self::read_interval_ms() as usize)) * self.frame_size
	}
	// Get the current write capacity for the buffer
	// (max # of bytes that can be written)
	pub fn write_capacity(&self) -> usize {
		self.data.capacity() - self.data.len()
	}
}

// Read/write impls

// The error we return when we exceed the buffer's fixed size by trying to read or write too many
// bytes
const ERR_OVERFLOW: std::io::ErrorKind = std::io::ErrorKind::QuotaExceeded;

impl FastStreamBuffer {
	pub fn read(&mut self, n: usize) -> std::io::Result<usize> {
		for _ in 0..n {
			if self.data.pop() == None {
				return Err(ERR_OVERFLOW.into());
			}
		}

		Ok(n)
	}
}

impl Write for FastStreamBuffer {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let mut n: usize = 0;

		for b in buf {
			self.data.push(*b).map_err(|_| ERR_OVERFLOW)?;
			n += 1;
		}

		Ok(n)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		Ok(())
	}
}

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_write(stream_ptr: *mut FastStream, src: *const c_uchar, n: usize) -> c_int {
	let stream = unsafe { &mut *stream_ptr };
	let buffer = &mut stream.buffer;

	// Construct slice and write to our stream's buffer
	let data = unsafe { slice::from_raw_parts(src, n) };
	if let Err(e) = buffer.write_all(data) {
		if cfg!(debug_assertions) {
			eprintln!("FastStream write error: {}", e);
		}

		return 1;
	}

	return 0;
}
