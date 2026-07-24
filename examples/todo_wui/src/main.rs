use log::Level;
use std::sync::Arc;
use wgui::wui::runtime::Ctx;
use wgui::Wgui;
use wgui::WguiModel;

mod context;
mod routes;

#[derive(Debug, Clone, WguiModel)]
pub struct TodoItem {
	id: u32,
	name: String,
	completed: bool,
}

#[derive(Debug, Default, Clone, WguiModel)]
pub struct TodoState {
	items: Vec<TodoItem>,
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let ctx = Arc::new(Ctx::new(context::SharedContext::default()));
	let mut wgui = Wgui::new("0.0.0.0:12345".parse().unwrap());
	wgui.set_ctx(ctx.clone());
	wgui.add_route(routes::page_todos_route);
	wgui.add_route(routes::page_todo_route);
	wgui.add_route(routes::create_todo_route);
	wgui.add_route(routes::toggle_todo_route);
	wgui.add_route(routes::page_not_found_route);
	wgui.run().await;
}
