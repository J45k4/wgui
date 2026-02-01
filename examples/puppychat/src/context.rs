use crate::ChatState;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug)]
pub struct SharedContext {
	pub state: Mutex<ChatState>,
	pub next_message_id: Mutex<u32>,
	pub sessions: Mutex<HashMap<String, crate::SessionState>>,
	pub next_channel_id: Mutex<u32>,
}

impl Default for SharedContext {
	fn default() -> Self {
		let state = ChatState::default();
		let next_channel_id = state
			.channels
			.iter()
			.map(|c| c.id)
			.max()
			.unwrap_or(0)
			.saturating_add(1);
		Self {
			state: Mutex::new(state),
			next_message_id: Mutex::new(1),
			sessions: Mutex::new(HashMap::new()),
			next_channel_id: Mutex::new(next_channel_id),
		}
	}
}
