use std::{error::Error, fmt::Display};

/// A FastStream operation error
#[derive(Debug)]
pub enum StreamError {
	BufferOverflow(usize, usize),
	BufferUnderrun(usize, usize),
	StdioError(std::io::Error)
}

impl Display for StreamError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::BufferUnderrun(i, n) => write!(f, "Buffer underrun on read byte {}/{}", i, n),
			Self::BufferOverflow(i, n) => write!(f, "Buffer overflow on write byte {}/{}", i, n),
			Self::StdioError(e) => {
				e.fmt(f)
			}
		}
	}
}

impl Error for StreamError {}
