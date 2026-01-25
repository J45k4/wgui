pub struct TodoController {
	pub state: crate::TodoState,
	next_id: u32,
}

impl TodoController {
	pub fn new(state: crate::TodoState) -> Self {
		Self { state, next_id: 1 }
	}

	// <wui:handlers>
	pub(crate) fn edit_new_todo(&mut self, value: String) {
		self.state.new_todo_name = value;
	}

	pub(crate) fn add_todo(&mut self) {
		let name = self.state.new_todo_name.trim().to_string();
		if !name.is_empty() {
			self.state.items.push(crate::TodoItem {
				id: self.next_id,
				name,
				completed: false,
			});
			self.next_id += 1;
		}
		self.state.new_todo_name.clear();
	}

	pub(crate) fn toggle_todo(&mut self, arg: u32) {
		if let Some(item) = self.state.items.iter_mut().find(|item| item.id == arg) {
			item.completed = !item.completed;
		}
	}

	// </wui:handlers>
}
