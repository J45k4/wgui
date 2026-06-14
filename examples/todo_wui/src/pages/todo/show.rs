use crate::context::SharedContext;
use async_trait::async_trait;
use std::sync::Arc;
use wgui::wgui_controller;
use wgui::wui::runtime::{Component, Ctx, MountResult, RouteContext};

#[derive(Debug, Default, Clone, wgui::WguiModel)]
pub struct ShowState {
	todo_id: String,
}

pub struct Show {
	ctx: Arc<Ctx<SharedContext>>,
}

#[wgui_controller]
impl Show {
	pub fn new(ctx: Arc<Ctx<SharedContext>>) -> Self {
		Self { ctx }
	}

	pub fn state(&self) -> ShowState {
		ShowState {
			todo_id: self.ctx.param("todo_id").unwrap_or_default(),
		}
	}

	pub fn title(&self) -> String {
		format!("Todo {} - Todo", self.state().todo_id)
	}
}

#[async_trait]
impl Component for Show {
	type Context = SharedContext;
	type Db = ();
	type Model = ShowState;

	async fn mount(ctx: Arc<Ctx<SharedContext>>, _route: RouteContext) -> MountResult<Self> {
		MountResult::Ready(Self::new(ctx))
	}

	fn render(&self, _ctx: &Ctx<SharedContext>) -> Self::Model {
		self.state()
	}

	fn unmount(self, _ctx: Arc<Ctx<SharedContext>>) {}
}
