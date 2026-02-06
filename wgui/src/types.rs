use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

use crate::gui::{Item, ThreeKind, ThreeProp, ThreePropValue};

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub struct OnClick {
	pub id: u32,
	pub inx: Option<u32>,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct OnKeyDown {
	pub id: Option<String>,
	pub keycode: String,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct OnTextChanged {
	pub id: u32,
	pub inx: Option<u32>,
	pub value: String,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PathChanged {
	pub path: String,
	pub query: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputQuery {}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct OnSliderChange {
	pub id: u32,
	pub inx: Option<u32>,
	pub value: i32,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct OnSelect {
	pub id: u32,
	pub inx: Option<u32>,
	pub value: String,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ClientEvent {
	Disconnected { id: usize },
	Connected { id: usize },
	PathChanged(PathChanged),
	Input(InputQuery),
	OnClick(OnClick),
	OnTextChanged(OnTextChanged),
	OnSliderChange(OnSliderChange),
	OnSelect(OnSelect),
}

#[derive(Debug, Clone)]
pub struct ClientMessage {
	pub client_id: usize,
	pub event: ClientEvent,
}

pub type ItemPath = Vec<usize>;

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Replace {
	pub path: ItemPath,
	pub item: Item,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplaceAt {
	pub path: ItemPath,
	pub item: Item,
	pub inx: usize,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddBack {
	pub path: ItemPath,
	pub item: Item,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddFront {
	pub path: ItemPath,
	pub item: Item,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct InsertAt {
	pub path: ItemPath,
	pub item: Item,
	pub inx: usize,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoveInx {
	pub path: ItemPath,
	pub inx: usize,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PushState {
	pub url: String,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplaceState {
	pub url: String,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct SetQuery {
	pub query: HashMap<String, String>,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub enum Value {
	String(String),
	Number(u32),
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub enum PropKey {
	ID = 1,
	Border = 2,
	BackgroundColor = 3,
	Spacing = 4,
	FlexDirection = 5,
	Grow = 6,
	Width = 7,
	Height = 8,
	MinWidth = 9,
	MaxWidth = 10,
	MinHeight = 11,
	MaxHeight = 12,
	Padding = 13,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct SetProp {
	pub key: PropKey,
	pub value: Value,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum ThreeOp {
	Create {
		id: u32,
		kind: ThreeKind,
		props: Vec<ThreeProp>,
	},
	Attach {
		#[serde(rename = "parentId")]
		parent_id: u32,
		#[serde(rename = "childId")]
		child_id: u32,
	},
	Detach {
		#[serde(rename = "parentId")]
		parent_id: u32,
		#[serde(rename = "childId")]
		child_id: u32,
	},
	SetProp {
		id: u32,
		key: String,
		value: ThreePropValue,
	},
	UnsetProp {
		id: u32,
		key: String,
	},
	Delete {
		id: u32,
	},
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ClientAction {
	Replace(Replace),
	ReplaceAt(ReplaceAt),
	AddBack(AddBack),
	AddFront(AddFront),
	InsertAt(InsertAt),
	RemoveInx(RemoveInx),
	PushState(PushState),
	ReplaceState(ReplaceState),
	SetQuery(SetQuery),
	SetProp { path: ItemPath, sets: Vec<SetProp> },
	ThreePatch { path: ItemPath, ops: Vec<ThreeOp> },
	SetTitle { title: String },
}

pub enum ServerEvent {
	Connected {
		ch: mpsc::UnboundedSender<ClientMessage>,
	},
	ClientEvent {
		id: usize,
		event: ClientEvent,
	},
}

#[derive(Debug, Clone)]
pub enum Command {
	Render(Item),
	SetTitle(String),
}

pub type Clients = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Command>>>>;
