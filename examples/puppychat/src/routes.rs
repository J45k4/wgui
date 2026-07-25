use crate::context::SharedContext;
use crate::{
	Channel, ChannelView, ChatViewState, DirectMessageView, Message, PuppyDb, Session, SessionState,
};
use serde::Deserialize;
use wgui::wui::runtime::Ctx;
use wgui::{Redirect, RouteResult, View, partial, route, view};

#[derive(Deserialize)]
pub struct LoginForm {
	name: String,
	password: String,
}

#[derive(Deserialize)]
pub struct SendMessageForm {
	body: String,
	active_kind: String,
	active_id: u32,
}

#[derive(Deserialize)]
pub struct IdForm {
	id: u32,
}

#[derive(Deserialize)]
pub struct ChannelForm {
	name: String,
}

#[derive(Deserialize)]
pub struct ImageForm {
	image_url: Option<String>,
}

fn login_view(name: String, error: String, status: u16) -> View {
	view!({ name: name, error: error }).with_status(status)
}

fn register_view(name: String, error: String, status: u16) -> View {
	view!({ name: name, error: error }).with_status(status)
}

fn session_key(ctx: &Ctx<SharedContext, PuppyDb>) -> String {
	ctx.session_id()
		.unwrap_or_else(|| format!("client-{}", ctx.client_id().unwrap_or(0)))
}

fn session_state<'a>(
	ctx: &Ctx<SharedContext, PuppyDb>,
	sessions: &'a mut std::collections::HashMap<String, SessionState>,
) -> &'a mut SessionState {
	let auth_key = session_key(ctx);
	// Keep SSR and websocket requests on the same state entry. Both use the
	// authenticated session id, while only websocket requests have a client id.
	let key = auth_key.clone();
	let default_channel = ctx
		.db()
		.channels
		.snapshot()
		.into_iter()
		.next()
		.map(|channel| (channel.id, channel.display_name))
		.or(Some((1, "# general".to_string())));
	let db = ctx.db();
	sessions.entry(key).or_insert_with(|| {
		let mut state = SessionState::new(default_channel);
		if let Some(row) = db
			.sessions
			.snapshot()
			.into_iter()
			.find(|session| session.session_key == auth_key)
		{
			state.user_name = row.user_name;
		}
		state
	})
}

fn chat_state(ctx: &Ctx<SharedContext, PuppyDb>, conversation_page: bool) -> ChatViewState {
	let messages = ctx.db().messages.snapshot();
	let mut channels = ctx.db().channels.snapshot();
	if channels.is_empty() {
		channels.push(Channel {
			id: 1,
			name: "general".to_string(),
			display_name: "# general".to_string(),
			messages: "[]".to_string(),
		});
	}
	let mut sessions = ctx.state.sessions.lock().unwrap();
	let session = session_state(ctx, &mut sessions);
	let user_name = session.user_name.clone();
	let channels = channels
		.into_iter()
		.map(|channel| ChannelView {
			id: channel.id,
			name: channel.name,
			display_name: channel.display_name,
			href: format!("/channel/{}", channel.id),
			messages: messages
				.iter()
				.filter(|message| message.channel_id == Some(channel.id))
				.cloned()
				.collect(),
		})
		.collect();
	let directs = ctx
		.db()
		.direct_messages
		.snapshot()
		.into_iter()
		.filter(|dm| dm.name != user_name)
		.map(|dm| {
			let thread = if user_name <= dm.name {
				format!("{}|{}", user_name, dm.name)
			} else {
				format!("{}|{}", dm.name, user_name)
			};
			DirectMessageView {
				id: dm.id,
				name: dm.name,
				display_name: dm.display_name,
				href: format!("/direct/{}", dm.id),
				online: dm.online,
				messages: messages
					.iter()
					.filter(|message| message.dm_thread_key.as_deref() == Some(&thread))
					.cloned()
					.collect(),
			}
		})
		.collect();
	ChatViewState {
		user_name,
		login_name: session.login_name.clone(),
		login_password: session.login_password.clone(),
		auth_error: session.auth_error.clone(),
		new_message: String::new(),
		new_picture_url: String::new(),
		new_channel_name: String::new(),
		show_create_channel: session.show_create_channel,
		show_attach_menu: session.show_attach_menu,
		show_image_modal: session.show_image_modal,
		selected_image_url: session.selected_image_url.clone(),
		active_kind: session.active_kind.clone(),
		active_id: session.active_id,
		active_name: session.active_name.clone(),
		call_active: session.call_active,
		call_with_video: session.call_with_video,
		call_room: if session.active_kind == "channel" {
			format!("channel:{}", session.active_id)
		} else {
			String::new()
		},
		push_status: session.push_status.clone(),
		shell_class: if conversation_page {
			"puppychat-shell puppychat-conversation-page".to_string()
		} else {
			"puppychat-shell puppychat-list-page".to_string()
		},
		message_partial_addr: format!(
			"/chat/messages/{}/{}",
			session.active_kind, session.active_id
		),
		channels,
		directs,
	}
}

