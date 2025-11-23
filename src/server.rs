use tokio::runtime::{Runtime, Builder as RuntimeBuilder};
use std::io;
use std::sync::Arc;


/// A FAST instance which embeds a runtime needed to schedule things like audio sink streams (see FastStream) and callbacks (see FastLoop).
pub struct FastServer {
	pub runtime: Arc<Runtime>
}

/// Allocate and initialize a new FastServer by embedding a Runtime
#[unsafe(no_mangle)]
pub extern "C" fn FastServer_new() -> *mut FastServer {
	// Initialize Tokio async runtime
	let runtime = match new_runtime() {
		Ok(rt) => rt,
		Err(err) => {
			eprintln!("Failed to initialize Tokio runtime: {}", err);
			return std::ptr::null_mut();
		}
	};

	let srv = Box::new(FastServer {
		runtime: Arc::new(runtime)
	});

	Box::leak(srv)
}

/// Deinitialize and free a FastServer
#[unsafe(no_mangle)]
pub extern "C" fn FastServer_free(srv_ptr: *mut FastServer) {
	let srv = unsafe { Box::from_raw(srv_ptr) };
	if Arc::strong_count(&srv.runtime) > 1 {
		eprintln!("ERROR: FastServer_free called with actrive references to FastServer. Deadlock is possible!");
	}

	drop(srv);
}

/// Initialize a Tokio runtime for powering a FastServer
fn new_runtime() -> io::Result<Runtime>{
	RuntimeBuilder::new_multi_thread()
		.worker_threads(4) // stream_task + callback_task + growing room
		.enable_all()
		.build()
}
