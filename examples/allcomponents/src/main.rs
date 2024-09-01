use std::collections::HashSet;

use log::Level;
use wgui::gui::option;
use wgui::gui::select;
use wgui::gui::slider;
use wgui::gui::table;
use wgui::gui::text;
use wgui::gui::tr;
use wgui::gui::vstack;
use wgui::gui::Item;
use wgui::types::ClientEvent;
use wgui::Wgui;

const SELECT: u32 = 1;
const SLIDER: u32 = 2;

#[derive(Default, Debug)]
struct State {
	option: String,
	slider_value: i32
}

fn render(state: &State) -> Item {
	log::info!("render state: {:?}", state);

	vstack([
		text("This is text"),
		select([
			option("Option 1", "option1"),
			option("Option 2", "option2"),
			option("Option 3", "option3")
		]).id(SELECT).svalue(&state.option).width(100),
		slider()
			.id(SLIDER)
			.min(0).max(100)
			.ivalue(state.slider_value)
			.width(100),
		table(
			[
				text("Header 1"),
				text("Header 2"),
			],
			[
				[
					text("row1 col1"),
					text("row1 col2"),
				],
				[
					text("row2 col1"),
					text("row2 col2"),
				],
			]
		)
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
				if s.id == SLIDER {
					state.slider_value = s.value;
				}
			}
			ClientEvent::OnSelect(o) => {
				if o.id == SELECT {
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