use crate::TodoState;
use std::sync::Mutex;

#[derive(Debug, Default)]
pub struct SharedContext {
	pub state: Mutex<TodoState>,
	pub next_id: Mutex<u32>,
}
