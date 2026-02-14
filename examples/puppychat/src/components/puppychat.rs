use crate::context::SharedContext;
use crate::{Channel, ChatViewState, Message, PuppyDb, SessionState, User};
use async_trait::async_trait;
use std::sync::Arc;
use wgui::wgui_controller;
use wgui::wui::runtime::{Component, Ctx};

pub struct Puppychat {
	ctx: Arc<Ctx<SharedContext, PuppyDb>>,
}

impl Puppychat {
	pub fn new(ctx: Arc<Ctx<SharedContext, PuppyDb>>) -> Self {
		ctx.set_title("Puppychat | chat");
		Self { ctx }
	}

	fn session_key(&self) -> String {
		self.ctx
			.session_id()
			.unwrap_or_else(|| format!("client-{}", self.ctx.client_id().unwrap_or(0)))
	}

	fn ensure_session_state<'a>(
		&self,
		sessions: &'a mut std::collections::HashMap<String, SessionState>,
	) -> &'a mut SessionState {
		let key = self.session_key();
		let default_channel = self
			.ctx
			.db()
			.channels
			.snapshot()
			.into_iter()
			.next()
			.map(|channel| (channel.id, channel.display_name));
		sessions
			.entry(key)
			.or_insert_with(|| SessionState::new(default_channel))
	}

	fn dm_thread_key(left: &str, right: &str) -> String {
		if left <= right {
			format!("{}|{}", left, right)
		} else {
			format!("{}|{}", right, left)
		}
	}

	fn message_scope(&self, session: &SessionState) -> (Option<u32>, Option<String>) {
		if session.active_kind == "channel" {
			return (Some(session.active_id), None);
		}
		if session.active_kind == "dm" {
			return (None, self.ctx.db().dm_thread_key_for_session(session));
		}
		(None, None)
	}
}

#[wgui_controller]
impl Puppychat {
	pub fn state(&self) -> ChatViewState {
		let messages = self.ctx.db().messages.snapshot();
		let mut channels = self.ctx.db().channels.snapshot();
		let directs_base = self.ctx.db().directs.snapshot();

		for channel in &mut channels {
			channel.messages = messages
				.iter()
				.filter(|msg| msg.channel_id == Some(channel.id))
				.cloned()
				.collect();
		}

		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		let user_name = session.user_name.clone();
		ChatViewState {
			user_name: user_name.clone(),
			login_name: session.login_name.clone(),
			login_password: session.login_password.clone(),
			auth_error: session.auth_error.clone(),
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
			channels,
			directs: {
				let mut directs = directs_base
					.iter()
					.filter(|dm| dm.name != user_name)
					.cloned()
					.collect::<Vec<_>>();
				for dm in &mut directs {
					dm.messages = if user_name.is_empty() {
						Vec::new()
					} else {
						let key = Self::dm_thread_key(&user_name, &dm.name);
						messages
							.iter()
							.filter(|msg| msg.dm_thread_key.as_deref() == Some(&key))
							.cloned()
							.collect()
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
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.login_name = value;
		session.auth_error.clear();
	}

	pub(crate) fn edit_login_password(&mut self, value: String) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.login_password = value;
		session.auth_error.clear();
	}

	pub(crate) fn open_register_page(&mut self) {
		self.ctx.push_state("/register");
	}

	pub(crate) fn open_login_page(&mut self) {
		self.ctx.push_state("/");
	}

	pub(crate) async fn login(&mut self) {
		let (name, password) = {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			(
				session.login_name.trim().to_string(),
				session.login_password.clone(),
			)
		};
		if name.is_empty() || password.trim().is_empty() {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.auth_error = "username and password are required".to_string();
			return;
		}

		match User::find(&name, &self.ctx.db).await {
			Some(saved) if saved.password == password => {}
			Some(_) => {
				let mut sessions = self.ctx.state.sessions.lock().unwrap();
				let session = self.ensure_session_state(&mut sessions);
				session.auth_error = "invalid username or password".to_string();
				return;
			}
			None => {
				let mut sessions = self.ctx.state.sessions.lock().unwrap();
				let session = self.ensure_session_state(&mut sessions);
				session.auth_error = "account not found, register first".to_string();
				return;
			}
		}

		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.user_name = name;
		let user_name = session.user_name.clone();
		session.login_name.clear();
		session.login_password.clear();
		session.auth_error.clear();
		self.ctx.db().ensure_direct_entry(&user_name);
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) async fn register(&mut self) {
		let (name, password) = {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			(
				session.login_name.trim().to_string(),
				session.login_password.clone(),
			)
		};

		if name.is_empty() || password.trim().is_empty() {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.auth_error = "username and password are required".to_string();
			return;
		}

		if User::find(&name, &self.ctx.db).await.is_some() {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.auth_error = "username already exists".to_string();
			return;
		}

		User {
			name: name.clone(),
			password,
		}
		.save(&self.ctx.db)
		.await;

		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.login_name = name.clone();
		session.login_password.clear();
		session.auth_error = "account created, please login".to_string();
		self.ctx.db().ensure_direct_entry(&name);
		self.ctx.push_state("/");
	}

	pub(crate) fn edit_new_message(&mut self, value: String) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.new_message = value;
	}

	pub(crate) async fn edit_new_picture_url(&mut self, value: String) {
		let message = {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			let image_url = value.trim().to_string();
			if image_url.is_empty() || session.user_name.is_empty() {
				return;
			}
			let (channel_id, dm_thread_key) = self.message_scope(session);
			Message {
				id: 0,
				author: session.user_name.clone(),
				body: String::new(),
				image_url,
				time: "now".to_string(),
				channel_id,
				dm_thread_key,
			}
		};
		message.save(&self.ctx.db).await;

		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.new_picture_url.clear();
		session.show_attach_menu = false;
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) fn open_attach_menu(&mut self) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.show_attach_menu = true;
	}

	pub(crate) fn close_attach_menu(&mut self) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.show_attach_menu = false;
	}

