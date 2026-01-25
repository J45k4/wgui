use std::sync::{Arc, Mutex};
use crate::context::SharedContext;

pub struct TodoController {
	next_id: u32,
	shared: Arc<Mutex<SharedContext>>,
}

impl TodoController {
	pub fn new(shared: Arc<Mutex<SharedContext>>) -> Self {
		Self {
			next_id: 1,
			shared,
		}
	}

	pub fn state(&self) -> crate::TodoState {
		let shared = self.shared.lock().unwrap();
		shared.state.clone()
	}

	// <wui:handlers>
	pub(crate) fn edit_new_todo(&mut self, value: String) {
		let mut shared = self.shared.lock().unwrap();
		shared.state.new_todo_name = value;
	}

	pub(crate) fn add_todo(&mut self) {
		let mut shared = self.shared.lock().unwrap();
		let name = shared.state.new_todo_name.trim().to_string();
		if !name.is_empty() {
			shared.state.items.push(crate::TodoItem {
				id: self.next_id,
				name,
				completed: false,
			});
			self.next_id += 1;
		}
		shared.state.new_todo_name.clear();
	}

	pub(crate) fn toggle_todo(&mut self, arg: u32) {
		let mut shared = self.shared.lock().unwrap();
		if let Some(item) = shared.state.items.iter_mut().find(|item| item.id == arg) {
			item.completed = !item.completed;
		}
	}

	// </wui:handlers>
}
