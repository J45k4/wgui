use std::collections::HashMap;

use wgui::wui::runtime::WuiController;
use wgui::{text, wgui_controller, FormData, HttpCtx, HttpRequest, HttpResponse, Json};

#[derive(Debug, serde::Deserialize)]
struct LoginBody {
	name: String,
}

struct TestController;

#[wgui_controller]
impl TestController {
	fn render(&self) -> wgui::Item {
		text("test")
	}

	#[wgui_post("/form")]
	async fn form(&mut self, form: FormData, ctx: HttpCtx) -> HttpResponse {
		HttpResponse::new(
			200,
			format!(
				"{}:{}",
				form.get("name").unwrap_or(""),
				ctx.params.get("id").map(String::as_str).unwrap_or("")
			),
		)
	}

	#[wgui_post("/json")]
	fn json(&mut self, Json(body): Json<LoginBody>) -> HttpResponse {
		HttpResponse::new(200, body.name)
	}

	#[wgui_post("/raw")]
	fn raw(&mut self, req: HttpRequest) -> HttpResponse {
		HttpResponse::new(200, req.body)
	}
}

fn request(path: &str, content_type: &str, body: impl Into<Vec<u8>>) -> HttpRequest {
	let mut headers = HashMap::new();
	headers.insert("content-type".to_string(), content_type.to_string());
	HttpRequest {
		method: "POST".to_string(),
		path: path.to_string(),
		query: HashMap::new(),
		headers,
		body: body.into(),
	}
}

fn ctx() -> HttpCtx {
	let mut params = HashMap::new();
	params.insert("id".to_string(), "42".to_string());
	HttpCtx {
		path: "/form".to_string(),
		params,
		query: HashMap::new(),
		headers: HashMap::new(),
		session: Some("session".to_string()),
	}
}

#[test]
fn controller_declares_post_routes() {
	let routes = TestController::http_routes();
	assert_eq!(routes.len(), 3);
	assert_eq!(routes[0].method, "POST");
	assert_eq!(routes[0].path, "/form");
}

#[tokio::test]
async fn post_handler_extracts_form_data_and_context() {
	let route = TestController::http_routes()
		.into_iter()
		.find(|route| route.path == "/form")
		.unwrap();
	let mut controller = TestController;
	let response = controller
		.handle_http(
			route.id,
			request("/form", "application/x-www-form-urlencoded", "name=puppy"),
			ctx(),
		)
		.await
		.unwrap();

	assert_eq!(response.status, 200);
	assert_eq!(response.body, b"puppy:42");
}

#[tokio::test]
async fn post_handler_extracts_json() {
	let route = TestController::http_routes()
		.into_iter()
		.find(|route| route.path == "/json")
		.unwrap();
	let mut controller = TestController;
	let response = controller
		.handle_http(
			route.id,
			request("/json", "application/json", br#"{"name":"puppy"}"#),
			ctx(),
		)
		.await
		.unwrap();

	assert_eq!(response.status, 200);
	assert_eq!(response.body, b"puppy");
}

#[tokio::test]
async fn post_handler_can_accept_raw_request() {
	let route = TestController::http_routes()
		.into_iter()
		.find(|route| route.path == "/raw")
		.unwrap();
	let mut controller = TestController;
	let response = controller
		.handle_http(route.id, request("/raw", "text/plain", "raw-body"), ctx())
		.await
		.unwrap();

	assert_eq!(response.status, 200);
	assert_eq!(response.body, b"raw-body");
}
