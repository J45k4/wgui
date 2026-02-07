use log::Level;
use std::sync::Arc;
use wgui::wui::runtime::{Component, Ctx};
use wgui::Wgui;
use wgui::WuiModel;

mod components;
mod context;
mod generated;

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
	wgui.add_component("/", move || {
		let ctx = ctx.clone();
		async move { components::todo::Todo::mount(ctx).await }
	});
	wgui.run().await;
}