fn update_session(ctx: &Ctx<SharedContext, PuppyDb>, update: impl FnOnce(&mut SessionState)) {
	let mut sessions = ctx.state.sessions.lock().unwrap();
	update(session_state(ctx, &mut sessions));
}

#[route("/", view, template = "puppychat")]
pub async fn page_chat(ctx: &Ctx<SharedContext, PuppyDb>) -> View {
	let _ = ctx.db().channels.find(1).await;
	let state = chat_state(ctx, false);
	view!(state)
}

#[route("/channel/:id", view, template = "puppychat")]
pub async fn page_channel(ctx: &Ctx<SharedContext, PuppyDb>, id: u32) -> RouteResult {
	let Some(channel) = ctx.db().channels.find(id).await else {
		return Redirect::to("/").into();
	};
	update_session(ctx, |session| {
		session.active_kind = "channel".to_string();
		session.active_id = channel.id;
		session.active_name = channel.display_name;
		session.call_active = false;
	});
	view!(chat_state(ctx, true)).into()
}

#[route("/direct/:id", view, template = "puppychat")]
pub async fn page_direct(ctx: &Ctx<SharedContext, PuppyDb>, id: u32) -> RouteResult {
	let Some(dm) = ctx.db().direct_messages.find(id).await else {
		return Redirect::to("/").into();
	};
	update_session(ctx, |session| {
		session.active_kind = "dm".to_string();
		session.active_id = dm.id;
		session.active_name = dm.display_name;
		session.call_active = false;
	});
	view!(chat_state(ctx, true)).into()
}

#[partial("/chat/messages/:kind/:id", template = "messages")]
pub fn message_list(ctx: &Ctx<SharedContext, PuppyDb>, kind: String, id: u32) -> View {
	view!(chat_state(ctx, true).with_message_target(kind, id))
}

#[route("/login", view)]
pub fn page_login(_ctx: &Ctx<SharedContext, PuppyDb>) -> View {
	login_view(String::new(), String::new(), 200)
}

#[route("/login", method = "POST", template = "pages/login/index")]
pub async fn login(ctx: &Ctx<SharedContext, PuppyDb>, form: LoginForm) -> RouteResult {
	let name = form.name.trim().to_string();
	if name.is_empty() || form.password.trim().is_empty() {
		return login_view(name, "username and password are required".to_string(), 422).into();
	}

	match ctx
		.db()
		.users
		.snapshot()
		.into_iter()
		.find(|user| user.name == name)
	{
		Some(user) if user.password == form.password => {}
		Some(_) => {
			return login_view(name, "invalid username or password".to_string(), 422).into();
		}
		None => {
			return login_view(name, "account not found, register first".to_string(), 422).into();
		}
	}

	let session_key = ctx
		.session_id()
		.expect("POST routes always receive an HTTP or websocket session id");
	let existing = ctx
		.db()
		.sessions
		.snapshot()
		.into_iter()
		.find(|session| session.session_key == session_key);
	let mut session = existing.unwrap_or(Session {
		id: 0,
		session_key,
		user_name: name.clone(),
	});
	session.user_name = name.clone();
	ctx.db().sessions.save(session).await;
	update_session(ctx, |ui_session| {
		ui_session.user_name = name.clone();
	});

	Redirect::to("/").into()
}

