use std::sync::{Mutex, Condvar};

struct ThreadFlagVal<T: PartialEq + Copy> {
	actual: T,
	desired: Option<T>
}

// A basic flag value for one way parent -> child thread state communication
pub struct ThreadFlag<T: PartialEq + Copy> {
	val: Mutex<ThreadFlagVal<T>>, // actual and desired values + lock
	done: Condvar // child thread signals that val.actual == val.desired
}

impl<T: PartialEq + Copy> ThreadFlag<T> {
	pub fn new(initial_val: T) -> Self {
		Self {
			val: Mutex::new(ThreadFlagVal { actual: initial_val, desired: None }),
			done: Condvar::new()
		}	
	}

	/// Set a ThreadFlag's value
	/// NOTE: should be called from the parent thread
	pub fn set(&mut self, val: T) {
		// Set desired value of flag, thread is expected to periodically check and update the actual
		// value. This can be done implicitly using [get_new].
		let mut guard = self.val.lock().unwrap();
		guard.desired = Some(val);

		// Wait for thread to set actual value
		self.done.wait_while(guard, |v| v.desired != None);
	}

	// Get a ThreadFlag's value without triggering any side effects.
	//
	// NOTE: can be called on either the parent or the child thread
	pub fn get(&self) -> T {
		let guard = self.val.lock().unwrap();
		guard.actual
	}

	/// Get a ThreadFlag's value iff it has changed, alowing
	/// the child thread to update state based on changes in value.
	/// If [Some(T)] is returned, [self.done] was signalled.
	///
	/// DEADLOCK HAZARD: if the child thread fails to periodically call [get_new] for all of its ThreadFlag's,
	/// it risks locking up its parent thread and creating a deadlock.
	///
	/// Returns [None] if the ThreadFlag's value is unchanged
	pub fn get_new(&mut self) -> Option<T> {
		let mut guard = self.val.lock().unwrap();

		if let Some(desired) = guard.desired {
			guard.actual = desired;
			guard.desired = None;
			Some(guard.actual)
		} else {
			None
		}
	}
}
