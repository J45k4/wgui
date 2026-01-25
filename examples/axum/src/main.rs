use axum::Router;
use log::Level;
use std::{collections::HashSet, net::SocketAddr, sync::Arc};
use tokio::sync::Mutex;
use wgui::{ClientEvent, Item, Wgui};

#[derive(Default)]
struct UiState {
	client_ids: HashSet<usize>,
	count: i32,
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let mut wgui = Wgui::new_without_server();
	let handle = wgui.handle();
	let router = wgui.router();
	let ui_state = Arc::new(Mutex::new(UiState::default()));

	tokio::spawn({
		let ui_state = ui_state.clone();
		let render_handle = handle.clone();
		async move {
			while let Some(message) = wgui.next().await {
				let client_id = message.client_id;
				match message.event {
					ClientEvent::Connected { id: _ } => {
						let count = {
							let mut state = ui_state.lock().await;
							state.client_ids.insert(client_id);
							state.count
						};
						render_handle
							.render(client_id, render_counter(count))
							.await;
					}
					ClientEvent::Disconnected { id: _ } => {
						ui_state.lock().await.client_ids.remove(&client_id);
					}
					ClientEvent::OnClick(_) => {
						let (count, ids) = {
							let mut state = ui_state.lock().await;
							state.count += 1;
							(
								state.count,
								state.client_ids.iter().copied().collect::<Vec<_>>(),
							)
						};
						for id in ids {
							render_handle.render(id, render_counter(count)).await;
						}
					}
					_ => {}
				}
			}
		}
	});

	let app = Router::new().merge(router);

	let addr: SocketAddr = "0.0.0.0:4001".parse().unwrap();
	log::info!("listening on http://localhost:4001");

	let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
	axum::serve(listener, app).await.unwrap();
}

fn render_counter(count: i32) -> Item {
	let label = format!("Count: {}", count);
	wgui::vstack([
		wgui::img(
			"https://images.unsplash.com/photo-1524678606370-a47ad25cb82a?auto=format&fit=crop&w=600&q=60",
			"Sample space"
		)
		.width(360)
		.height(210)
		.object_fit("cover"),
		wgui::text(&label).margin_bottom(4),
		wgui::button("+1").id(1),
	])
}
