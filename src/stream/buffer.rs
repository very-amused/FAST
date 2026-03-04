use crossbeam_queue::ArrayQueue;

use crate::stream::error::StreamError;
use crate::sys::FastStreamSettings;

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

	// Get the ideal read size for a tick occuring every [read_interval_ms] milliseconds
	pub const fn read_size(&self) -> usize {
		(self.sample_rate / (1000 / Self::read_interval_ms() as usize)) * self.frame_size
	}
	// Get the constant read interval
	pub const fn read_interval_ms() -> u64 {
		10
	}

	// Get the current write capacity for the buffer
	// (max # of bytes that can be written)
	pub fn write_capacity(&self) -> usize {
		self.data.capacity() - self.data.len()
	}

	pub fn read(&mut self, n: usize) -> Result<usize, StreamError> {
		for i in 0..n {
			if self.data.pop() == None {
				return Err(StreamError::BufferUnderrun(i, n));
			}
		}

		Ok(n)
	}

	pub fn write(&mut self, buf: &[u8]) -> Result<usize, StreamError> {
		let n: usize = buf.len();

		for (i, b) in buf.iter().enumerate() {
			// if push fails,
			// crossbeam_queue returns the byte itself as the err val
			if let Err(_) = self.data.push(*b) {
				return Err(StreamError::BufferOverflow(i, n));
			}
		}

		Ok(n)
	}
}
