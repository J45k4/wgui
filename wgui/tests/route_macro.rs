//! Compile-only test for the `#[route]` macro and `Wgui::add_route`
//! registration. Verifies that:
//!
//! - `#[route("/path")]` generates a marker struct + RouteHandler impl
//! - `pub const <fn>_route` is the public handle
//! - `Wgui::add_route(<fn>_route)` stores the handler
//! - Path params (`:id`) map 1:1 to typed fn args (`id: u32`)
//! - Both sync and async fns are supported
//! - GET (default) and POST handlers both compile
//!
//! We don't dispatch through the WS loop here — that's the Phase 5 wiring
//! which is deferred to a follow-up. This test only proves the macro
//! surface compiles and registers without runtime behavior.

use std::sync::Arc;
use wgui::wui::runtime::Ctx;
use wgui::{
	partial, route, view, HttpMethod, PathParams, Redirect, RouteFormData, RouteHandler,
	RouteResult, RuntimeContext, View, Wgui,
};

#[derive(Default)]
struct TestAppState {
	counter: std::sync::Mutex<u32>,
}

#[derive(serde::Deserialize)]
struct CreateTodoForm {
	name: String,
}

// Page render, GET, async, no params
#[route("/")]
async fn page_index(ctx: &Ctx<TestAppState>) -> View {
	let _ = ctx;
	View::page("Home", wgui::gui::text("hi"))
}

// Page render with one path param, sync fn
#[route("/todos/:id")]
fn page_show(ctx: &Ctx<TestAppState>, id: u32) -> View {
	let _ = (ctx, id);
	View::page("Show", wgui::gui::text("todo"))
}

// Action mutation: POST + Redirect
#[route("/todos/create", method = "POST")]
async fn action_create(ctx: &Ctx<TestAppState>) -> Redirect {
	let _ = ctx;
	Redirect::to("/todos")
}

// Action with path param + POST
#[route("/todos/:id/toggle", method = "POST")]
async fn action_toggle(ctx: &Ctx<TestAppState>, id: u32) -> Redirect {
	let _ = id;
	*ctx.state.counter.lock().unwrap() += 1;
	Redirect::to("/todos")
}

// String path params
#[route("/users/:name")]
async fn page_user(ctx: &Ctx<TestAppState>, name: String) -> View {
	let _ = (ctx, name);
	View::page("User", wgui::gui::text("user"))
}

// Fallback wildcard
#[route("/*")]
fn page_not_found(ctx: &Ctx<TestAppState>) -> View {
	let _ = ctx;
	View::page("Not Found", wgui::gui::text("404"))
}

// Action returning () (no redirect): supported via From<()> → redirect to ""
#[route("/silent", method = "POST")]
async fn action_silent(ctx: &Ctx<TestAppState>) {
	let _ = ctx;
}

// A non-path argument on a POST route is decoded from URL-encoded form data.
#[route("/todos/form-create", method = "POST")]
async fn action_form_create(ctx: &Ctx<TestAppState>, form: CreateTodoForm) -> Redirect {
	*ctx.state.counter.lock().unwrap() = form.name.len() as u32;
	Redirect::to("/todos")
}

#[partial("/todos/:id/status")]
fn todo_status(_ctx: &Ctx<TestAppState>, id: u32) -> View {
	View::partial(wgui::partial_region(
		format!("/todos/{id}/status"),
		wgui::gui::text("ready"),
	))
}

#[route("/macro-view", view, template = "pages/route_macro/index")]
fn macro_view(_ctx: &Ctx<TestAppState>) -> View {
	view!({
		greeting: "hello",
		nested: { enabled: true },
	})
}

#[route(
	"/macro-form-error",
	method = "POST",
	template = "pages/route_macro/index"
)]
fn macro_form_error(_ctx: &Ctx<TestAppState>) -> View {
	view!({
		greeting: "invalid form",
		nested: { enabled: false },
	})
	.with_status(422)
}

