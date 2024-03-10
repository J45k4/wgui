use std::collections::HashSet;
use std::vec;

use log::Level;
use wgui::gui::Button;
use wgui::gui::Item;
use wgui::gui::Text;
use wgui::gui::TextInput;
use wgui::gui::View;
use wgui::gui::H1;
use wgui::types::ClientEvent;
use wgui::Wgui;
use wgui::WguiBuilder;

struct TodoItems {
    name: String,
    completed: bool,
}

struct TodoState {
    new_todo_name: String,
    items: Vec<TodoItems>,
}

fn render(state: &TodoState) -> Item {
    Item::View(View {
        body: vec![
            Item::H1(H1{ text: "Todo List".to_string()}),
            Item::View(View { 
                body: vec![
                    Item::View(View { 
                        body: vec![
                            Item::TextInput(TextInput {
                                placeholder: "What needs to be done?".to_string(),
                                name: "new_todo_name".to_string(),
                                value: state.new_todo_name.clone(),
                                ..Default::default()
                            }),
                            Item::Button(Button {
                                title: "Add".to_string(),
                                id: Some("add_todo_button".to_string()),
                                ..Default::default()
                            })
                        ],
                        ..Default::default()
                    }),
                    Item::View(View {
                        body: state.items.iter().enumerate().map(|(i, item)| {
                            Item::View(View {
                                body: vec![
                                    Item::Text(Text {
                                        text: item.name.clone()
                                    }),
                                    Item::Checkbox(wgui::gui::Checkbox {
                                        id: format!("todo_checkbox_{}", i),
                                        checked: item.completed,
                                        ..Default::default()
                                    })
                                ],
                                ..Default::default()
                            })
                        }).collect(),
                        ..Default::default()
                    })
                ],
                ..Default::default()
            })
        ],
        ..Default::default()
    })
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();

    let mut state = TodoState {
        new_todo_name: "".to_string(),
        items: vec![]
    };

    let mut client_ids = HashSet::new();

    let mut wgui = WguiBuilder::new().build();
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
                }
            },
            ClientEvent::OnTextChanged(t) => {
                // log::info!("OnTextChanged {:?}", t);
                if let Some(name) = t.name {
                    if name == "new_todo_name" {
                        log::info!("new_todo_name {:?}", t.value);
                        state.new_todo_name = t.value;
                    }
                }
            }
            _ => {}
        }

        for id in &client_ids {
            wgui.render(*id, render(&state)).await;
        }
    }
}
