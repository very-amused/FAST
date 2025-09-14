use core::ffi::c_void;

pub struct Userdata(pub *mut c_void);
unsafe impl Send for Userdata {}
