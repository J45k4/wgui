#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlexDirection {
    Column,
    Row
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
	Undefined
}

impl Default for Value {
	fn default() -> Self {
		Value::Undefined
	}
}

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Layout {
	pub body: Vec<Item>,
	pub flex: FlexDirection,
	pub spacing: u32,
	pub wrap: bool
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ItemPayload {
	Layout(Layout),
	Text {
		value: String,
	},
	TextInput {
		value: String,
		placeholder: String
	},
	Textarea {
		value: String,
		placeholder: String
	},
	Select {
		value: String,
		options: Vec<SelectOption>,
	},
	Checkbox {
		checked: bool
	},
	Slider {
		min: i32,
		max: i32,
		value: i32,
		step: i32
	},
	Button {
		title: String
	},
	Table {
		items: Vec<Item>,
	},
	Tbody {
		items: Vec<Item>
	},
	Thead {
		items: Vec<Item>
	},
	Tr {
		items: Vec<Item>
	},
	Th {
		item: Box<Item>
	},
	Td {
		item: Box<Item>
	},
	None
}

impl Default for ItemPayload {
	fn default() -> Self {
		ItemPayload::None
	}
}

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Item {
	pub id: u32,
	pub inx: u32,
	pub payload: ItemPayload,
	pub border: String,
	pub background_color: String,
	pub cursor: String,
	pub height: u32,
	pub width: u32,
	pub max_height: u32,
	pub max_width: u32,
	pub grow: u32,
	pub text_align: String,
	pub margin: u16,
	pub margin_left: u16,
	pub margin_right: u16,
	pub margin_top: u16,
	pub margin_bottom: u16,
	pub padding: u16,
	pub padding_left: u16,
	pub padding_right: u16,
	pub padding_top: u16,
	pub padding_bottom: u16,
	pub overflow: String,
	pub editable: bool,
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
		payload: ItemPayload::Layout(
			Layout {
				body: body.into_iter().collect(),
				flex: FlexDirection::Column,
				..Default::default()
			}
		),
        ..Default::default()
    }
}

pub fn hstack<I>(body: I) -> Item
where
    I: IntoIterator<Item = Item>,
{
	Item {
		payload: ItemPayload::Layout(
			Layout {
				body: body.into_iter().collect(),
				flex: FlexDirection::Row,
				..Default::default()
			}
		),
		..Default::default()
	}
}


pub fn button(title: &str) -> Item {
	Item {
		payload: ItemPayload::Button { 
			title: title.to_string(), 
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
			placeholder: "".to_string()
		},
		..Default::default()
	}
}

pub fn textarea() -> Item {
	Item {
		payload: ItemPayload::Textarea {
			value: "".to_string(),
			placeholder: "".to_string()
		},
		..Default::default()
	}
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelectOption {
	value: String,
	name: String
}

pub fn option(value: &str, name: &str) -> SelectOption {
	SelectOption {
		value: value.to_string(),
		name: name.to_string()
	}
}

pub fn select<I>(options: I) -> Item
where
    I: IntoIterator<Item = SelectOption>,
{
	Item {
		payload: ItemPayload::Select {
			value: "".to_string(),
			options: options.into_iter().collect() 
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
			step: 1
		},
		..Default::default()
	}
}

pub fn table<T>(body: T) -> Item
where
	T: IntoIterator<Item = Item>
{
	Item {
		payload: ItemPayload::Table {
			items: body.into_iter().collect()
		},
		..Default::default()
	}
}

pub fn thead<T>(items: T) -> Item
where
	T: IntoIterator<Item = Item>
{
	Item {
		payload: ItemPayload::Thead {
			items: items.into_iter().collect()
		},
		..Default::default()
	}
}

pub fn tbody<T>(items: T) -> Item
where
	T: IntoIterator<Item = Item>
{
	Item {
		payload: ItemPayload::Tbody {
			items: items.into_iter().collect()
		},
		..Default::default()
	}
}

pub fn tr<T>(items: T) -> Item
where
	T: IntoIterator<Item = Item>
{
	Item {
		payload: ItemPayload::Tr {
			items: items.into_iter().collect()
		},
		..Default::default()
	}
}

pub fn th(item: Item) -> Item {
	Item {
		payload: ItemPayload::Th {
			item: Box::new(item)
		},
		..Default::default()
	}
}

pub fn td(items: Item) -> Item {
	Item {
		payload: ItemPayload::Td {
			item: Box::new(items)
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
	
	pub fn checked(mut self, checked: bool) -> Self {
		self.payload = ItemPayload::Checkbox { checked };
		self
	}

	pub fn min(mut self, m: i32) -> Self {
		match self.payload {
			ItemPayload::Slider { ref mut min, .. } => {
				*min = m;
			},
			_ => {}
		}
		self
	}

	pub fn max(mut self, m: i32) -> Self {
		match self.payload {
			ItemPayload::Slider { ref mut max, .. } => {
				*max = m;
			},
			_ => {}
		}
		self
	}

	pub fn ivalue(mut self, v: i32) -> Self {
		match self.payload {
			ItemPayload::Slider { ref mut value, .. } => {
				*value = v;
			},
			_ => {}
		}
		self
	}

	pub fn svalue(mut self, v: &str) -> Self {
		match self.payload {
			ItemPayload::Text { ref mut value, .. } => {
				*value = v.to_string();
			},
			ItemPayload::TextInput { ref mut value, .. } => {
				*value = v.to_string();
			},
			ItemPayload::Textarea { ref mut value, .. } => {
				*value = v.to_string();
			},
			ItemPayload::Select { ref mut value, .. } => {
				*value = v.to_string();
			},
			_ => {}
		}
		self
	}

	pub fn step(mut self, s: i32) -> Self {
		match self.payload {
			ItemPayload::Slider { ref mut step, .. } => {
				*step = s;
			},
			_ => {}
		}
		self
	}

	pub fn spacing(mut self, spacing: u32) -> Self {
		match self.payload {
			ItemPayload::Layout(ref mut layout) => {
				layout.spacing = spacing;
			},
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
			ItemPayload::TextInput { ref mut placeholder, .. } => {
				*placeholder = p.to_string();
			},
			ItemPayload::Textarea { ref mut placeholder, .. } => {
				*placeholder = p.to_string();
			},
			_ => {}
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

	pub fn width(mut self, w: u32) -> Self {
		self.width = w;
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

	pub fn cursor(mut self, c: &str) -> Self {
		self.cursor = c.to_string();
		self
	}

	pub fn height(mut self, h: u32) -> Self {
		self.height = h;
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
			},
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
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn test_vstack() {
		let view = vstack([
			hstack([
				text("Hello"),
				button("Click me")
			]),
			button("DUNNO")
		]);
	}
}