#[route("/logout", method = "POST")]
pub async fn logout(ctx: &Ctx<SharedContext, PuppyDb>) -> RouteResult {
	if let Some(session_key) = ctx.session_id() {
		if let Some(session) = ctx
			.db()
			.sessions
			.snapshot()
			.into_iter()
			.find(|session| session.session_key == session_key)
		{
			ctx.db().sessions.delete(session.id).await;
		}
	}
	update_session(ctx, |session| {
		session.user_name.clear();
		session.login_name.clear();
		session.login_password.clear();
		session.auth_error.clear();
		session.show_create_channel = false;
		session.show_attach_menu = false;
		session.show_image_modal = false;
		session.selected_image_url.clear();
		session.call_active = false;
	});
	Redirect::to("/login").into()
}

#[route("/register", view)]
pub fn page_register(_ctx: &Ctx<SharedContext, PuppyDb>) -> View {
	register_view(String::new(), String::new(), 200)
}

#[route("/register", method = "POST", template = "pages/register/index")]
pub async fn register(ctx: &Ctx<SharedContext, PuppyDb>, form: LoginForm) -> RouteResult {
	let name = form.name.trim().to_string();
	if name.is_empty() || form.password.trim().is_empty() {
		return register_view(name, "username and password are required".to_string(), 422).into();
	}

	if ctx
		.db()
		.users
		.snapshot()
		.into_iter()
		.any(|user| user.name == name)
	{
		return register_view(name, "username already exists".to_string(), 422).into();
	}

	ctx.db()
		.users
		.insert(crate::User {
			name,
			password: form.password,
		})
		.await;

	Redirect::to("/login").into()
}

#[route("/messages", method = "POST")]
pub async fn send_message(ctx: &Ctx<SharedContext, PuppyDb>, form: SendMessageForm) -> RouteResult {
	let Some(session_key) = ctx.session_id() else {
		return Redirect::to("/login").into();
	};
	let Some(session) = ctx
		.db()
		.sessions
		.snapshot()
		.into_iter()
		.find(|session| session.session_key == session_key)
	else {
		return Redirect::to("/login").into();
	};

	let body = form.body.trim().to_string();
	if body.is_empty() {
		return Redirect::to("").into();
	}
	let (channel_id, dm_thread_key) = if form.active_kind == "dm" {
		let Some(other) = ctx.db().direct_messages.find(form.active_id).await else {
			return Redirect::to("").into();
		};
		let thread = if session.user_name <= other.name {
			format!("{}|{}", session.user_name, other.name)
		} else {
			format!("{}|{}", other.name, session.user_name)
		};
		(None, Some(thread))
	} else {
		let channel_id = ctx
			.db()
			.channels
			.snapshot()
			.into_iter()
			.find(|channel| channel.id == form.active_id)
			.or_else(|| ctx.db().channels.snapshot().into_iter().next())
			.map(|channel| channel.id);
		(channel_id, None)
	};

	ctx.db()
		.messages
		.save(Message {
			id: 0,
			author: session.user_name,
			body,
			image_url: String::new(),
			time: "now".to_string(),
			channel_id,
			dm_thread_key,
		})
		.await;
	let topic_kind = if form.active_kind == "dm" {
		"dm"
	} else {
		"channel"
	};
	let topic_id = channel_id.unwrap_or(form.active_id);
	ctx.render(format!("/chat/messages/{topic_kind}/{topic_id}"));

	Redirect::to("").into()
}

#[route("/chat/channels/new", method = "POST")]
pub fn open_create_channel(ctx: &Ctx<SharedContext, PuppyDb>) -> Redirect {
	update_session(ctx, |session| session.show_create_channel = true);
	Redirect::to("")
}

#[route("/chat/channels/cancel", method = "POST")]
pub fn close_create_channel(ctx: &Ctx<SharedContext, PuppyDb>) -> Redirect {
	update_session(ctx, |session| session.show_create_channel = false);
	Redirect::to("")
}

