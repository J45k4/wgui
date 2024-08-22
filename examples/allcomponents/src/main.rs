use std::collections::HashSet;

use log::Level;
use wgui::gui::select;
use wgui::gui::slider;
use wgui::gui::text;
use wgui::gui::vstack;
use wgui::gui::Item;
use wgui::types::ClientEvent;
use wgui::Wgui;

#[derive(Default, Debug)]
struct State {
	option: String,
	slider_value: i32
}

fn render(state: &State) -> Item {
	log::info!("render state: {:?}", state);

	vstack(vec![
		text("This is text"),
		select()
		.id("select")
		.value(&state.option)
		.add_option("", "")
		.add_option("Option 1", "option1")
		.add_option("Option 2", "option2")
		.add_option("Option 3", "option3")
		.into(),
		slider()
		.id("slider")
		.min(0).max(100)
		.value(state.slider_value)
		.width(100)
		.into(),
	]).into()
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();
	let mut wgui = Wgui::new("0.0.0.0:12346".parse().unwrap());
	let mut client_ids = HashSet::new();
	let mut state = State::default();


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
				
            },
            ClientEvent::OnTextChanged(t) => {

            }
			ClientEvent::OnSliderChange(s) => {
				if s.id == "slider" {
					state.slider_value = s.value;
				}
			}
			ClientEvent::OnSelect(o) => {
				if o.id == "select" {
					state.option = o.value;
				}
			}
            _ => {}
        }

        for id in &client_ids {
            wgui.render(*id, render(&state)).await;
        }
    }
}