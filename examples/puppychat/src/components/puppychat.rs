use crate::context::SharedContext;
use crate::{
	Channel, ChannelView, ChatViewState, DirectMessage, DirectMessageView, Message, PuppyDb,
	PushSubscription, Session, SessionState, User,
};
use async_trait::async_trait;
use std::collections::BTreeSet;
use std::sync::Arc;
use wgui::wgui_controller;
use wgui::wui::runtime::{Component, Ctx};

use crate::notifications::{NotificationChannel, PushPayload, PushTarget, WebPushSubscription};

const WEB_PUSH_SERVICE_WORKER_PATH: &str = "/assets/puppychat-sw.js";

pub struct Puppychat {
	ctx: Arc<Ctx<SharedContext, PuppyDb>>,
}

impl Puppychat {
	pub fn new(ctx: Arc<Ctx<SharedContext, PuppyDb>>) -> Self {
		ctx.set_title("Puppychat | chat");
		Self { ctx }
	}

	fn auth_session_key(&self) -> String {
		self.ctx
			.session_id()
			.unwrap_or_else(|| format!("client-{}", self.ctx.client_id().unwrap_or(0)))
	}

	fn ui_session_key(&self) -> String {
		format!(
			"{}::client:{}",
			self.auth_session_key(),
			self.ctx.client_id().unwrap_or(0)
		)
	}

