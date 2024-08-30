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

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Flex {
    #[serde(rename = "flexDirection")]
    pub direction: FlexDirection,
    pub grow: Option<u32>,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub enum Margin {
	All(f32),
	Individual {
		top: f32,
		right: f32,
		bottom: f32,
		left: f32
	},
	None
}

impl Default for Margin {
	fn default() -> Self {
		Margin::None
	}
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub enum Padding {
	All(u32),
	Individual {
		top: u32,
		right: u32,
		bottom: u32,
		left: u32
	},
	None
}

impl Default for Padding {
	fn default() -> Self {
		Padding::None
	}
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct HStack {
	pub body: Vec<Item>,
	pub margin: Margin,
	pub padding: Padding,
	pub spacing: f32
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct VStack {
	pub body: Vec<Item>,
	pub margin: Margin,
	pub padding: Padding,
	pub spacing: f32
}

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
	pub id: Option<String>,
    pub flex: Option<Flex>,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub body: Vec<Item>,
	pub margin: Option<u32>,
	pub padding: Option<u32>,
	pub spacing: Option<u32>,
	pub border: Option<String>,
	pub wrap: Option<bool>,
	pub background_color: Option<String>,
	pub cursor: Option<String>,
	pub max_width: Option<u32>
}


impl View {
	pub fn id(mut self, id: &str) -> Self {
		self.id = Some(id.to_string());
		self
	}

	pub fn add<T: Into<Item>>(mut self, item: T) -> Self {
		self.body.push(item.into());
		self
	}

	pub fn add_many<T: Into<Item>>(mut self, items: Vec<T>) -> Self {
		self.body.extend(items.into_iter().map(|item| item.into()));
		self
	}

	pub fn spacing(mut self, spacing: u32) -> Self {
		self.spacing = Some(spacing);
		self
	}

	pub fn border(mut self, border: &str) -> Self {
		self.border = Some(border.to_string());
		self
	}
	
	pub fn padding(mut self, padding: u32) -> Self {
		self.padding = Some(padding);
		self
	}

	pub fn margin(mut self, margin: u32) -> Self {
		self.margin = Some(margin);
		self
	}

	pub fn wrap(mut self, wrap: bool) -> Self {
		self.wrap = Some(wrap);
		self
	}

	pub fn background_color(mut self, color: &str) -> Self {
		self.background_color = Some(color.to_string());
		self
	}

	pub fn cursor(mut self, cursor: &str) -> Self {
		self.cursor = Some(cursor.to_string());
		self
	}

	pub fn max_width(mut self, max_width: u32) -> Self {
		self.max_width = Some(max_width);
		self
	}

	pub fn width(mut self, width: u32) -> Self {
		self.width = Some(width);
		self
	}

	pub fn height(mut self, height: u32) -> Self {
		self.height = Some(height);
		self
	}
}

#[derive(Debug, PartialEq, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Button {
    pub id: Option<String>,
    pub name: Option<String>,
    pub title: String,
    pub flex: Option<Flex>
}

impl Button {
	pub fn id(mut self, id: &str) -> Self {
		self.id = Some(id.to_string());
		self
	}

	pub fn name(mut self, name: &str) -> Self {
		self.name = Some(name.to_string());
		self
	}

	pub fn title(mut self, title: &str) -> Self {
		self.title = title.to_string();
		self
	}

	pub fn flex(mut self, flex: Flex) -> Self {
		self.flex = Some(flex);
		self
	
	}
}

// impl Into<Item> for Button {
// 	fn into(self) -> Item {
// 		Item::Button(self)
// 	}
// }

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Text {
    pub text: String,
}

// impl Into<Item> for Text {
// 	fn into(self) -> Item {
// 		Item::Text(self)
// 	}
// }

#[derive(Debug, PartialEq, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextInput {
    pub id: String,
    pub name: String,
    pub placeholder: String,
    pub value: String,
    pub flex: Option<Flex>,
}

impl TextInput {
	pub fn id(mut self, id: &str) -> Self {
		self.id = id.to_string();
		self
	}

	pub fn name(mut self, name: &str) -> Self {
		self.name = name.to_string();
		self
	}

	pub fn placeholder(mut self, placeholder: &str) -> Self {
		self.placeholder = placeholder.to_string();
		self
	}

	pub fn value(mut self, value: &str) -> Self {
		self.value = value.to_string();
		self
	}

	pub fn flex(mut self, flex: Flex) -> Self {
		self.flex = Some(flex);
		self
	}

}

#[derive(Debug, Default, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Checkbox {
    pub id: String,
    pub name: String,
    pub checked: bool
}