#[test]
fn routes_register_and_stored_by_path() {
	// `add_route` doesn't require a context — it just stores the marker.
	// Dispatch (which needs the ctx) is Phase 5 wiring, deferred.
	let mut wgui = Wgui::new_without_server();

	wgui.add_route(page_index_route);
	wgui.add_route(page_show_route);
	wgui.add_route(action_create_route);
	wgui.add_route(action_toggle_route);
	wgui.add_route(page_user_route);
	wgui.add_route(page_not_found_route);
	wgui.add_route(action_silent_route);
	wgui.add_route(action_form_create_route);
	wgui.add_partial(todo_status_partial);
	wgui.add_route(macro_view_route);
	wgui.add_route(macro_form_error_route);

	// No runtime assertion on length — routes field is pub(crate).
	// Successful registration (no panic, traits resolve) is the contract.
}

#[test]
fn const_handles_implement_route_handler() {
	// Marker types implement RouteHandler — verify the trait methods return
	// the expected path/method without invoking dispatch.
	assert_eq!(page_index_route.path(), "/");
	assert_eq!(page_index_route.method(), HttpMethod::Get);

	assert_eq!(page_show_route.path(), "/todos/:id");
	assert_eq!(page_show_route.method(), HttpMethod::Get);

	assert_eq!(action_create_route.path(), "/todos/create");
	assert_eq!(action_create_route.method(), HttpMethod::Post);

	assert_eq!(action_toggle_route.path(), "/todos/:id/toggle");
	assert_eq!(action_toggle_route.method(), HttpMethod::Post);

	assert_eq!(page_user_route.path(), "/users/:name");
	assert_eq!(page_not_found_route.path(), "/*");
	assert_eq!(action_silent_route.path(), "/silent");
	assert_eq!(action_silent_route.method(), HttpMethod::Post);
	assert_eq!(action_form_create_route.method(), HttpMethod::Post);
	assert_eq!(todo_status_partial.path(), "/todos/:id/status");
	assert_eq!(macro_view_route.path(), "/macro-view");
	assert_eq!(macro_form_error_route.path(), "/macro-form-error");
	assert_eq!(macro_form_error_route.method(), HttpMethod::Post);
}

#[tokio::test]
async fn post_view_route_renders_the_template_and_retains_its_status() {
	let ctx = Arc::new(Ctx::new(TestAppState::default()));
	let result = wgui::DynRouteHandler::call_dyn(
		&macro_form_error_route,
		ctx,
		PathParams::default(),
		RouteFormData::default(),
		RuntimeContext {
			client_id: None,
			session: None,
			route: Some(wgui::wui::runtime::RouteContext {
				path: "/macro-form-error".to_string(),
				params: Default::default(),
				query: Default::default(),
			}),
		},
	)
	.await;
	let RouteResult::View(view) = result else {
		panic!("form validation route should render a view");
	};

	assert_eq!(view.status, 422);
	assert!(serde_json::to_string(&view.item)
		.unwrap()
		.contains("invalid form"));
}

#[tokio::test]
async fn view_macro_renders_anonymous_model_with_registered_template() {
	let ctx = Arc::new(Ctx::new(TestAppState::default()));
	let result = wgui::DynRouteHandler::call_dyn(
		&macro_view_route,
		ctx,
		PathParams::default(),
		RouteFormData::default(),
		RuntimeContext {
			client_id: None,
			session: None,
			route: Some(wgui::wui::runtime::RouteContext {
				path: "/macro-view".to_string(),
				params: Default::default(),
				query: Default::default(),
			}),
		},
	)
	.await;
	let RouteResult::View(view) = result else {
		panic!("view route should render a view");
	};
	let rendered = serde_json::to_string(&view.item).unwrap();
	assert!(rendered.contains("hello"));
	assert!(rendered.contains("true"));
}

#[tokio::test]
async fn post_route_decodes_typed_form_argument() {
	let ctx = Arc::new(Ctx::new(TestAppState::default()));
	let result = action_form_create_route
		.call(
			ctx.clone(),
			PathParams::default(),
			RouteFormData::from_urlencoded(b"name=puppy"),
		)
		.await;

	assert!(matches!(result, RouteResult::Redirect(_)));
	assert_eq!(*ctx.state.counter.lock().unwrap(), 5);
}

#[test]
fn pascal_case_marker_names_are_generated() {
	// All marker structs are zero-sized; construction is free.
	let _ = page_index_route;
	let _ = page_show_route;
	let _ = action_create_route;
	let _ = action_toggle_route;
	let _ = page_user_route;
	let _ = page_not_found_route;
	let _ = action_silent_route;
	let _ = action_form_create_route;
	let _ = todo_status_partial;
	let _ = macro_view_route;
	let _ = macro_form_error_route;
}
