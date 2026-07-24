use std::collections::HashMap;

fn is_default<T: Default + PartialEq>(value: &T) -> bool {
	value == &T::default()
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlexDirection {
	Column,
	Row,
}

impl Default for FlexDirection {
	fn default() -> Self {
		FlexDirection::Column
	}
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
enum Value {
	String(String),
	Bool(bool),
	Undefined,
}

impl Default for Value {
	fn default() -> Self {
		Value::Undefined
	}
}

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Pos {
	x: u32,
	y: u32,
}

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Layout {
	#[serde(skip_serializing_if = "is_default")]
	pub body: Vec<Item>,
	#[serde(skip_serializing_if = "is_default")]
	pub flex: FlexDirection,
	#[serde(skip_serializing_if = "is_default")]
	pub spacing: u32,
	#[serde(skip_serializing_if = "is_default")]
	pub wrap: bool,
	#[serde(skip_serializing_if = "is_default")]
	pub horizontal_resize: bool,
	#[serde(skip_serializing_if = "is_default")]
	pub vresize: bool,
	#[serde(skip_serializing_if = "is_default")]
	pub hresize: bool,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub pos: Option<Pos>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub events: Option<LayoutEvents>,
}

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutEvents {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub scroll_near_bottom: Option<u32>,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ItemPayload {
	Layout(Layout),
	Form {
		action: String,
		method: String,
		spacing: u32,
		body: Vec<Item>,
	},
	Text {
		value: String,
	},
	TextInput {
		value: String,
		placeholder: String,
		#[serde(rename = "inputType")]
		input_type: String,
	},
	DatePicker {
		value: String,
		placeholder: String,
	},
	Textarea {
		value: String,
		placeholder: String,
	},
	Select {
		value: String,
		options: Vec<SelectOption>,
	},
	Checkbox {
		checked: bool,
	},
	Slider {
		min: i32,
		max: i32,
		value: i32,
		step: i32,
	},
	Button {
		title: String,
		#[serde(skip_serializing_if = "Option::is_none")]
		events: Option<ButtonEvents>,
	},
	Link {
		href: String,
		text: String,
	},
	Table {
		items: Vec<Item>,
	},
	Tbody {
		items: Vec<Item>,
	},
	Thead {
		items: Vec<Item>,
	},
	Tr {
		items: Vec<Item>,
	},
	Th {
		item: Box<Item>,
	},
	Td {
		item: Box<Item>,
	},
	Img {
		src: String,
		alt: String,
		object_fit: Option<String>,
	},
	Video {
		room: String,
		local: bool,
		autoplay: bool,
		muted: bool,
		controls: bool,
	},
	Audio {
		room: String,
		local: bool,
		autoplay: bool,
		muted: bool,
		controls: bool,
	},
	FolderPicker,
	FloatingLayout {
		x: u32,
		y: u32,
		width: u32,
		height: u32,
	},
	Modal {
		body: Vec<Item>,
		open: bool,
	},
	ConnectionStatus {
		connected: bool,
		flex: FlexDirection,
		spacing: u32,
		wrap: bool,
		body: Vec<Item>,
	},
	Custom {
		name: String,
		entry: String,
		props: serde_json::Value,
		#[serde(default, skip_serializing_if = "HashMap::is_empty")]
		events: HashMap<String, u32>,
	},
	None,
}

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ButtonEvents {
	pub click: Option<u32>,
	pub press: Option<u32>,
	pub release: Option<u32>,
	pub repeat: Option<u32>,
	pub repeat_interval: Option<u32>,
}

impl Default for ItemPayload {
	fn default() -> Self {
		ItemPayload::None
	}
}

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct Item {
	#[serde(skip_serializing_if = "is_default")]
	pub id: u32,
	#[serde(skip_serializing_if = "is_default")]
	pub inx: u32,
	pub payload: ItemPayload,
	#[serde(skip_serializing_if = "is_default")]
	pub border: String,
	#[serde(skip_serializing_if = "is_default")]
	pub background_color: String,
	#[serde(skip_serializing_if = "is_default")]
	pub color: String,
	#[serde(skip_serializing_if = "is_default")]
	pub cursor: String,
	#[serde(skip_serializing_if = "is_default")]
	pub break_words: bool,
	#[serde(skip_serializing_if = "is_default")]
	pub fill: bool,
	#[serde(skip_serializing_if = "is_default")]
	pub height: u32,
	#[serde(skip_serializing_if = "is_default")]
	pub width: u32,
	#[serde(skip_serializing_if = "is_default")]
	pub min_height: u32,
	#[serde(skip_serializing_if = "is_default")]
	pub max_height: u32,
	#[serde(skip_serializing_if = "is_default")]
	pub min_width: u32,
	#[serde(skip_serializing_if = "is_default")]
	pub max_width: u32,
	#[serde(skip_serializing_if = "is_default")]
	pub grow: u32,
	#[serde(skip_serializing_if = "is_default")]
	pub text_align: String,
	#[serde(skip_serializing_if = "is_default")]
	pub white_space: String,
	#[serde(skip_serializing_if = "is_default")]
	pub margin: u16,
	#[serde(skip_serializing_if = "is_default")]
	pub margin_left: u16,
	#[serde(skip_serializing_if = "is_default")]
	pub margin_right: u16,
	#[serde(skip_serializing_if = "is_default")]
	pub margin_top: u16,
	#[serde(skip_serializing_if = "is_default")]
	pub margin_bottom: u16,
	#[serde(skip_serializing_if = "is_default")]
	pub padding: u16,
	#[serde(skip_serializing_if = "is_default")]
	pub padding_left: u16,
	#[serde(skip_serializing_if = "is_default")]
	pub padding_right: u16,
	#[serde(skip_serializing_if = "is_default")]
	pub padding_top: u16,
	#[serde(skip_serializing_if = "is_default")]
	pub padding_bottom: u16,
	#[serde(skip_serializing_if = "is_default")]
	pub overflow: String,
	#[serde(skip_serializing_if = "is_default")]
	pub editable: bool,
	#[serde(skip_serializing_if = "is_default")]
	pub name: String,
	#[serde(skip_serializing_if = "is_default")]
	pub action: String,
	#[serde(skip_serializing_if = "is_default")]
	pub method: String,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub form_arg: Option<u32>,
	#[serde(skip_serializing_if = "is_default")]
	pub partial_addr: String,
}

pub fn checkbox() -> Item {
	Item {
		payload: ItemPayload::Checkbox { checked: false },
		..Default::default()
	}
}

pub fn vstack<I>(body: I) -> Item
where
	I: IntoIterator<Item = Item>,
{
	Item {
		payload: ItemPayload::Layout(Layout {
			body: body.into_iter().collect(),
			flex: FlexDirection::Column,
			..Default::default()
		}),
		..Default::default()
	}
}

pub fn hstack<I>(body: I) -> Item
where
	I: IntoIterator<Item = Item>,
{
	Item {
		payload: ItemPayload::Layout(Layout {
			body: body.into_iter().collect(),
			flex: FlexDirection::Row,
			..Default::default()
		}),
		..Default::default()
	}
}

pub fn form<I>(body: I) -> Item
where
	I: IntoIterator<Item = Item>,
{
	Item {
		payload: ItemPayload::Form {
			action: String::new(),
			method: String::from("post"),
			spacing: 0,
			body: body.into_iter().collect(),
		},
		..Default::default()
	}
}

/// Mark an item subtree as a re-renderable partial region.
///
/// `addr` is the concrete address later passed to [`Ctx::render`]. The
/// region remains an ordinary item tree in the browser; only the server uses
/// this metadata to scope re-renders to clients currently viewing it.
pub fn partial_region(addr: impl Into<String>, mut item: Item) -> Item {
	item.partial_addr = addr.into();
	item
}

pub fn button(title: &str) -> Item {
	Item {
		payload: ItemPayload::Button {
			title: title.to_string(),
			events: None,
		},
		..Default::default()
	}
}

pub fn link(href: &str, text: &str) -> Item {
	Item {
		payload: ItemPayload::Link {
			href: href.to_string(),
			text: text.to_string(),
		},
		..Default::default()
	}
}

pub fn text(text: &str) -> Item {
	Item {
		payload: ItemPayload::Text {
			value: text.to_string(),
		},
		..Default::default()
	}
}

pub fn text_input() -> Item {
	Item {
		payload: ItemPayload::TextInput {
			value: "".to_string(),
			placeholder: "".to_string(),
			input_type: "text".to_string(),
		},
		..Default::default()
	}
}

pub fn date_picker() -> Item {
	Item {
		payload: ItemPayload::DatePicker {
			value: "".to_string(),
			placeholder: "".to_string(),
		},
		..Default::default()
	}
}

pub fn textarea() -> Item {
	Item {
		payload: ItemPayload::Textarea {
			value: "".to_string(),
			placeholder: "".to_string(),
		},
		..Default::default()
	}
}

pub fn modal<I>(body: I) -> Item
where
	I: IntoIterator<Item = Item>,
{
	Item {
		payload: ItemPayload::Modal {
			body: body.into_iter().collect(),
			open: true,
		},
		..Default::default()
	}
}

pub fn connected<I>(body: I) -> Item
where
	I: IntoIterator<Item = Item>,
{
	Item {
		payload: ItemPayload::ConnectionStatus {
			connected: true,
			flex: FlexDirection::Column,
			spacing: 0,
			wrap: false,
			body: body.into_iter().collect(),
		},
		..Default::default()
	}
}

pub fn disconnected<I>(body: I) -> Item
where
	I: IntoIterator<Item = Item>,
{
	Item {
		payload: ItemPayload::ConnectionStatus {
			connected: false,
			flex: FlexDirection::Column,
			spacing: 0,
			wrap: false,
			body: body.into_iter().collect(),
		},
		..Default::default()
	}
}

pub fn custom_component(
	name: impl Into<String>,
	entry: impl Into<String>,
	props: serde_json::Value,
) -> Item {
	Item {
		payload: ItemPayload::Custom {
			name: name.into(),
			entry: entry.into(),
			props,
			events: HashMap::new(),
		},
		..Default::default()
	}
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelectOption {
	value: String,
	name: String,
}

pub fn option(value: &str, name: &str) -> SelectOption {
	SelectOption {
		value: value.to_string(),
		name: name.to_string(),
	}
}

impl SelectOption {
	pub fn value(&self) -> &str {
		&self.value
	}

	pub fn name(&self) -> &str {
		&self.name
	}
}

pub fn select<I>(options: I) -> Item
where
	I: IntoIterator<Item = SelectOption>,
{
	Item {
		payload: ItemPayload::Select {
			value: "".to_string(),
			options: options.into_iter().collect(),
		},
		..Default::default()
	}
}

pub fn slider() -> Item {
	Item {
		payload: ItemPayload::Slider {
			min: 0,
			max: 100,
			value: 0,
			step: 1,
		},
		..Default::default()
	}
}

pub fn table<T>(body: T) -> Item
where
	T: IntoIterator<Item = Item>,
{
	Item {
		payload: ItemPayload::Table {
			items: body.into_iter().collect(),
		},
		..Default::default()
	}
}

pub fn thead<T>(items: T) -> Item
where
	T: IntoIterator<Item = Item>,
{
	Item {
		payload: ItemPayload::Thead {
			items: items.into_iter().collect(),
		},
		..Default::default()
	}
}

pub fn tbody<T>(items: T) -> Item
where
	T: IntoIterator<Item = Item>,
{
	Item {
		payload: ItemPayload::Tbody {
			items: items.into_iter().collect(),
		},
		..Default::default()
	}
}

pub fn tr<T>(items: T) -> Item
where
	T: IntoIterator<Item = Item>,
{
	Item {
		payload: ItemPayload::Tr {
			items: items.into_iter().collect(),
		},
		..Default::default()
	}
}

pub fn th(item: Item) -> Item {
	Item {
		payload: ItemPayload::Th {
			item: Box::new(item),
		},
		..Default::default()
	}
}

pub fn td(items: Item) -> Item {
	Item {
		payload: ItemPayload::Td {
			item: Box::new(items),
		},
		..Default::default()
	}
}

pub fn img(src: &str, alt: &str) -> Item {
	Item {
		payload: ItemPayload::Img {
			src: src.to_string(),
			alt: alt.to_string(),
			object_fit: None,
		},
		..Default::default()
	}
}

pub fn folder_picker() -> Item {
	Item {
		payload: ItemPayload::FolderPicker,
		..Default::default()
	}
}

pub fn video(room: &str) -> Item {
	Item {
		payload: ItemPayload::Video {
			room: room.to_string(),
			local: false,
			autoplay: true,
			muted: false,
			controls: false,
		},
		..Default::default()
	}
}

pub fn audio(room: &str) -> Item {
	Item {
		payload: ItemPayload::Audio {
			room: room.to_string(),
			local: false,
			autoplay: true,
			muted: false,
			controls: false,
		},
		..Default::default()
	}
}

impl Item {
	pub fn id(mut self, id: u32) -> Self {
		self.id = id;
		self
	}

	pub fn inx(mut self, inx: u32) -> Self {
		self.inx = inx;
		self
	}

	pub fn on_click(mut self, id: u32) -> Self {
		self = self.id(id);
		self.set_button_event(|events| events.click = Some(id))
	}

	pub fn on_press(self, id: u32) -> Self {
		self.set_button_event(|events| events.press = Some(id))
	}

	pub fn on_release(self, id: u32) -> Self {
		self.set_button_event(|events| events.release = Some(id))
	}

	pub fn on_repeat(self, id: u32) -> Self {
		self.set_button_event(|events| events.repeat = Some(id))
	}

	pub fn repeat_interval(self, interval: u32) -> Self {
		self.set_button_event(|events| events.repeat_interval = Some(interval))
	}

	pub fn on_scroll_near_bottom(self, id: u32) -> Self {
		self.set_layout_event(|events| events.scroll_near_bottom = Some(id))
	}

	pub fn custom_event(mut self, name: impl Into<String>, id: u32) -> Self {
		if let ItemPayload::Custom { events, .. } = &mut self.payload {
			events.insert(name.into(), id);
		}
		self
	}

	pub fn name(mut self, name: impl Into<String>) -> Self {
		self.name = name.into();
		self
	}

	pub fn action(mut self, action: impl Into<String>) -> Self {
		self.action = action.into();
		if let ItemPayload::Form { action, .. } = &mut self.payload {
			*action = self.action.clone();
		}
		self
	}

	pub fn method(mut self, method: impl Into<String>) -> Self {
		self.method = method.into();
		if let ItemPayload::Form { method, .. } = &mut self.payload {
			*method = self.method.clone();
		}
		self
	}

	pub fn partial_addr(mut self, addr: impl Into<String>) -> Self {
		self.partial_addr = addr.into();
		self
	}

	pub fn form_arg(mut self, arg: u32) -> Self {
		self.form_arg = Some(arg);
		self
	}

	fn set_button_event<F>(mut self, update: F) -> Self
	where
		F: FnOnce(&mut ButtonEvents),
	{
		if let ItemPayload::Button { events, .. } = &mut self.payload {
			update(events.get_or_insert_with(ButtonEvents::default));
		}
		self
	}

	fn set_layout_event<F>(mut self, update: F) -> Self
	where
		F: FnOnce(&mut LayoutEvents),
	{
		if let ItemPayload::Layout(layout) = &mut self.payload {
			update(layout.events.get_or_insert_with(LayoutEvents::default));
		}
		self
	}

	pub fn checked(mut self, checked: bool) -> Self {
		self.payload = ItemPayload::Checkbox { checked };
		self
	}

	pub fn min(mut self, m: i32) -> Self {
		match self.payload {
			ItemPayload::Slider { ref mut min, .. } => {
				*min = m;
			}
			_ => {}
		}
		self
	}

	pub fn max(mut self, m: i32) -> Self {
		match self.payload {
			ItemPayload::Slider { ref mut max, .. } => {
				*max = m;
			}
			_ => {}
		}
		self
	}

	pub fn ivalue(mut self, v: i32) -> Self {
		match self.payload {
			ItemPayload::Slider { ref mut value, .. } => {
				*value = v;
			}
			_ => {}
		}
		self
	}

	pub fn svalue(mut self, v: &str) -> Self {
		match self.payload {
			ItemPayload::Text { ref mut value, .. } => {
				*value = v.to_string();
			}
			ItemPayload::TextInput { ref mut value, .. } => {
				*value = v.to_string();
			}
			ItemPayload::DatePicker { ref mut value, .. } => {
				*value = v.to_string();
			}
			ItemPayload::Textarea { ref mut value, .. } => {
				*value = v.to_string();
			}
			ItemPayload::Select { ref mut value, .. } => {
				*value = v.to_string();
			}
			_ => {}
		}
		self
	}

	pub fn step(mut self, s: i32) -> Self {
		match self.payload {
			ItemPayload::Slider { ref mut step, .. } => {
				*step = s;
			}
			_ => {}
		}
		self
	}

	pub fn spacing(mut self, spacing: u32) -> Self {
		match self.payload {
			ItemPayload::Layout(ref mut layout) => {
				layout.spacing = spacing;
			}
			ItemPayload::Form {
				spacing: ref mut form_spacing,
				..
			} => {
				*form_spacing = spacing;
			}
			ItemPayload::ConnectionStatus {
				spacing: ref mut status_spacing,
				..
			} => {
				*status_spacing = spacing;
			}
			_ => {}
		}
		self
	}

	pub fn padding(mut self, padding: u16) -> Self {
		self.padding = padding;
		self
	}

	pub fn padding_left(mut self, padding_left: u16) -> Self {
		self.padding_left = padding_left;
		self
	}

	pub fn padding_right(mut self, padding_right: u16) -> Self {
		self.padding_right = padding_right;
		self
	}

	pub fn padding_top(mut self, padding_top: u16) -> Self {
		self.padding_top = padding_top;
		self
	}

	pub fn padding_bottom(mut self, padding_bottom: u16) -> Self {
		self.padding_bottom = padding_bottom;
		self
	}

	pub fn margin(mut self, margin: u16) -> Self {
		self.margin = margin;
		self
	}

	pub fn margin_left(mut self, margin_left: u16) -> Self {
		self.margin_left = margin_left;
		self
	}

	pub fn margin_right(mut self, margin_right: u16) -> Self {
		self.margin_right = margin_right;
		self
	}

	pub fn margin_top(mut self, margin_top: u16) -> Self {
		self.margin_top = margin_top;
		self
	}

	pub fn margin_bottom(mut self, margin_bottom: u16) -> Self {
		self.margin_bottom = margin_bottom;
		self
	}

	pub fn placeholder(mut self, p: &str) -> Self {
		match self.payload {
			ItemPayload::TextInput {
				ref mut placeholder,
				..
			} => {
				*placeholder = p.to_string();
			}
			ItemPayload::DatePicker {
				ref mut placeholder,
				..
			} => {
				*placeholder = p.to_string();
			}
			ItemPayload::Textarea {
				ref mut placeholder,
				..
			} => {
				*placeholder = p.to_string();
			}
			_ => {}
		}
		self
	}

	pub fn input_type(mut self, t: &str) -> Self {
		let sanitized = match t {
			"hidden" => "hidden",
			"password" => "password",
			_ => "text",
		};
		if let ItemPayload::TextInput {
			ref mut input_type, ..
		} = self.payload
		{
			*input_type = sanitized.to_string();
		}
		self
	}

	pub fn border(mut self, b: &str) -> Self {
		self.border = b.to_string();
		self
	}

	pub fn background_color(mut self, c: &str) -> Self {
		self.background_color = c.to_string();
		self
	}

	pub fn color(mut self, c: &str) -> Self {
		self.color = c.to_string();
		self
	}

	pub fn break_words(mut self, value: bool) -> Self {
		self.break_words = value;
		self
	}

	pub fn fill(mut self, value: bool) -> Self {
		self.fill = value;
		self
	}

	pub fn width(mut self, w: u32) -> Self {
		self.width = w;
		self
	}

	pub fn min_width(mut self, w: u32) -> Self {
		self.min_width = w;
		self
	}

	pub fn max_width(mut self, w: u32) -> Self {
		self.max_width = w;
		self
	}

	pub fn grow(mut self, g: u32) -> Self {
		self.grow = g;
		self
	}

	pub fn text_align(mut self, a: &str) -> Self {
		self.text_align = a.to_string();
		self
	}

	pub fn white_space(mut self, value: &str) -> Self {
		self.white_space = value.to_string();
		self
	}

	pub fn cursor(mut self, c: &str) -> Self {
		self.cursor = c.to_string();
		self
	}

	pub fn height(mut self, h: u32) -> Self {
		self.height = h;
		self
	}

	pub fn min_height(mut self, h: u32) -> Self {
		self.min_height = h;
		self
	}

	pub fn max_height(mut self, h: u32) -> Self {
		self.max_height = h;
		self
	}

	pub fn wrap(mut self, w: bool) -> Self {
		match self.payload {
			ItemPayload::Layout(ref mut layout) => {
				layout.wrap = w;
			}
			ItemPayload::ConnectionStatus {
				wrap: ref mut status_wrap,
				..
			} => {
				*status_wrap = w;
			}
			_ => {}
		}
		self
	}

	pub fn object_fit(mut self, fit: &str) -> Self {
		match self.payload {
			ItemPayload::Img {
				ref mut object_fit, ..
			} => {
				*object_fit = Some(fit.to_string());
			}
			_ => {}
		}
		self
	}

	pub fn room(mut self, room: &str) -> Self {
		match self.payload {
			ItemPayload::Video {
				room: ref mut r, ..
			}
			| ItemPayload::Audio {
				room: ref mut r, ..
			} => {
				*r = room.to_string();
			}
			_ => {}
		}
		self
	}

	pub fn local(mut self, local: bool) -> Self {
		match self.payload {
			ItemPayload::Video {
				local: ref mut value,
				..
			}
			| ItemPayload::Audio {
				local: ref mut value,
				..
			} => {
				*value = local;
			}
			_ => {}
		}
		self
	}

	pub fn autoplay(mut self, autoplay: bool) -> Self {
		match self.payload {
			ItemPayload::Video {
				autoplay: ref mut value,
				..
			}
			| ItemPayload::Audio {
				autoplay: ref mut value,
				..
			} => {
				*value = autoplay;
			}
			_ => {}
		}
		self
	}

	pub fn muted(mut self, muted: bool) -> Self {
		match self.payload {
			ItemPayload::Video {
				muted: ref mut value,
				..
			}
			| ItemPayload::Audio {
				muted: ref mut value,
				..
			} => {
				*value = muted;
			}
			_ => {}
		}
		self
	}

	pub fn controls(mut self, controls: bool) -> Self {
		match self.payload {
			ItemPayload::Video {
				controls: ref mut value,
				..
			}
			| ItemPayload::Audio {
				controls: ref mut value,
				..
			} => {
				*value = controls;
			}
			_ => {}
		}
		self
	}

	pub fn editable(mut self, e: bool) -> Self {
		self.editable = e;
		self
	}

	pub fn overflow(mut self, o: &str) -> Self {
		self.overflow = o.to_string();
		self
	}

	pub fn open(mut self, open: bool) -> Self {
		if let ItemPayload::Modal {
			open: ref mut is_open,
			..
		} = self.payload
		{
			*is_open = open;
		}
		self
	}

	pub fn hresize(mut self, r: bool) -> Self {
		match self.payload {
			ItemPayload::Layout(ref mut layout) => {
				layout.horizontal_resize = r;
			}
			_ => {}
		}
		self
	}

	pub fn vresize(mut self, r: bool) -> Self {
		match self.payload {
			ItemPayload::Layout(ref mut layout) => {
				layout.vresize = r;
			}
			_ => {}
		}
		self
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_vstack() {
		let view = vstack([hstack([text("Hello"), button("Click me")]), button("DUNNO")]);
	}

	#[test]
	fn text_input_serializes_input_type_as_camel_case() {
		let value = serde_json::to_value(text_input().input_type("password")).unwrap();

		assert_eq!(value["payload"]["inputType"], "password");
		assert!(value["payload"].get("input_type").is_none());
	}

	#[test]
	fn text_input_accepts_hidden_input_type() {
		let value = serde_json::to_value(text_input().input_type("hidden")).unwrap();

		assert_eq!(value["payload"]["inputType"], "hidden");
	}

	#[test]
	fn item_wire_format_omits_defaults_and_round_trips() {
		let item = vstack([text("Hello")]).padding(8);
		let value = serde_json::to_value(&item).unwrap();

		assert_eq!(value["padding"], 8);
		assert!(value.get("border").is_none());
		assert!(value.get("margin").is_none());
		assert!(value.get("editable").is_none());
		assert!(value["payload"].get("spacing").is_none());
		assert!(value["payload"].get("wrap").is_none());

		let decoded: Item = serde_json::from_value(value).unwrap();
		assert_eq!(decoded, item);
	}
}
