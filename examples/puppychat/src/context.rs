use std::collections::HashMap;
use std::sync::Mutex;

pub struct SharedContext {
	pub sessions: Mutex<HashMap<String, crate::SessionState>>,
}

impl Default for SharedContext {
	fn default() -> Self {
		Self {
			sessions: Mutex::new(HashMap::new()),
		}
	}
}
