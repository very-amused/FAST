use std::{borrow::BorrowMut, ffi::c_void};

use crate::sys::FastStream_write_callback;
use super::FastStream;

#[unsafe(no_mangle)]
pub extern "C" fn FastStream_set_write_cb(stream_ptr: *mut FastStream, cb: FastStream_write_callback, userdata: *mut c_void) {
	let stream = unsafe { (*stream_ptr).borrow_mut() };

	stream.write_cb_userdata.0 = userdata;
	stream.write_cb = cb;
}
