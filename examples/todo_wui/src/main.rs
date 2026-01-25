use log::Level;
use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use crate::controllers::todo_controller::TodoController;
use wgui::*;

mod controllers;
mod generated;

#[derive(Debug, Clone, WuiValue)]
struct TodoItem {
	id: u32,
	name: String,
	completed: bool,
}

#[derive(Debug, Default, Clone, WuiValue)]
struct TodoState {
	new_todo_name: String,
	items: Vec<TodoItem>,
}

fn build_title(state: &TodoState) -> String {
	let done = state.items.iter().filter(|item| item.completed).count();
	let undone = state.items.len() - done;
	format!("Todo {} done / {} undone", done, undone)
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let controller = Arc::new(RwLock::new(TodoController::new(TodoState::default())));
	let mut client_ids = HashSet::new();

	let ssr_controller = controller.clone();
	let mut wgui = Wgui::new_with_ssr(
		"0.0.0.0:12345".parse().unwrap(),
		Arc::new(move || {
			let controller = ssr_controller.read().unwrap();
			controller.render()
		}),
	);

	loop {
		let mut dirty = false;
		tokio::select! {
			event = wgui.next() => {
				let Some(event) = event else { break; };
				match event {
					ClientEvent::Disconnected { id } => {
						client_ids.remove(&id);
					}
					ClientEvent::Connected { id } => {
						let controller = controller.read().unwrap();
						let title = build_title(&controller.state);
						wgui.set_title(id, &title).await;
						wgui.render(id, controller.render()).await;
						client_ids.insert(id);
					}
					ClientEvent::PathChanged(_) => {}
					ClientEvent::Input(_) => {}
					_ => {
						let mut controller = controller.write().unwrap();
						if controller.handle(&event) {
							dirty = true;
						}
					}
				}
			}
		}

		if dirty {
			let (title, item) = {
				let controller = controller.read().unwrap();
				(build_title(&controller.state), controller.render())
			};
			for id in &client_ids {
				wgui.set_title(*id, &title).await;
				wgui.render(*id, item.clone()).await;
			}
		}
	}
}