#[route("/chat/channels", method = "POST")]
pub async fn create_channel(ctx: &Ctx<SharedContext, PuppyDb>, form: ChannelForm) -> Redirect {
	let name = form.name.trim().to_string();
	if name.is_empty() {
		return Redirect::to("");
	}
	let display_name = if name.starts_with('#') {
		name.clone()
	} else {
		format!("# {name}")
	};
	let channel = ctx
		.db()
		.channels
		.save(Channel {
			id: 0,
			name,
			display_name,
			messages: "[]".to_string(),
		})
		.await;
	update_session(ctx, |session| {
		session.active_kind = "channel".to_string();
		session.active_id = channel.id;
		session.active_name = channel.display_name;
		session.show_create_channel = false;
	});
	Redirect::to(format!("/channel/{}", channel.id))
}

#[route("/chat/call/audio", method = "POST")]
pub fn start_audio_call(ctx: &Ctx<SharedContext, PuppyDb>) -> Redirect {
	update_session(ctx, |session| {
		if !session.active_kind.is_empty() {
			session.call_active = true;
			session.call_with_video = false;
		}
	});
	Redirect::to("")
}

#[route("/chat/call/video", method = "POST")]
pub fn start_video_call(ctx: &Ctx<SharedContext, PuppyDb>) -> Redirect {
	update_session(ctx, |session| {
		if !session.active_kind.is_empty() {
			session.call_active = true;
			session.call_with_video = true;
		}
	});
	Redirect::to("")
}

#[route("/chat/call/end", method = "POST")]
pub fn end_call(ctx: &Ctx<SharedContext, PuppyDb>) -> Redirect {
	update_session(ctx, |session| session.call_active = false);
	Redirect::to("")
}

#[route("/chat/attachments/open", method = "POST")]
pub fn open_attach_menu(ctx: &Ctx<SharedContext, PuppyDb>) -> Redirect {
	update_session(ctx, |session| session.show_attach_menu = true);
	Redirect::to("")
}

#[route("/chat/attachments/cancel", method = "POST")]
pub fn close_attach_menu(ctx: &Ctx<SharedContext, PuppyDb>) -> Redirect {
	update_session(ctx, |session| session.show_attach_menu = false);
	Redirect::to("")
}

#[route("/chat/attachments", method = "POST")]
pub async fn send_picture(ctx: &Ctx<SharedContext, PuppyDb>, form: ImageForm) -> Redirect {
	let image_url = form.image_url.unwrap_or_default().trim().to_string();
	if image_url.is_empty() {
		return Redirect::to("");
	}
	let (author, active_kind, active_id) = {
		let mut sessions = ctx.state.sessions.lock().unwrap();
		let session = session_state(ctx, &mut sessions);
		(
			session.user_name.clone(),
			session.active_kind.clone(),
			session.active_id,
		)
	};
	if author.is_empty() {
		return Redirect::to("/login");
	}
	let (channel_id, dm_thread_key) = if active_kind == "dm" {
		let Some(other) = ctx.db().direct_messages.find(active_id).await else {
			return Redirect::to("");
		};
		let thread = if author <= other.name {
			format!("{author}|{}", other.name)
		} else {
			format!("{}|{author}", other.name)
		};
		(None, Some(thread))
	} else {
		(Some(active_id), None)
	};
	ctx.db()
		.messages
		.save(Message {
			id: 0,
			author,
			body: String::new(),
			image_url,
			time: "now".to_string(),
			channel_id,
			dm_thread_key,
		})
		.await;
	ctx.render(format!("/chat/messages/{active_kind}/{active_id}"));
	update_session(ctx, |session| session.show_attach_menu = false);
	Redirect::to("")
}

#[route("/chat/images/open", method = "POST")]
pub async fn open_message_image(ctx: &Ctx<SharedContext, PuppyDb>, form: IdForm) -> Redirect {
	if let Some(message) = ctx.db().messages.find(form.id).await
		&& !message.image_url.is_empty()
	{
		update_session(ctx, |session| {
			session.selected_image_url = message.image_url;
			session.show_image_modal = true;
		});
	}
	Redirect::to("")
}

#[route("/chat/images/close", method = "POST")]
pub fn close_image_modal(ctx: &Ctx<SharedContext, PuppyDb>) -> Redirect {
	update_session(ctx, |session| {
		session.show_image_modal = false;
		session.selected_image_url.clear();
	});
	Redirect::to("")
}
