#[cfg(feature = "axum")]
use std::sync::Arc;
#[cfg(feature = "axum")]
use axum::Router;
#[cfg(feature = "axum")]
use crate::controllers::todo::TodoController;
use wgui::wui::runtime::Ctx;
use crate::context::SharedContext;

#[cfg(feature = "axum")]
pub fn router(ctx: Arc<Ctx<SharedContext>>) -> Router {
	let routes: Vec<&'static str> = ROUTES.iter().map(|r| r.route).collect();
	wgui::wui::runtime::router_with_component::<TodoController>(ctx, &routes)
}

pub struct RouteDef {
	pub module: &'static str,
	pub route: &'static str,
}

pub const ROUTES: &[RouteDef] = &[
	RouteDef { module: "todo", route: "/todo" },
];
