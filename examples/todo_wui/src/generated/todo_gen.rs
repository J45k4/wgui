use wgui::*;

pub enum Action {
	EditNewTodo { value: String },
	AddTodo,
	ToggleTodo { arg: u32 },
}

pub fn decode(event: &wgui::ClientEvent) -> Option<Action> {
	match event {
		wgui::ClientEvent::OnTextChanged(ev) if ev.id == 1342686741 => Some(Action::EditNewTodo { value: ev.value.clone() }),
		wgui::ClientEvent::OnClick(ev) if ev.id == 3063551964 => Some(Action::AddTodo),
		wgui::ClientEvent::OnClick(ev) if ev.id == 1241325501 => ev.inx.map(|arg| Action::ToggleTodo { arg }),
		_ => None,
	}
}

pub fn render(state: &crate::TodoState) -> Item {
	let mut children = Vec::new();
	children.push({
		let mut items = Vec::new();
		items.push(wgui::text("Todo List"));
		items.push({
			let mut items = Vec::new();
			items.push(wgui::text_input().svalue(&state.new_todo_name).placeholder("What needs to be done?").id(1342686741));
			items.push(wgui::button("Add").id(3063551964));
			wgui::hstack(items)
			}.spacing(4));
		items.push({
			let mut items = Vec::new();
			for (i, item) in state.items.iter().enumerate() {
				items.push({
					let mut items = Vec::new();
					items.push(wgui::checkbox().checked(item.completed).id(1241325501).inx(item.id));
					items.push(wgui::text(&item.name));
					wgui::hstack(items)
					}.spacing(4));
			}
			wgui::vstack(items)
			}.spacing(4));
		wgui::vstack(items)
		}.spacing(8).padding(8));
	wgui::vstack(children)
}
