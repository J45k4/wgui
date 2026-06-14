use log::Level;
use std::sync::Arc;
use wgui::wui::runtime::Ctx;
use wgui::Wgui;
use wgui::WguiModel;

mod context;
mod pages;

#[derive(Debug, Clone, WguiModel)]
pub struct TodoItem {
	id: u32,
	name: String,
	completed: bool,
}

#[derive(Debug, Default, Clone, WguiModel)]
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
	wgui.add_page::<pages::todo::Index>("/");
	wgui.add_page::<pages::todo::Show>("/todo/:todo_id");
	wgui.add_page::<pages::not_found::NotFound>("/*");
	wgui.run().await;
}
