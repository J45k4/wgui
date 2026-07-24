use crate::context::SharedContext;
use crate::{PuppyDb, Session};
use serde::Deserialize;
use wgui::wui::runtime::Ctx;
use wgui::{Redirect, RouteResult, View, route, view};

#[derive(Deserialize)]
pub struct LoginForm {
	name: String,
	password: String,
}

fn login_view(name: String, error: String, status: u16) -> View {
	view!({ name: name, error: error }).with_status(status)
}

fn register_view(name: String, error: String, status: u16) -> View {
	view!({ name: name, error: error }).with_status(status)
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
	session.user_name = name;
	ctx.db().sessions.save(session).await;

	Redirect::to("/").into()
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
