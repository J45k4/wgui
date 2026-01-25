use log::Level;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use wgui::wui::runtime::{
	load_template, spawn_template_watcher, RuntimeAction, TemplateLoadError, WuiValue, WuiValueProvider,
};
use wgui::*;

#[derive(Debug, Clone)]
struct TodoItem {
	id: u32,
	name: String,
	completed: bool,
}

impl TodoItem {
	fn to_wui_value(&self) -> WuiValue {
		WuiValue::object(vec![
			("id".to_string(), WuiValue::Number(self.id as f64)),
			("name".to_string(), WuiValue::String(self.name.clone())),
			("completed".to_string(), WuiValue::Bool(self.completed)),
		])
	}
}

#[derive(Debug, Default, Clone)]
struct TodoState {
	new_todo_name: String,
	items: Vec<TodoItem>,
}

impl WuiValueProvider for TodoState {
	fn wui_value(&self) -> WuiValue {
		WuiValue::object(vec![
			(
				"new_todo_name".to_string(),
				WuiValue::String(self.new_todo_name.clone()),
			),
			(
				"items".to_string(),
				WuiValue::List(self.items.iter().map(|item| item.to_wui_value()).collect()),
			),
		])
	}
}

fn load_template_or_panic(path: &Path, module_name: &str) -> Template {
	match load_template(path, module_name) {
		Ok(template) => template,
		Err(TemplateLoadError::Io(err)) => {
			panic!("failed to read {}: {}", path.display(), err);
		}
		Err(TemplateLoadError::Diagnostics(diags)) => {
			for diag in diags {
				eprintln!(
					"template error: {} at {}..{}",
					diag.message, diag.span.start, diag.span.end
				);
			}
			panic!("failed to parse template");
		}
	}
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let state = Arc::new(RwLock::new(TodoState::default()));
	let mut client_ids = HashSet::new();
	let mut next_id: u32 = 1;

	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
	let template_path = Path::new(&manifest_dir).join("wui/pages/todo.wui");
	let template = Arc::new(RwLock::new(load_template_or_panic(&template_path, "todo")));
	let (reload_tx, mut reload_rx) = mpsc::unbounded_channel();
	let _watcher = spawn_template_watcher(template_path.clone(), reload_tx);

	let ssr_state = state.clone();
	let ssr_template = template.clone();
	let mut wgui = Wgui::new_with_ssr(
		"0.0.0.0:12345".parse().unwrap(),
		Arc::new(move || {
			let state = ssr_state.read().unwrap();
			let template = ssr_template.read().unwrap();
			template.render(&*state)
		}),
	);

	loop {
		tokio::select! {
			event = wgui.next() => {
				let Some(event) = event else { break; };
				match event {
					ClientEvent::Disconnected { id } => {
						client_ids.remove(&id);
					}
					ClientEvent::Connected { id } => {
						let template = template.read().unwrap();
						let state = state.read().unwrap();
						wgui.render(id, template.render(&*state)).await;
						client_ids.insert(id);
					}
					ClientEvent::PathChanged(_) => {}
					ClientEvent::Input(_) => {}
					_ => {
						let action = {
							let template = template.read().unwrap();
							template.decode(&event)
						};
						if let Some(action) = action {
							let mut state = state.write().unwrap();
							match action {
								RuntimeAction::Click { name, arg } => match name.as_str() {
									"AddTodo" => {
										let name = state.new_todo_name.trim().to_string();
										if !name.is_empty() {
											state.items.push(TodoItem {
												id: next_id,
												name,
												completed: false,
											});
											next_id += 1;
										}
										state.new_todo_name.clear();
									}
									"ToggleTodo" => {
										if let Some(arg) = arg {
											if let Some(item) = state.items.iter_mut().find(|item| item.id == arg) {
												item.completed = !item.completed;
											}
										}
									}
									_ => {}
								},
								RuntimeAction::TextChanged { name, value } => {
									if name == "EditNewTodo" {
										state.new_todo_name = value;
									}
								}
								_ => {}
							}
						}
					}
				}
			}
			Some(()) = reload_rx.recv() => {
				let source = match fs::read_to_string(&template_path) {
					Ok(source) => source,
					Err(err) => {
						log::warn!("failed to read {}: {}", template_path.display(), err);
						continue;
					}
				};
				match Template::parse(&source, "todo") {
					Ok(new_template) => {
						log::info!("reloaded template {}", template_path.display());
						*template.write().unwrap() = new_template;
						for id in &client_ids {
							let template = template.read().unwrap();
							let state = state.read().unwrap();
							wgui.render(*id, template.render(&*state)).await;
						}
					}
					Err(diags) => {
						for diag in diags {
							log::warn!("template error: {} at {}..{}", diag.message, diag.span.start, diag.span.end);
						}
					}
				}
			}
		}

		let (done, undone) = {
			let state = state.read().unwrap();
			let done = state.items.iter().filter(|item| item.completed).count();
			let undone = state.items.len() - done;
			(done, undone)
		};
		let title = format!("Todo {} done / {} undone", done, undone);

		for id in &client_ids {
			wgui.set_title(*id, &title).await;
			let template = template.read().unwrap();
			let state = state.read().unwrap();
			wgui.render(*id, template.render(&*state)).await;
		}
	}
}
