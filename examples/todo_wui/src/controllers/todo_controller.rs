use crate::context::SharedContext;
use std::sync::{Arc, Mutex};

pub struct TodoController {
	shared: Arc<Mutex<SharedContext>>,
}

impl TodoController {
	pub fn new(shared: Arc<Mutex<SharedContext>>) -> Self {
		Self { shared }
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
		if shared.next_id == 0 {
			shared.next_id = 1;
		}
		let name = shared.state.new_todo_name.trim().to_string();
		if !name.is_empty() {
			let id = shared.next_id;
			shared.state.items.push(crate::TodoItem {
				id,
				name,
				completed: false,
			});
			shared.next_id += 1;
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
