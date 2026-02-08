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
		self.ctx
			.session_id()
			.unwrap_or_else(|| format!("client-{}", self.ctx.client_id().unwrap_or(0)))
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

	fn dm_thread_key(left: &str, right: &str) -> String {
		if left <= right {
			format!("{}|{}", left, right)
		} else {
			format!("{}|{}", right, left)
		}
	}

	fn push_message_to_active(
		shared: &mut crate::ChatState,
		session: &crate::SessionState,
		message: crate::Message,
	) {
		if session.active_kind == "channel" {
			if let Some(channel) = shared
				.channels
				.iter_mut()
				.find(|c| c.id == session.active_id)
			{
				channel.messages.push(message);
			}
		} else if session.active_kind == "dm" {
			let other_name = shared
				.directs
				.iter()
				.find(|dm| dm.id == session.active_id)
				.map(|dm| dm.name.clone());
			if let Some(other_name) = other_name {
				let key = Self::dm_thread_key(&session.user_name, &other_name);
				shared.dm_threads.entry(key).or_default().push(message);
			}
		}
	}

	fn active_image_by_id(
		shared: &crate::ChatState,
		session: &crate::SessionState,
		message_id: u32,
	) -> Option<String> {
		if session.active_kind == "channel" {
			return shared
				.channels
				.iter()
				.find(|c| c.id == session.active_id)
				.and_then(|channel| {
					channel
						.messages
						.iter()
						.find(|msg| msg.id == message_id)
						.map(|msg| msg.image_url.clone())
				})
				.filter(|url| !url.is_empty());
		}
		if session.active_kind == "dm" {
			let other_name = shared
				.directs
				.iter()
				.find(|dm| dm.id == session.active_id)
				.map(|dm| dm.name.clone())?;
			let key = Self::dm_thread_key(&session.user_name, &other_name);
			return shared
				.dm_threads
				.get(&key)
				.and_then(|messages| {
					messages
						.iter()
						.find(|msg| msg.id == message_id)
						.map(|msg| msg.image_url.clone())
				})
				.filter(|url| !url.is_empty());
		}
		None
	}
}

#[wgui_controller]
impl Puppychat {
	pub fn state(&self) -> crate::ChatViewState {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		let user_name = session.user_name.clone();
		crate::ChatViewState {
			user_name: user_name.clone(),
			login_name: session.login_name.clone(),
			new_message: session.new_message.clone(),
			new_picture_url: session.new_picture_url.clone(),
			new_channel_name: session.new_channel_name.clone(),
			show_create_channel: session.show_create_channel,
			show_attach_menu: session.show_attach_menu,
			show_image_modal: session.show_image_modal,
			selected_image_url: session.selected_image_url.clone(),
			active_kind: session.active_kind.clone(),
			active_id: session.active_id,
			active_name: session.active_name.clone(),
			channels: shared.channels.clone(),
			directs: {
				let mut directs = shared
					.directs
					.iter()
					.filter(|dm| dm.name != user_name)
					.cloned()
					.collect::<Vec<_>>();
				for dm in &mut directs {
					dm.messages = if user_name.is_empty() {
						Vec::new()
					} else {
						let key = Self::dm_thread_key(&user_name, &dm.name);
						shared.dm_threads.get(&key).cloned().unwrap_or_default()
					};
				}
				directs.sort_by(|left, right| {
					let left_last = left.messages.last().map(|msg| msg.id).unwrap_or(0);
					let right_last = right.messages.last().map(|msg| msg.id).unwrap_or(0);
					right_last.cmp(&left_last)
				});
				directs
			},
		}
	}

