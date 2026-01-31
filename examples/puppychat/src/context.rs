use crate::ChatState;
use std::sync::Mutex;

#[derive(Debug)]
pub struct SharedContext {
	pub state: Mutex<ChatState>,
	pub next_message_id: Mutex<u32>,
}

impl Default for SharedContext {
	fn default() -> Self {
		Self {
			state: Mutex::new(ChatState::default()),
			next_message_id: Mutex::new(1),
		}
	}
}