	fn ensure_session_state<'a>(
		&self,
		sessions: &'a mut std::collections::HashMap<String, SessionState>,
	) -> &'a mut SessionState {
		let key = self.ui_session_key();
		let auth_key = self.auth_session_key();
		let default_channel = self
			.ctx
			.db()
			.channels
			.snapshot()
			.into_iter()
			.next()
			.map(|channel| (channel.id, channel.display_name));
		let db = self.ctx.db();
		let auth_key_for_load = auth_key.clone();
		sessions.entry(key).or_insert_with(|| {
			let mut state = SessionState::new(default_channel);
			if let Some(row) = db
				.sessions
				.snapshot()
				.into_iter()
				.find(|session| session.session_key == auth_key_for_load)
			{
				state.user_name = row.user_name.clone();
				if !state.user_name.is_empty() {
					Self::ensure_direct_entry(&db, &state.user_name);
				}
			}
			state
		})
	}

	fn message_scope(&self, session: &SessionState) -> (Option<u32>, Option<String>) {
		if session.active_kind == "channel" {
			return (Some(session.active_id), None);
		}
		if session.active_kind == "dm" {
			return (
				None,
				Self::dm_thread_key_for_session(self.ctx.db(), session),
			);
		}
		(None, None)
	}

	fn dm_thread_key(left: &str, right: &str) -> String {
		if left <= right {
			format!("{}|{}", left, right)
		} else {
			format!("{}|{}", right, left)
		}
	}

	fn ensure_direct_entry(db: &PuppyDb, user_name: &str) {
		let mut directs = db.direct_messages.snapshot();
		if directs.iter().any(|dm| dm.name == user_name) {
			return;
		}
		directs.push(DirectMessage {
			id: db.direct_messages.next_id(),
			name: user_name.to_string(),
			display_name: format!("@ {}", user_name),
			online: true,
			messages: "[]".to_string(),
		});
		db.direct_messages.replace(directs);
	}

	fn dm_thread_key_for_session(db: &PuppyDb, session: &SessionState) -> Option<String> {
		if session.active_kind != "dm" {
			return None;
		}
		let directs = db.direct_messages.snapshot();
		let other_name = directs
			.iter()
			.find(|dm| dm.id == session.active_id)
			.map(|dm| dm.name.clone())?;
		Some(Self::dm_thread_key(&session.user_name, &other_name))
	}

	fn call_room_for_session(db: &PuppyDb, session: &SessionState) -> Option<String> {
		if session.active_kind == "dm" {
			return Self::dm_thread_key_for_session(db, session).map(|key| format!("dm:{key}"));
		}
		if session.active_kind == "channel" && session.active_id != 0 {
			return Some(format!("channel:{}", session.active_id));
		}
		None
	}

	async fn find_user(db: &std::sync::Arc<PuppyDb>, name: &str) -> Option<User> {
		db.users
			.snapshot()
			.into_iter()
			.find(|user| user.name == name)
	}

	async fn persist_auth_session(
		db: &std::sync::Arc<PuppyDb>,
		session_key: &str,
		user_name: &str,
	) {
		let existing = db
			.sessions
			.snapshot()
			.into_iter()
			.find(|session| session.session_key == session_key);
		let mut row = existing.unwrap_or(Session {
			id: 0,
			session_key: session_key.to_string(),
			user_name: user_name.to_string(),
		});
		row.user_name = user_name.to_string();
		db.sessions.save(row).await;
	}

	fn notification_recipients(&self, message: &Message) -> Vec<String> {
		let mut recipients = BTreeSet::new();
		if let Some(channel_id) = message.channel_id {
			let _ = channel_id;
			for user in self.ctx.db().users.snapshot() {
				if user.name != message.author {
					recipients.insert(user.name);
				}
			}
			return recipients.into_iter().collect();
		}
		if let Some(thread) = message.dm_thread_key.as_deref() {
			for part in thread.split('|') {
				if !part.is_empty() && part != message.author {
					recipients.insert(part.to_string());
				}
			}
		}
		recipients.into_iter().collect()
	}

	async fn dispatch_push_for_message(&self, message: &Message) {
		let recipients = self.notification_recipients(message);
		if recipients.is_empty() {
			return;
		}
		let subscriptions = self.ctx.db().push_subscriptions.snapshot();
		let body = if message.body.is_empty() {
			"sent an image".to_string()
		} else {
			message.body.clone()
		};
		let payload = PushPayload {
			title: format!("New message from {}", message.author),
			body,
			thread: message
				.dm_thread_key
				.clone()
				.unwrap_or_else(|| format!("channel:{}", message.channel_id.unwrap_or(0))),
		};

		for user_name in recipients {
			for subscription in subscriptions
				.iter()
				.filter(|subscription| subscription.active && subscription.user_name == user_name)
				.cloned()
			{
				let Some(channel) = NotificationChannel::from_str(&subscription.channel) else {
					continue;
				};
				let target = PushTarget {
					channel,
					endpoint: subscription.endpoint.clone(),
					p256dh_key: subscription.p256dh_key.clone(),
					auth_key: subscription.auth_key.clone(),
				};
				if let Err(err) = self.ctx.state.push.send(&target, &payload).await {
					log::warn!(
						"failed to send push notification to user {} on {}: {}",
						user_name,
						subscription.channel,
						err
					);
				}
			}
		}
	}

	async fn upsert_web_push_subscription(&self, user_name: &str, data: &WebPushSubscription) {
		if user_name.is_empty() || data.endpoint.trim().is_empty() {
			return;
		}
		let endpoint = data.endpoint.trim().to_string();
		let existing = self
			.ctx
			.db()
			.push_subscriptions
			.snapshot()
			.into_iter()
			.find(|subscription| {
				subscription.channel == NotificationChannel::Web.as_str()
					&& subscription.endpoint == endpoint
			});
		let mut row = existing.unwrap_or(PushSubscription {
			id: 0,
			user_name: user_name.to_string(),
			channel: NotificationChannel::Web.as_str().to_string(),
			endpoint: endpoint.clone(),
			p256dh_key: String::new(),
			auth_key: String::new(),
			active: true,
			updated_at: "now".to_string(),
		});
		row.user_name = user_name.to_string();
		row.channel = NotificationChannel::Web.as_str().to_string();
		row.endpoint = endpoint;
		row.p256dh_key = data.keys.p256dh.clone();
		row.auth_key = data.keys.auth.clone();
		row.active = true;
		row.updated_at = "now".to_string();
		self.ctx.db().push_subscriptions.save(row).await;
	}

	fn remove_web_push_subscriptions(&self, user_name: &str) -> usize {
		let mut rows = self.ctx.db().push_subscriptions.snapshot();
		let before = rows.len();
		rows.retain(|subscription| {
			!(subscription.user_name == user_name
				&& subscription.channel == NotificationChannel::Web.as_str())
		});
		let removed = before.saturating_sub(rows.len());
		if removed > 0 {
			self.ctx.db().push_subscriptions.replace(rows);
		}
		removed
	}
}