	pub(crate) async fn open_message_image(&mut self, arg: u32) {
		let (channel_id, dm_thread_key) = {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			self.message_scope(session)
		};
		if channel_id.is_none() && dm_thread_key.is_none() {
			return;
		}

		let selected_url = self.ctx.db().messages.find(arg).await.and_then(|message| {
			let in_scope =
				message.channel_id == channel_id && message.dm_thread_key == dm_thread_key;
			if in_scope && !message.image_url.is_empty() {
				Some(message.image_url)
			} else {
				None
			}
		});

		if let Some(url) = selected_url {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.selected_image_url = url;
			session.show_image_modal = true;
			self.ctx.pubsub().publish("rerender", ());
		}
	}

	pub(crate) fn close_image_modal(&mut self) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.show_image_modal = false;
		session.selected_image_url.clear();
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) fn open_create_channel(&mut self) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.show_create_channel = true;
	}

	pub(crate) fn close_create_channel(&mut self) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.show_create_channel = false;
	}

	pub(crate) fn edit_new_channel_name(&mut self, value: String) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.new_channel_name = value;
	}

	pub(crate) async fn create_channel(&mut self) {
		let channel_name = {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.new_channel_name.clone()
		};
		let trimmed = channel_name.trim().to_string();
		if trimmed.is_empty() {
			return;
		}

		let saved_channel = Channel {
			id: 0,
			name: trimmed,
			display_name: String::new(),
			messages: Vec::new(),
		}
		.save(&self.ctx.db)
		.await;

		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.active_kind = "channel".to_string();
		session.active_id = saved_channel.id;
		session.active_name = saved_channel.display_name;
		session.new_channel_name.clear();
		session.show_create_channel = false;
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) async fn select_channel(&mut self, arg: u32) {
		let selected = self
			.ctx
			.db()
			.channels
			.find(arg)
			.await
			.map(|channel| (channel.id, channel.display_name));
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		if let Some((id, name)) = selected {
			session.active_kind = "channel".to_string();
			session.active_id = id;
			session.active_name = name;
		}
	}

	pub(crate) async fn select_direct(&mut self, arg: u32) {
		let selected = self
			.ctx
			.db()
			.directs
			.find(arg)
			.await
			.map(|dm| (dm.id, dm.display_name));
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		if let Some((id, name)) = selected {
			session.active_kind = "dm".to_string();
			session.active_id = id;
			session.active_name = name;
		}
	}

	pub(crate) async fn send_message(&mut self) {
		let message = {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			let body = session.new_message.trim().to_string();
			if body.is_empty() || session.user_name.is_empty() {
				return;
			}
			let (channel_id, dm_thread_key) = self.message_scope(session);
			Message {
				id: 0,
				author: session.user_name.clone(),
				body,
				image_url: String::new(),
				time: "now".to_string(),
				channel_id,
				dm_thread_key,
			}
		};
		message.save(&self.ctx.db).await;

		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.new_message.clear();
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) async fn send_picture(&mut self) {
		let message = {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			let image_url = session.new_picture_url.trim().to_string();
			if image_url.is_empty() || session.user_name.is_empty() {
				return;
			}
			let (channel_id, dm_thread_key) = self.message_scope(session);
			Message {
				id: 0,
				author: session.user_name.clone(),
				body: String::new(),
				image_url,
				time: "now".to_string(),
				channel_id,
				dm_thread_key,
			}
		};
		message.save(&self.ctx.db).await;

		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.new_picture_url.clear();
		self.ctx.pubsub().publish("rerender", ());
	}
}

#[async_trait]
impl Component for Puppychat {
	type Context = SharedContext;
	type Db = PuppyDb;
	type Model = ChatViewState;

	async fn mount(ctx: Arc<Ctx<SharedContext, PuppyDb>>) -> Self {
		let _ = ctx.db().channels.find(1).await;
		Self::new(ctx)
	}

	fn render(&self, _ctx: &Ctx<SharedContext, PuppyDb>) -> Self::Model {
		self.state()
	}

	fn unmount(self, _ctx: Arc<Ctx<SharedContext, PuppyDb>>) {}
}
