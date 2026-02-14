use crate::context::SharedContext;
use async_trait::async_trait;
use std::sync::Arc;
use wgui::wgui_controller;
use wgui::wui::runtime::{Component, Ctx};

pub struct Todo {
	ctx: Arc<Ctx<SharedContext>>,
}

#[wgui_controller]
impl Todo {
	pub fn new(ctx: Arc<Ctx<SharedContext>>) -> Self {
		let component = Self { ctx };
		component.update_title();
		component
	}

	pub fn state(&self) -> crate::TodoState {
		self.ctx.state.state.lock().unwrap().clone()
	}

	fn update_title(&self) {
		let title = {
			let state = self.ctx.state.state.lock().unwrap();
			let done = state.items.iter().filter(|item| item.completed).count();
			let undone = state.items.len() - done;
			format!("Todo {} done / {} undone", done, undone)
		};
		self.ctx.set_title(title);
	}

	pub(crate) fn edit_new_todo(&mut self, value: String) {
		self.ctx.state.state.lock().unwrap().new_todo_name = value;
	}

	pub(crate) fn add_todo(&mut self) {
		let mut next_id = self.ctx.state.next_id.lock().unwrap();
		if *next_id == 0 {
			*next_id = 1;
		}
		let mut state = self.ctx.state.state.lock().unwrap();
		let name = state.new_todo_name.trim().to_string();
		if !name.is_empty() {
			let id = *next_id;
			state.items.push(crate::TodoItem {
				id,
				name,
				completed: false,
			});
			*next_id += 1;
		}
		state.new_todo_name.clear();
		let done = state.items.iter().filter(|item| item.completed).count();
		let undone = state.items.len() - done;
		let title = format!("Todo {} done / {} undone", done, undone);
		println!("title {}", title);
		self.ctx.set_title(title);
	}

	pub(crate) fn toggle_todo(&mut self, arg: u32) {
		let title = {
			let mut state = self.ctx.state.state.lock().unwrap();
			if let Some(item) = state.items.iter_mut().find(|item| item.id == arg) {
				item.completed = !item.completed;
			}
			let done = state.items.iter().filter(|item| item.completed).count();
			let undone = state.items.len() - done;
			format!("Todo {} done / {} undone", done, undone)
		};
		self.ctx.set_title(title);
	}
}

#[async_trait]
impl Component for Todo {
	type Context = SharedContext;
	type Db = ();
	type Model = crate::TodoState;

	async fn mount(ctx: Arc<Ctx<SharedContext>>) -> Self {
		Self::new(ctx)
	}

	fn render(&self, ctx: &Ctx<SharedContext>) -> Self::Model {
		ctx.state.state.lock().unwrap().clone()
	}

	fn unmount(self, _ctx: Arc<Ctx<SharedContext>>) {}
}
