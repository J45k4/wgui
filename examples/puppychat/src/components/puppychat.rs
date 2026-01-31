use crate::context::SharedContext;
use async_trait::async_trait;
use std::sync::Arc;
use wgui::wgui_controller;
use wgui::wui::runtime::{Component, Ctx};

pub struct Puppychat {
	ctx: Arc<Ctx<SharedContext>>,
}

impl Puppychat {
	pub fn new(ctx: Arc<Ctx<SharedContext>>) -> Self {
		Self { ctx }
	}

	fn session_key(&self) -> String {
		if let Some(session_id) = self.ctx.session_id() {
			return session_id;
		}
		if let Some(client_id) = self.ctx.client_id() {
			return format!("client-{}", client_id);
		}
		"client-local".to_string()
	}

	fn ensure_session_state<'a>(
		&self,
		shared: &crate::ChatState,
		sessions: &'a mut std::collections::HashMap<String, crate::SessionState>,
	) -> &'a mut crate::SessionState {
		let key = self.session_key();
		sessions
			.entry(key)
			.or_insert_with(|| crate::SessionState::new(shared))
	}
}

#[wgui_controller]
impl Puppychat {
	pub fn state(&self) -> crate::ChatViewState {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		crate::ChatViewState {
			user_name: session.user_name.clone(),
			login_name: session.login_name.clone(),
			new_message: session.new_message.clone(),
			active_kind: session.active_kind.clone(),
			active_id: session.active_id,
			active_name: session.active_name.clone(),
			channels: shared.channels.clone(),
			directs: shared.directs.clone(),
		}
	}

	// <wui:handlers>
	pub(crate) fn edit_login_name(&mut self, value: String) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		session.login_name = value;
	}

	pub(crate) fn login(&mut self) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		let name = session.login_name.trim().to_string();
		if name.is_empty() {
			return;
		}
		session.user_name = name;
		session.login_name.clear();
	}

	pub(crate) fn edit_new_message(&mut self, value: String) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		session.new_message = value;
	}

	pub(crate) fn select_channel(&mut self, arg: u32) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		let selected = shared
			.channels
			.iter()
			.find(|channel| channel.id == arg)
			.map(|channel| (channel.id, channel.display_name.clone()));
		if let Some((id, name)) = selected {
			session.active_kind = "channel".to_string();
			session.active_id = id;
			session.active_name = name;
		}
	}

	pub(crate) fn select_direct(&mut self, arg: u32) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		let selected = shared
			.directs
			.iter()
			.find(|dm| dm.id == arg)
			.map(|dm| (dm.id, dm.display_name.clone()));
		if let Some((id, name)) = selected {
			session.active_kind = "dm".to_string();
			session.active_id = id;
			session.active_name = name;
		}
	}

	pub(crate) fn send_message(&mut self) {
		let mut next_id = self.ctx.state.next_message_id.lock().unwrap();
		let mut shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		let body = session.new_message.trim().to_string();
		if body.is_empty() {
			return;
		}
		let author = if session.user_name.is_empty() {
			"You".to_string()
		} else {
			session.user_name.clone()
		};
		let message = crate::Message {
			id: *next_id,
			author,
			body,
			time: "now".to_string(),
		};
		*next_id += 1;
		let active_kind = session.active_kind.clone();
		let active_id = session.active_id;
		if active_kind == "channel" {
			if let Some(channel) = shared.channels.iter_mut().find(|c| c.id == active_id) {
				channel.messages.push(message);
			}
		} else if active_kind == "dm" {
			if let Some(dm) = shared.directs.iter_mut().find(|d| d.id == active_id) {
				dm.messages.push(message);
			}
		}
		session.new_message.clear();
	}
	// </wui:handlers>
}

#[async_trait]
impl Component for Puppychat {
	type Context = SharedContext;
	type Model = crate::ChatViewState;

	async fn mount(ctx: Arc<Ctx<SharedContext>>) -> Self {
		Self::new(ctx)
	}

	fn render(&self, ctx: &Ctx<SharedContext>) -> Self::Model {
		self.state()
	}

	fn unmount(self, _ctx: Arc<Ctx<SharedContext>>) {}
}
