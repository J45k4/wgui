use std::collections::HashSet;
use log::Level;
use wgui::*;

const SELECT: u32 = 1;
const SLIDER: u32 = 2;
const TEXT_INPUT: u32 = 3;

#[derive(Default, Debug)]
struct State {
	option: String,
	text_input_value: String,
	slider_value: i32
}

fn render(state: &State) -> Item {
	log::info!("render state: {:?}", state);

	vstack([
		hstack([
			text("This is text1").grow(2).background_color("green").cursor("pointer").id(3),
			text("This is text2").grow(1).background_color("lightblue"),
		]).margin(20).padding(10).border("1px solid black"),
		text_input() //.placeholder("Enter text here")
		.id(TEXT_INPUT)
		.width(100)
		.svalue(&state.text_input_value)
		.margin_bottom(10),
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
		table([
			thead([
				tr([
					th(text("Header 1")),
					th(text("Header 2")),
				])
			]),
			tbody([
				tr([
					td(text("Row 1, Cell 1")).text_align("center"),
					td(text("Row 1, Cell 2")).text_align("center"),
				]),
				tr([
					td(text("Row 2, Cell 1")).text_align("center"),
					td(text("Row 2, Cell 2")).text_align("center"),
				]),
			])
		])
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
				if t.id == TEXT_INPUT {
					state.text_input_value = t.value;
				}
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