#[wgui_controller]
impl Puppychat {
	pub fn state(&self) -> ChatViewState {
		let messages = self.ctx.db().messages.snapshot();
		let channels_base = self.ctx.db().channels.snapshot();
		let directs_base = self.ctx.db().direct_messages.snapshot();
		let channels = channels_base
			.into_iter()
			.map(|channel| ChannelView {
				id: channel.id,
				name: channel.name,
				display_name: channel.display_name,
				messages: messages
					.iter()
					.filter(|msg| msg.channel_id == Some(channel.id))
					.cloned()
					.collect(),
			})
			.collect::<Vec<_>>();

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
			call_active: session.call_active,
			call_with_video: session.call_with_video,
			call_room: Self::call_room_for_session(self.ctx.db(), session).unwrap_or_default(),
			push_status: session.push_status.clone(),
			web_push_sink: session.web_push_sink.clone(),
			channels,
			directs: {
				let mut directs = directs_base
					.iter()
					.filter(|dm| dm.name != user_name)
					.cloned()
					.map(|dm| {
						let key = Self::dm_thread_key(&user_name, &dm.name);
						let messages_for_dm = if user_name.is_empty() {
							Vec::new()
						} else {
							messages
								.iter()
								.filter(|msg| msg.dm_thread_key.as_deref() == Some(&key))
								.cloned()
								.collect()
						};
						DirectMessageView {
							id: dm.id,
							name: dm.name,
							display_name: dm.display_name,
							online: dm.online,
							messages: messages_for_dm,
						}
					})
					.collect::<Vec<_>>();
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
		let auth_session_key = self.auth_session_key();
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

		match Self::find_user(&self.ctx.db, &name).await {
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

		let user_name = {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.user_name = name;
			let user_name = session.user_name.clone();
			session.login_name.clear();
			session.login_password.clear();
			session.auth_error.clear();
			user_name
		};
		Self::ensure_direct_entry(self.ctx.db(), &user_name);
		Self::persist_auth_session(&self.ctx.db, &auth_session_key, &user_name).await;
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

		if Self::find_user(&self.ctx.db, &name).await.is_some() {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.auth_error = "username already exists".to_string();
			return;
		}

		self.ctx
			.db()
			.users
			.insert(User {
				name: name.clone(),
				password,
			})
			.await;

		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.login_name = name.clone();
		session.login_password.clear();
		session.auth_error = "account created, please login".to_string();
		Self::ensure_direct_entry(self.ctx.db(), &name);
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
		let saved_message = self.ctx.db().messages.save(message).await;
		self.dispatch_push_for_message(&saved_message).await;

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

		let display_name = if trimmed.starts_with('#') {
			trimmed.clone()
		} else {
			format!("# {}", trimmed)
		};
		let saved_channel = self
			.ctx
			.db()
			.channels
			.save(Channel {
				id: 0,
				name: trimmed,
				display_name,
				messages: "[]".to_string(),
			})
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
			session.call_active = false;
		}
	}

	pub(crate) async fn select_direct(&mut self, arg: u32) {
		let selected = self
			.ctx
			.db()
			.direct_messages
			.find(arg)
			.await
			.map(|dm| (dm.id, dm.display_name));
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		if let Some((id, name)) = selected {
			session.active_kind = "dm".to_string();
			session.active_id = id;
			session.active_name = name;
			session.call_active = false;
		}
	}

	pub(crate) fn start_video_call(&mut self) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		if session.active_kind.is_empty() {
			return;
		}
		session.call_active = true;
		session.call_with_video = true;
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) fn start_audio_call(&mut self) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		if session.active_kind.is_empty() {
			return;
		}
		session.call_active = true;
		session.call_with_video = false;
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) fn end_call(&mut self) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.call_active = false;
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) fn enable_push_notifications(&mut self) {
		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		if session.user_name.is_empty() {
			session.push_status = "login first to enable notifications".to_string();
		} else {
			session.push_status = "waiting for browser push permission...".to_string();
			self.ctx.enable_web_push(WEB_PUSH_SERVICE_WORKER_PATH, None);
		}
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) async fn disable_push_notifications(&mut self) {
		let has_user = {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			!session.user_name.is_empty()
		};
		if !has_user {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.push_status = "login first to disable notifications".to_string();
			self.ctx.pubsub().publish("rerender", ());
			return;
		}

		self.ctx.disable_web_push(WEB_PUSH_SERVICE_WORKER_PATH);

		let mut sessions = self.ctx.state.sessions.lock().unwrap();
		let session = self.ensure_session_state(&mut sessions);
		session.push_status = "disabling push notifications...".to_string();
		self.ctx.pubsub().publish("rerender", ());
	}

	pub(crate) async fn register_web_push_subscription(&mut self, value: String) {
		let raw = value.trim().to_string();
		let user_name = {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.web_push_sink = value;
			session.user_name.clone()
		};
		if user_name.is_empty() {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.push_status = "login first to register push".to_string();
			session.web_push_sink.clear();
			self.ctx.pubsub().publish("rerender", ());
			return;
		}
		if raw.is_empty() {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.web_push_sink.clear();
			return;
		}

		match serde_json::from_str::<WebPushSubscription>(&raw) {
			Ok(subscription) => {
				self.upsert_web_push_subscription(&user_name, &subscription)
					.await;
				let mut sessions = self.ctx.state.sessions.lock().unwrap();
				let session = self.ensure_session_state(&mut sessions);
				session.push_status = "web push subscription saved".to_string();
				session.web_push_sink.clear();
				self.ctx.pubsub().publish("rerender", ());
			}
			Err(err) => {
				let mut sessions = self.ctx.state.sessions.lock().unwrap();
				let session = self.ensure_session_state(&mut sessions);
				session.push_status = format!("invalid web push subscription payload: {}", err);
				session.web_push_sink.clear();
				self.ctx.pubsub().publish("rerender", ());
			}
		}
	}

	pub(crate) async fn handle_event(&mut self, event: &wgui::ClientEvent) -> bool {
		let wgui::ClientEvent::WebPushSubscriptionChanged(change) = event else {
			return false;
		};

		let user_name = {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.user_name.clone()
		};
		if user_name.is_empty() {
			let mut sessions = self.ctx.state.sessions.lock().unwrap();
			let session = self.ensure_session_state(&mut sessions);
			session.push_status = "login first to register push".to_string();
			self.ctx.pubsub().publish("rerender", ());
			return true;
		}

		match change.subscription.clone() {
			Some(raw_subscription) => {
				match serde_json::from_value::<WebPushSubscription>(raw_subscription) {
					Ok(subscription) => {
						self.upsert_web_push_subscription(&user_name, &subscription)
							.await;
						let mut sessions = self.ctx.state.sessions.lock().unwrap();
						let session = self.ensure_session_state(&mut sessions);
						session.push_status = "web push subscription saved".to_string();
					}
					Err(err) => {
						let mut sessions = self.ctx.state.sessions.lock().unwrap();
						let session = self.ensure_session_state(&mut sessions);
						session.push_status =
							format!("invalid web push subscription payload: {}", err);
					}
				}
			}
			None => {
				let removed = self.remove_web_push_subscriptions(&user_name);
				let mut sessions = self.ctx.state.sessions.lock().unwrap();
				let session = self.ensure_session_state(&mut sessions);
				session.push_status = if removed > 0 {
					"push notifications disabled".to_string()
				} else {
					"no active push subscriptions found".to_string()
				};
			}
		}

		self.ctx.pubsub().publish("rerender", ());
		true
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
		let saved_message = self.ctx.db().messages.save(message).await;
		self.dispatch_push_for_message(&saved_message).await;

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
		let saved_message = self.ctx.db().messages.save(message).await;
		self.dispatch_push_for_message(&saved_message).await;

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
