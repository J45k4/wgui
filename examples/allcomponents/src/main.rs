use log::Level;
use std::collections::HashSet;
use wgui::*;

const SELECT: u32 = 1;
const SLIDER: u32 = 2;
const TEXT_INPUT: u32 = 3;
const SHOW_TABLE_BUTTON: u32 = 4;
const TEXTAREA: u32 = 5;
const OPEN_MODAL_BUTTON: u32 = 6;
const CLOSE_MODAL_BUTTON: u32 = 7;
const MODAL_BACKDROP: u32 = 8;

#[derive(Default, Debug)]
struct State {
	option: String,
	text_input_value: String,
	slider_value: i32,
	show_table: bool,
	show_modal: bool,
}

fn render(state: &State) -> Item {
	log::info!("render state: {:?}", state);

	vstack([
		hstack([
			text("This is text1")
				.grow(2)
				.background_color("green")
				.cursor("pointer")
				.id(3),
			text("This is text2").grow(1).background_color("lightblue"),
		])
		.margin(20)
		.padding(10)
		.border("1px solid black")
		.editable(true),
		text_input() //.placeholder("Enter text here")
			.id(TEXT_INPUT)
			.width(100)
			.svalue(&state.text_input_value)
			.margin_bottom(10),
		select([
			option("Option 1", "option1"),
			option("Option 2", "option2"),
			option("Option 3", "option3"),
		])
		.id(SELECT)
		.svalue(&state.option)
		.width(100),
		slider()
			.id(SLIDER)
			.min(0)
			.max(100)
			.ivalue(state.slider_value)
			.width(100),
		textarea().placeholder("Enter text here").id(TEXTAREA),
		button("show table").id(SHOW_TABLE_BUTTON),
		button("open modal").id(OPEN_MODAL_BUTTON),
		if state.show_table {
			table([
				thead([tr([th(text("Header 1")), th(text("Header 2"))])]),
				tbody([
					tr([
						td(text("Row 1, Cell 1")).text_align("center"),
						td(text("Row 1, Cell 2")).text_align("center"),
					]),
					tr([
						td(text("Row 2, Cell 1")).text_align("center"),
						td(text("Row 2, Cell 2")).text_align("center"),
					]),
				]),
			])
		} else {
			text("Table is hidden")
		},
		modal([
			vstack([
				text("Modal heading").text_align("center"),
				text("This modal is rendered by the new component and can be dismissed from here or by clicking the backdrop."),
				hstack([
					button("close").id(CLOSE_MODAL_BUTTON),
				])
			])
			.padding(20)
			.spacing(12)
			.background_color("white")
			.width(340)
		])
		.id(MODAL_BACKDROP)
		.open(state.show_modal),
	])
	.into()
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();
	let mut wgui = Wgui::new("0.0.0.0:12346".parse().unwrap());
	let mut client_ids = HashSet::new();
	let mut state = State::default();

	while let Some(message) = wgui.next().await {
		log::info!("{:?}", message.event);

		let client_id = message.client_id;
		match message.event {
			ClientEvent::Disconnected { id: _ } => {
				client_ids.remove(&client_id);
			}
			ClientEvent::Connected { id: _ } => {
				wgui.render(client_id, render(&state)).await;
				client_ids.insert(client_id);
			}
			ClientEvent::PathChanged(_) => {}
			ClientEvent::Input(q) => {}
			ClientEvent::OnClick(o) => match o.id {
				SHOW_TABLE_BUTTON => {
					state.show_table = !state.show_table;
				}
				OPEN_MODAL_BUTTON => {
					state.show_modal = true;
				}
				CLOSE_MODAL_BUTTON | MODAL_BACKDROP => {
					state.show_modal = false;
				}
				_ => {}
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
