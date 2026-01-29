#[cfg(feature = "axum")]
use std::sync::{Arc, Mutex};
#[cfg(feature = "axum")]
use axum::Router;
#[cfg(feature = "axum")]
use crate::controllers::todo_controller::TodoController;
use crate::context::SharedContext;

#[cfg(feature = "axum")]
pub fn router(shared: Arc<Mutex<SharedContext>>) -> Router {
	let routes: Vec<&'static str> = ROUTES.iter().map(|r| r.route).collect();
	wgui::wui::runtime::router_with_component::<TodoController>(shared, &routes)
}

pub struct RouteDef {
	pub module: &'static str,
	pub route: &'static str,
}

pub const ROUTES: &[RouteDef] = &[
	RouteDef { module: "todo", route: "/todo" },
];
