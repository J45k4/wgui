use crate::context::SharedContext;
use async_trait::async_trait;
use std::sync::Arc;
use wgui::wgui_controller;
use wgui::wui::runtime::{Component, Ctx, MountResult, RouteContext};

#[derive(Debug, Default, Clone, wgui::WguiModel)]
pub struct NotFoundState {
	path: String,
}

pub struct NotFound {
	ctx: Arc<Ctx<SharedContext>>,
}

#[wgui_controller]
impl NotFound {
	pub fn new(ctx: Arc<Ctx<SharedContext>>) -> Self {
		Self { ctx }
	}

	pub fn state(&self) -> NotFoundState {
		NotFoundState {
			path: self
				.ctx
				.route()
				.map(|route| route.path)
				.unwrap_or_else(|| "/".to_string()),
		}
	}

	pub fn title(&self) -> String {
		"Not Found".to_string()
	}
}

#[async_trait]
impl Component for NotFound {
	type Context = SharedContext;
	type Db = ();
	type Model = NotFoundState;

	async fn mount(ctx: Arc<Ctx<SharedContext>>, _route: RouteContext) -> MountResult<Self> {
		MountResult::Ready(Self::new(ctx))
	}

	fn render(&self, _ctx: &Ctx<SharedContext>) -> Self::Model {
		self.state()
	}

	fn unmount(self, _ctx: Arc<Ctx<SharedContext>>) {}
}
