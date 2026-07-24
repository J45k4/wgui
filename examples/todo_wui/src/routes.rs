use crate::context::SharedContext;
use crate::{TodoItem, TodoState};
use serde::Deserialize;
use wgui::wui::runtime::Ctx;
use wgui::{route, view, Redirect, View};

#[derive(Deserialize)]
pub struct CreateTodoForm {
	name: String,
}

fn todo_title(state: &TodoState) -> String {
	let done = state.items.iter().filter(|item| item.completed).count();
	let undone = state.items.len() - done;
	format!("Todo {done} done / {undone} undone")
}

fn todos(ctx: &Ctx<SharedContext>) -> TodoState {
	ctx.state.state.lock().unwrap().clone()
}

#[route("/todos", view)]
pub fn page_todos(ctx: &Ctx<SharedContext>) -> View {
	let state = todos(ctx);
	view!({
		title: todo_title(&state),
		items: state.items,
	})
}

#[route("/todos/:id", view)]
pub fn page_todo(_ctx: &Ctx<SharedContext>, id: u32) -> View {
	view!({ title: format!("Todo {id} - Todo") })
}

#[route("/todos/create", method = "POST")]
pub fn create_todo(ctx: &Ctx<SharedContext>, form: CreateTodoForm) -> Redirect {
	let name = form.name.trim().to_string();
	if !name.is_empty() {
		let mut next_id = ctx.state.next_id.lock().unwrap();
		if *next_id == 0 {
			*next_id = 1;
		}
		ctx.state.state.lock().unwrap().items.push(TodoItem {
			id: *next_id,
			name,
			completed: false,
		});
		*next_id += 1;
	}
	Redirect::to("/todos")
}

#[route("/todos/:id/toggle", method = "POST")]
pub fn toggle_todo(ctx: &Ctx<SharedContext>, id: u32) -> Redirect {
	if let Some(item) = ctx
		.state
		.state
		.lock()
		.unwrap()
		.items
		.iter_mut()
		.find(|item| item.id == id)
	{
		item.completed = !item.completed;
	}
	Redirect::to("/todos")
}

#[route("/*", view)]
pub fn page_not_found(_ctx: &Ctx<SharedContext>) -> View {
	view!({})
}
