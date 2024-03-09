
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

#[derive(Debug, PartialEq, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub flex: Option<Flex>,
    pub height: Option<u32>,
    pub width: Option<u32>,
    pub body: Vec<Item>,
    pub margin_top: Option<u32>,
    pub margin_right: Option<u32>,
    pub margin_bottom: Option<u32>,
    pub margin_left: Option<u32>,
    pub margin: Option<u32>,
    pub padding_top: Option<u32>,
    pub padding_right: Option<u32>,
    pub padding_bottom: Option<u32>,
    pub padding_left: Option<u32>,
    pub padding: Option<u32>,
}

#[derive(Debug, PartialEq, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Button {
    pub id: Option<String>,
    pub name: Option<String>,
    pub title: String,
    pub flex: Option<Flex>
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Text {
    pub text: String,
}

#[derive(Debug, PartialEq, Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct TextInput {
    pub id: String,
    pub name: String,
    pub placeholder: String,
    pub value: String,
    pub flex: Option<Flex>,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Checkbox {
    pub id: String,
    pub name: String,
    pub checked: bool
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Video {
    id: String,
    name: String,
    src: String,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum Item {
    View(View),
    Text(Text),
    Button(Button),
    TextInput(TextInput),
    Checkbox(Checkbox)
}