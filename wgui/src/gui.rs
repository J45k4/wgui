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

pub fn margin(margin: f32) {

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

impl Into<Item> for View {
	fn into(self) -> Item {
		Item::View(self)
	}
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

pub fn view() -> View {
	View {
		..Default::default()
	}
}

pub fn vstack(body: Vec<Item>) -> View {
	View {
		body,
		flex: Some(Flex {
			direction: FlexDirection::Column,
			grow: None
		}),
		..Default::default()
	}
}

pub fn hstack(body: Vec<Item>) -> View {
	View {
		body,
		flex: Some(Flex {
			direction: FlexDirection::Row,
			grow: None
		}),
		..Default::default()
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

impl Into<Item> for Button {
	fn into(self) -> Item {
		Item::Button(self)
	}
}

pub fn button(title: &str) -> Button {
	Button {
		id: None,
		name: None,
		title: title.to_string(),
		flex: None
	}
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Text {
    pub text: String,
}

impl Into<Item> for Text {
	fn into(self) -> Item {
		Item::Text(self)
	}
}

pub fn text(text: &str) -> Item {
	Item::Text(Text {
		text: text.to_string()
	})
}

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

impl Into<Item> for TextInput {
	fn into(self) -> Item {
		Item::TextInput(self)
	}
}

pub fn text_input() -> TextInput {
	TextInput {
		id: "".to_string(),
		name: "".to_string(),
		placeholder: "".to_string(),
		value: "".to_string(),
		flex: None
	}
}

#[derive(Debug, Default, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Checkbox {
    pub id: String,
    pub name: String,
    pub checked: bool
}

impl Into<Item> for Checkbox {
	fn into(self) -> Item {
		Item::Checkbox(self)
	}
}

pub fn checkbox() -> Checkbox {
	Checkbox {
		id: "".to_string(),
		name: "".to_string(),
		checked: false
	}
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

pub fn h1(text: &str) -> Item {
	Item::H1(H1 {
		text: text.to_string()
	})
}

pub fn title(text: &str) -> Item {
	Item::Title {
		title: text.to_string()
	}
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Item {
    H1(H1),
    View(View),
    Text(Text),
    Button(Button),
    TextInput(TextInput),
    Checkbox(Checkbox),
	Title { title: String }
}