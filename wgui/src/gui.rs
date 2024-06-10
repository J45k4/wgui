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
	All(f32),
	Individual {
		top: f32,
		right: f32,
		bottom: f32,
		left: f32
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
    pub flex: Option<Flex>,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub body: Vec<Item>,
	pub margin: Margin,
	pub padding: Padding,
}

impl Into<Item> for View {
	fn into(self) -> Item {
		Item::View(self)
	}
}

pub fn view() -> View {
	View {
		flex: None,
		height: None,
		width: None,
		body: vec![],
		margin: Margin::None,
		padding: Padding::None
	}
}

#[derive(Debug, PartialEq, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Button {
    pub id: Option<String>,
    pub name: Option<String>,
    pub title: String,
    pub flex: Option<Flex>
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

pub fn text(text: &str) -> Text {
	Text {
		text: text.to_string()
	}
}

#[derive(Debug, PartialEq, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextInput {
    pub id: String,
    pub name: String,
    pub placeholder: String,
    pub value: String,
    pub flex: Option<Flex>,
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

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Item {
    H1(H1),
    View(View),
    Text(Text),
    Button(Button),
    TextInput(TextInput),
    Checkbox(Checkbox)
}