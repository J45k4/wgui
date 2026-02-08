use log::Level;
use std::sync::Arc;
use wgui::wui::runtime::Ctx;
use wgui::Wgui;
use wgui::WuiModel;

mod components;
mod context;

#[derive(Debug, Clone, WuiModel)]
pub struct TodoItem {
	id: u32,
	name: String,
	completed: bool,
}

#[derive(Debug, Default, Clone, WuiModel)]
pub struct TodoState {
	new_todo_name: String,
	items: Vec<TodoItem>,
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let ctx = Arc::new(Ctx::new(context::SharedContext::default()));
	let mut wgui = Wgui::new("0.0.0.0:12345".parse().unwrap());
	wgui.set_ctx(ctx.clone());
	wgui.add_component::<components::todo::Todo>("/");
	wgui.run().await;
}
