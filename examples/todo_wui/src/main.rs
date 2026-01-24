use log::Level;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime};
use tokio::sync::mpsc;
use wgui::*;
use wgui::wui::runtime::{RuntimeAction, Template, WuiValue, WuiValueProvider};

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
			(
				"completed".to_string(),
				WuiValue::Bool(self.completed),
			),
		])
	}
}

#[derive(Debug, Default)]
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

fn load_template(path: &Path, module_name: &str) -> Template {
	let source = fs::read_to_string(path).unwrap_or_else(|err| {
		panic!("failed to read {}: {}", path.display(), err);
	});
	match Template::parse(&source, module_name) {
		Ok(template) => template,
		Err(diags) => {
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

fn file_mtime(path: &Path) -> SystemTime {
	fs::metadata(path)
		.and_then(|meta| meta.modified())
		.unwrap_or(SystemTime::UNIX_EPOCH)
}

fn watch_template(path: PathBuf, tx: mpsc::UnboundedSender<()>) {
	let mut last_mtime = file_mtime(&path);
	loop {
		thread::sleep(Duration::from_millis(250));
		let mtime = file_mtime(&path);
		if mtime > last_mtime {
			last_mtime = mtime;
			let _ = tx.send(());
		}
	}
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let mut state = TodoState::default();
	let mut client_ids = HashSet::new();
	let mut next_id: u32 = 1;

	let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());
	let template_path = Path::new(&manifest_dir).join("wui/pages/todo.wui");
	let mut template = load_template(&template_path, "todo");
	let (reload_tx, mut reload_rx) = mpsc::unbounded_channel();
	thread::spawn({
		let path = template_path.clone();
		move || watch_template(path, reload_tx)
	});

	let mut wgui = Wgui::new("0.0.0.0:12345".parse().unwrap());

	loop {
		tokio::select! {
			event = wgui.next() => {
				let Some(event) = event else { break; };
				match event {
					ClientEvent::Disconnected { id } => {
						client_ids.remove(&id);
					}
					ClientEvent::Connected { id } => {
						wgui.render(id, template.render(&state)).await;
						client_ids.insert(id);
					}
					ClientEvent::PathChanged(_) => {}
					ClientEvent::Input(_) => {}
					_ => {
						if let Some(action) = template.decode(&event) {
							match action {
								RuntimeAction::Click { name, arg } => match name.as_str() {
									"AddTodo" => {
										if !state.new_todo_name.trim().is_empty() {
											state.items.push(TodoItem {
												id: next_id,
												name: state.new_todo_name.trim().to_string(),
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
						template = new_template;
						for id in &client_ids {
							wgui.render(*id, template.render(&state)).await;
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

		let done = state.items.iter().filter(|item| item.completed).count();
		let undone = state.items.len() - done;
		let title = format!("Todo {} done / {} undone", done, undone);

		for id in &client_ids {
			wgui.set_title(*id, &title).await;
			wgui.render(*id, template.render(&state)).await;
		}
	}
}
