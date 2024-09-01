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
	pub padding: u32,
	pub spacing: u32,
	pub wrap: bool,
	pub max_width: u32
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

// #[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
// pub struct Tr {
// 	items: Vec<Td>
// }

// pub fn tr<T>(items: T) -> Tr 
// where
// 	T: IntoIterator<Item = Td>
// {
// 	Tr {
// 		items: items.into_iter().collect()
// 	}
// }

// pub fn th(text: &str) -> Item {
// 	Item {
// 		..Default::default()
// 	}
// }

// #[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
// pub struct Th {
// 	text: String
// }

// #[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
// pub struct Td {

// }

// pub fn td(items: Item) -> Td {
// 	Td { }
// }

// pub fn table<T, B>(head: T, body: B) -> Item 
// where	
// 	T: IntoIterator<Item = Th>,
// 	B: IntoIterator<Item = Tr>
// {
// 	Item {
// 		payload: ItemPayload::Table { 
// 			head: head.into_iter().collect(), 
// 			body: body.into_iter().collect()
// 		},
// 		..Default::default()
// 	}
// }

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

	pub fn padding(mut self, padding: u32) -> Self {
		match self.payload {
			ItemPayload::Layout(ref mut layout) => {
				layout.padding = padding;
			},
			_ => {}
		}
		self
	}

	pub fn margin(mut self, margin: u32) -> Self {
		match self.payload {
			ItemPayload::Layout(ref mut layout) => {
				layout.padding = margin;
			},
			_ => {}
		}
		self
	}

	pub fn placeholder(mut self, p: &str) -> Self {
		match self.payload {
			ItemPayload::TextInput { ref mut placeholder, .. } => {
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