	pub(crate) fn edit_login_name(&mut self, value: String) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		session.login_name = value;
	}

	pub(crate) fn login(&mut self) {
		let mut shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		let name = session.login_name.trim().to_string();
		if name.is_empty() {
			return;
		}
		session.user_name = name;
		let user_name = session.user_name.clone();
		session.login_name.clear();
		if !shared.directs.iter().any(|dm| dm.name == user_name) {
			let next_id = shared
				.directs
				.iter()
				.map(|dm| dm.id)
				.max()
				.unwrap_or(0)
				.saturating_add(1);
			shared.directs.push(crate::DirectMessage {
				id: next_id,
				name: user_name.clone(),
				display_name: format!("@ {}", user_name),
				online: true,
				messages: Vec::new(),
			});
		}
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) fn edit_new_message(&mut self, value: String) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		session.new_message = value;
	}

	pub(crate) fn edit_new_picture_url(&mut self, value: String) {
		let mut next_id = self.ctx.state.next_message_id.lock().unwrap();
		let mut shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		let image_url = value.trim().to_string();
		if image_url.is_empty() || session.user_name.is_empty() {
			return;
		}
		let message = crate::Message {
			id: *next_id,
			author: session.user_name.clone(),
			body: String::new(),
			image_url,
			time: "now".to_string(),
		};
		*next_id += 1;
		Self::push_message_to_active(&mut shared, session, message);
		session.new_picture_url.clear();
		session.show_attach_menu = false;
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) fn open_attach_menu(&mut self) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		session.show_attach_menu = true;
	}

	pub(crate) fn close_attach_menu(&mut self) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		session.show_attach_menu = false;
	}

	pub(crate) fn open_message_image(&mut self, arg: u32) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		if let Some(url) = Self::active_image_by_id(&shared, session, arg) {
			session.selected_image_url = url;
			session.show_image_modal = true;
			self.ctx.pubsub().publish("rerender", ());
		}
	}

	pub(crate) fn close_image_modal(&mut self) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		session.show_image_modal = false;
		session.selected_image_url.clear();
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) fn open_create_channel(&mut self) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		session.show_create_channel = true;
	}

	pub(crate) fn close_create_channel(&mut self) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		session.show_create_channel = false;
	}

	pub(crate) fn edit_new_channel_name(&mut self, value: String) {
		let shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		session.new_channel_name = value;
	}

	pub(crate) fn create_channel(&mut self) {
		let mut shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		let name = session.new_channel_name.trim().to_string();
		if name.is_empty() {
			return;
		}
		let mut next_id = self.ctx.state.next_channel_id.lock().unwrap();
		let id = *next_id;
		*next_id = next_id.saturating_add(1);
		let display = if name.starts_with('#') {
			name.clone()
		} else {
			format!("# {}", name)
		};
		shared.channels.push(crate::Channel {
			id,
			name: name.clone(),
			display_name: display.clone(),
			messages: Vec::new(),
		});
		session.active_kind = "channel".to_string();
		session.active_id = id;
		session.active_name = display;
		session.new_channel_name.clear();
		session.show_create_channel = false;
		self.ctx.pubsub().publish("rerender", ());
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
		if body.is_empty() || session.user_name.is_empty() {
			return;
		}
		let author = session.user_name.clone();
		let message = crate::Message {
			id: *next_id,
			author,
			body,
			image_url: String::new(),
			time: "now".to_string(),
		};
		*next_id += 1;
		Self::push_message_to_active(&mut shared, session, message);
		session.new_message.clear();
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) fn send_picture(&mut self) {
		let mut next_id = self.ctx.state.next_message_id.lock().unwrap();
		let mut shared = self.ctx.state.state.lock().unwrap();
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&shared, &mut sessions);
		let image_url = session.new_picture_url.trim().to_string();
		if image_url.is_empty() || session.user_name.is_empty() {
			return;
		}
		let author = session.user_name.clone();
		let message = crate::Message {
			id: *next_id,
			author,
			body: String::new(),
			image_url,
			time: "now".to_string(),
		};
		*next_id += 1;
		Self::push_message_to_active(&mut shared, session, message);
		session.new_picture_url.clear();
		self.ctx.pubsub().publish("rerender", ());
	}
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
