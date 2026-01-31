use crate::context::SharedContext;
use async_trait::async_trait;
use std::sync::Arc;
use wgui::wgui_controller;
use wgui::wui::runtime::{Component, Ctx};

pub struct Puppychat {
	ctx: Arc<Ctx<SharedContext>>,
}

#[wgui_controller]
impl Puppychat {
	pub fn new(ctx: Arc<Ctx<SharedContext>>) -> Self {
		Self { ctx }
	}

	pub fn state(&self) -> crate::ChatState {
		self.ctx.state.state.lock().unwrap().clone()
	}

	// <wui:handlers>
	pub(crate) fn edit_new_message(&mut self, value: String) {
		self.ctx.state.state.lock().unwrap().new_message = value;
	}

	pub(crate) fn select_channel(&mut self, arg: u32) {
		let mut state = self.ctx.state.state.lock().unwrap();
		let selected = state
			.channels
			.iter()
			.find(|channel| channel.id == arg)
			.map(|channel| (channel.id, channel.display_name.clone()));
		if let Some((id, name)) = selected {
			state.active_kind = "channel".to_string();
			state.active_id = id;
			state.active_name = name;
		}
	}

	pub(crate) fn select_direct(&mut self, arg: u32) {
		let mut state = self.ctx.state.state.lock().unwrap();
		let selected = state
			.directs
			.iter()
			.find(|dm| dm.id == arg)
			.map(|dm| (dm.id, dm.display_name.clone()));
		if let Some((id, name)) = selected {
			state.active_kind = "dm".to_string();
			state.active_id = id;
			state.active_name = name;
		}
	}

	pub(crate) fn send_message(&mut self) {
		let mut next_id = self.ctx.state.next_message_id.lock().unwrap();
		let mut state = self.ctx.state.state.lock().unwrap();
		let body = state.new_message.trim().to_string();
		if body.is_empty() {
			return;
		}
		let message = crate::Message {
			id: *next_id,
			author: "You".to_string(),
			body,
			time: "now".to_string(),
		};
		*next_id += 1;
		let active_kind = state.active_kind.clone();
		let active_id = state.active_id;
		if active_kind == "channel" {
			if let Some(channel) = state.channels.iter_mut().find(|c| c.id == active_id) {
				channel.messages.push(message);
			}
		} else if active_kind == "dm" {
			if let Some(dm) = state.directs.iter_mut().find(|d| d.id == active_id) {
				dm.messages.push(message);
			}
		}
		state.new_message.clear();
	}
	// </wui:handlers>
}

#[async_trait]
impl Component for Puppychat {
	type Context = SharedContext;
	type Model = crate::ChatState;

	async fn mount(ctx: Arc<Ctx<SharedContext>>) -> Self {
		Self::new(ctx)
	}

	fn render(&self, ctx: &Ctx<SharedContext>) -> Self::Model {
		ctx.state.state.lock().unwrap().clone()
	}

	fn unmount(self, _ctx: Arc<Ctx<SharedContext>>) {}
}
