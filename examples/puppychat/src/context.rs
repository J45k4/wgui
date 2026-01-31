use crate::ChatState;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct SharedContext {
	pub state: Mutex<ChatState>,
	pub next_message_id: Mutex<u32>,
	pub sessions: Mutex<HashMap<String, crate::SessionState>>,
}

impl Default for SharedContext {
	fn default() -> Self {
		Self {
			state: Mutex::new(ChatState::default()),
			next_message_id: Mutex::new(1),
			sessions: Mutex::new(HashMap::new()),
		}
	}
}
