use log::Level;
use std::collections::HashSet;
use std::vec;
use wgui::*;

struct TodoItems {
	name: String,
	completed: bool,
}

struct TodoState {
	new_todo_name: String,
	slider_value: i32,
	items: Vec<TodoItems>,
}

fn get_color(completed: bool) -> String {
	if completed {
		"#d3d3d3".to_string()
	} else {
		"#ffffff".to_string()
	}
}

const ADD_TODO_ID: u32 = 1;
const TODO_CHECKBOX_ID: u32 = 2;
const NEW_TODO_TEXT_ID: u32 = 3;

fn render(state: &TodoState) -> Item {
	vstack([
		text("Todo List"),
		vstack([
			hstack([
				text_input()
					.id(NEW_TODO_TEXT_ID)
					.placeholder("What needs to be done?")
					.svalue(&state.new_todo_name),
				button("Add").id(ADD_TODO_ID),
			])
			.spacing(3),
			vstack(state.items.iter().enumerate().map(|(i, item)| {
				hstack([
					text(&item.name),
					checkbox()
						.id(TODO_CHECKBOX_ID)
						.inx(i as u32)
						.checked(item.completed),
				])
				.border(&format!("1px solid {}", get_color(item.completed)))
				.background_color(&get_color(item.completed))
				.padding(10)
				.margin(5)
			}))
			.spacing(5),
		]),
	])
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();

	let mut state = TodoState {
		new_todo_name: "".to_string(),
		slider_value: 0,
		items: vec![],
	};

	let mut client_ids = HashSet::new();

	let mut wgui = Wgui::new("0.0.0.0:12345".parse().unwrap());

	while let Some(event) = wgui.next().await {
		log::info!("{:?}", event);

		match event {
			ClientEvent::Disconnected { id } => {
				client_ids.remove(&id);
			}
			ClientEvent::Connected { id } => {
				wgui.render(id, render(&state)).await;
				client_ids.insert(id);
			}
			ClientEvent::PathChanged(_) => {}
			ClientEvent::Input(q) => {}
			ClientEvent::OnClick(o) => match o.id {
				ADD_TODO_ID => {
					log::info!("add_todo_button clicked");
					state.items.push(TodoItems {
						name: state.new_todo_name.clone(),
						completed: false,
					});
					state.new_todo_name = "".to_string();
				}
				TODO_CHECKBOX_ID => {
					if let Some(inx) = o.inx {
						log::info!("todo_checkbox_{} clicked", inx);
						state.items[inx as usize].completed = !state.items[inx as usize].completed;
					}
				}
				_ => {}
			},
			ClientEvent::OnTextChanged(t) => match t.id {
				NEW_TODO_TEXT_ID => {
					log::info!("new_todo_name {:?}", t.value);
					state.new_todo_name = t.value;
				}
				_ => {}
			},
			ClientEvent::OnSliderChange(s) => {
				state.slider_value = s.value;
			}
			_ => {}
		}

		for id in &client_ids {
			wgui.render(*id, render(&state)).await;
		}
	}
}
