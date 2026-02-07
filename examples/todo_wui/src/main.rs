use log::Level;
use std::collections::HashMap;
use std::sync::Arc;
use wgui::wui::runtime::{Component, Ctx, WuiController};
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
	let ssr_ctx = ctx.clone();
	let mut wgui = Wgui::new_with_ssr(
		"0.0.0.0:12345".parse().unwrap(),
		Arc::new(move || {
			let controller = tokio::task::block_in_place(|| {
				tokio::runtime::Handle::current()
					.block_on(components::todo::Todo::mount(ssr_ctx.clone()))
			});
			WuiController::render(&controller)
		}),
	);
	let mut controllers: HashMap<usize, components::todo::Todo> = HashMap::new();
	let mut paths: HashMap<usize, String> = HashMap::new();
	let mut pubsub_rx = ctx.pubsub().subscribe("rerender");

	while let Some(message) = wgui.next().await {
		let client_id = message.client_id;
		match message.event {
			wgui::ClientEvent::Connected { id: _ } => {
				let path = paths
					.get(&client_id)
					.cloned()
					.unwrap_or_else(|| "/".to_string());
				let controller = components::todo::Todo::mount(ctx.clone()).await;
				let item = WuiController::render_with_path(&controller, &path);
				if let Some(title) = WuiController::route_title(&controller, &path) {
					wgui.set_title(client_id, &title).await;
				}
				wgui.render(client_id, item).await;
				controllers.insert(client_id, controller);
			}
			wgui::ClientEvent::Disconnected { id: _ } => {
				if let Some(controller) = controllers.remove(&client_id) {
					controller.unmount(ctx.clone());
				}
				paths.remove(&client_id);
				wgui.clear_session(client_id).await;
			}
			wgui::ClientEvent::PathChanged(change) => {
				paths.insert(client_id, change.path.clone());
				if let Some(controller) = controllers.get_mut(&client_id) {
					let item = WuiController::render_with_path(controller, &change.path);
					if let Some(title) = WuiController::route_title(controller, &change.path) {
						wgui.set_title(client_id, &title).await;
					}
					wgui.render(client_id, item).await;
				}
			}
			wgui::ClientEvent::Input(_) => {}
			_ => {
				let path = paths
					.get(&client_id)
					.cloned()
					.unwrap_or_else(|| "/".to_string());
				if let Some(controller) = controllers.get_mut(&client_id) {
					if WuiController::handle(controller, &message.event) {
						let item = WuiController::render_with_path(controller, &path);
						if let Some(title) = WuiController::route_title(controller, &path) {
							wgui.set_title(client_id, &title).await;
						}
						wgui.render(client_id, item).await;
					}
				}
			}
		}

		while pubsub_rx.try_recv().is_ok() {
			for (client_id, controller) in controllers.iter_mut() {
				let path = paths
					.get(client_id)
					.cloned()
					.unwrap_or_else(|| "/".to_string());
				let item = WuiController::render_with_path(controller, &path);
				if let Some(title) = WuiController::route_title(controller, &path) {
					wgui.set_title(*client_id, &title).await;
				}
				wgui.render(*client_id, item).await;
			}
		}
	}
}