impl Checkbox {
	pub fn new(id: &str, name: &str, checked: bool) -> Checkbox {
		Checkbox {
			id: id.to_string(),
			name: name.to_string(),
			checked
		}
	}

	pub fn id(mut self, id: &str) -> Self {
		self.id = id.to_string();
		self
	}

	pub fn checked(mut self, checked: bool) -> Self {
		self.checked = checked;
		self
	}
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Video {
    id: String,
    name: String,
    src: String,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct H1 {
    pub text: String
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct SelectOption {
	value: String,
	name: String
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Select {
	id: String,
	value: Option<String>,
	options: Vec<SelectOption>,
	width: Option<u32>,
	height: Option<u32>
}

impl Select {
	pub fn id(mut self, id: &str) -> Self {
		self.id = id.to_string();
		self
	}

	pub fn value(mut self, value: &str) -> Self {
		self.value = Some(value.to_string());
		self
	}

	pub fn width(mut self, width: u32) -> Self {
		self.width = Some(width);
		self
	}

	pub fn height(mut self, height: u32) -> Self {
		self.height = Some(height);
		self
	}

	pub fn add_option(mut self, value: &str, name: &str) -> Self {
		self.options.push(SelectOption {
			value: value.to_string(),
			name: name.to_string()
		});
		self
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

pub const ITEM_CHECKBOX: u8 = 1;
pub const ITEM_VSTACK: u8 = 2;
pub const ITEM_HSTACK: u8 = 3;
pub const ITEM_BUTTON: u8 = 4;
pub const ITEM_TEXT: u8 = 5;
pub const ITEM_TEXT_INPUT: u8 = 6;

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Layout {
	pub body: Vec<Item>,
	pub flex: FlexDirection,
	pub height: u32,
	pub width: u32,
	pub padding: u32,
	pub spacing: u32,
	pub wrap: bool,
	pub max_width: u32
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub enum ItemPayload {
	Layout(Layout),
	Text {
		value: String,
		placeholder: String
	},
	Select {
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
	None
}

impl Default for ItemPayload {
	fn default() -> Self {
		ItemPayload::None
	}
}

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub struct Item {
	pub id: u32,
	pub typ: u8,
	pub payload: ItemPayload,
	pub border: String,
	pub background_color: String,
	pub cursor: String,
}

pub fn checkbox() -> Item {
	Item {
		typ: ITEM_CHECKBOX,
		..Default::default()
	}
}

pub fn vstack<I>(body: I) -> Item
where
    I: IntoIterator<Item = Item>,
{
    Item {
		typ: ITEM_VSTACK,
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
		typ: ITEM_HSTACK,
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
		typ: ITEM_BUTTON,
		payload: ItemPayload::Text { 
			value: title.to_string(), 
			placeholder: "".to_string()
		},
		..Default::default()
	}
}

pub fn text(text: &str) -> Item {
	Item {
		typ: ITEM_TEXT,
		payload: ItemPayload::Text {
			value: text.to_string(),
			placeholder: "".to_string()
		},
		..Default::default()
	}
}

pub fn text_input() -> Item {
	Item {
		typ: ITEM_TEXT_INPUT,
		..Default::default()
	}
}

pub fn select() -> Item {
	Item {
		..Default::default()
	}
}

pub fn slider() -> Item {
	Item {
		..Default::default()
	}
}

impl Item {
	pub fn id(mut self, id: u32) -> Self {
		self.id = id;
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

	pub fn value(mut self, v: i32) -> Self {
		match self.payload {
			ItemPayload::Slider { ref mut value, .. } => {
				*value = v;
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