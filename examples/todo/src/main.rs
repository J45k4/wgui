use std::collections::HashSet;
use std::vec;

use log::Level;
use wgui::gui::*;
use wgui::types::ClientEvent;
use wgui::Wgui;

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

fn render(state: &TodoState) -> Item {
	vstack(vec![
		text("Todo List"),
		vstack(vec![
			hstack(vec![
				text_input().placeholder("What needs to be done?").name("new_todo_name").value(&state.new_todo_name).into(),
				button("Add").id("add_todo_button").into()
			]).spacing(3).id("add_todo").into(),
			vstack(
				state.items.iter().enumerate().map(|(i, item)| {
					hstack(vec![
						text(&item.name).into(),
						checkbox().id(&format!("todo_checkbox_{}", i)).checked(item.completed).into()
					])
					.border(&format!("1px solid {}", get_color(item.completed)))
					.background_color(&get_color(item.completed))
					.padding(10)
					.margin(5)
					.into()
				}).collect()
			).spacing(5).into()
		]).into()
	]).into()
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();

    let mut state = TodoState {
        new_todo_name: "".to_string(),
		slider_value: 0,
        items: vec![]
    };

    let mut client_ids = HashSet::new();

    let mut wgui = Wgui::new("0.0.0.0:12345".parse().unwrap());

    while let Some(event) = wgui.next().await {
        log::info!("{:?}", event);

        match event {
            ClientEvent::Disconnected { id } => {
                client_ids.remove(&id);
            },
            ClientEvent::Connected { id } => {
                wgui.render(id, render(&state)).await;
                client_ids.insert(id);
            },
            ClientEvent::PathChanged(_) => {},
            ClientEvent::Input(q) => {},
            ClientEvent::OnClick(o) => {
                if let Some(id) = o.id {
                    if id == "add_todo_button" {
                        log::info!("add_todo_button clicked");
                        state.items.push(TodoItems {
                            name: state.new_todo_name.clone(),
                            completed: false
                        });
                        state.new_todo_name = "".to_string();
                    }

                    if id == "todo_checkbox" {
                        log::info!("todo_checkbox clicked");
                        state.items[0].completed = !state.items[0].completed;
                    }

                    if id.starts_with("todo_checkbox_") {
                        log::info!("todo_checkbox_ clicked");
                        let inx = id.split("_").last().unwrap().parse::<usize>().unwrap();
                        state.items[inx].completed = !state.items[inx].completed;
                    }

					if id == "add_todo" {
						log::info!("add_todo clicked");
					}
                }
            },
            ClientEvent::OnTextChanged(t) => {
                if let Some(name) = t.name {
                    if name == "new_todo_name" {
                        log::info!("new_todo_name {:?}", t.value);
                        state.new_todo_name = t.value;
                    }
                }
            }
			ClientEvent::OnSliderChange(s) => {
				if s.id == "slider" {
					state.slider_value = s.value;
				}
			}
            _ => {}
        }

        for id in &client_ids {
            wgui.render(*id, render(&state)).await;
        }
    }
}
