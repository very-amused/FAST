use tokio::sync::Notify;

// A basic flag value for one way parent -> child thread state communication
pub struct ThreadFlag<T: PartialEq + Copy> {
	desired: Option<T>, // desired value, modified by the value setter thread
	actual: T,

	setter_wake: Notify, // wake up a setter thread b/c actual has been set from desired
	getter_wake: Notify // wake up get_new b/c a new desired value has been provided
}

impl<T: PartialEq + Copy> ThreadFlag<T> {
	pub fn new(initial_val: T) -> Self {
		Self {
			desired: None,
			actual: initial_val,

			setter_wake: Notify::new(),
			getter_wake: Notify::new()
		}
	}

	/// Set a ThreadFlag's value
	/// NOTE: should be called from the parent thread
	pub async fn set(&mut self, val: T) {
		self.desired = Some(val);

		// Notify getters that [desired] has changed
		self.getter_wake.notify_one();
		// Wait for child thread to set actual value
		self.setter_wake.notified().await;
	}

	// Get a ThreadFlag's actual value without triggering any side effects.
	//
	// NOTE: can be called on either the parent or the child thread
	pub fn get(&self) -> T {
		self.actual
	}

	// Get the actual val when it has changed, notifying the setter of state change
	//
	// NOTE: should only be called on the child thread, ideally as part of a tokio::select block
	pub async fn get_new(&mut self) -> T {
		// Wait for [desired] to change
		loop {
			self.getter_wake.notified().await;
			match self.desired {
				Some(val) => {
					self.desired = None;
					self.actual = val;
					self.setter_wake.notify_one();
					return val;
				},
				None => {}
			}
		}
	}
}
