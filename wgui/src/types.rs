use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;

use crate::gui::Item;

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub struct OnClick {
    pub id: Option<String>,
    pub name: Option<String>
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct OnKeyDown {
    pub id: Option<String>,
    pub keycode: String
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct OnTextChanged {
    pub id: Option<String>,
    pub name: Option<String>,
    pub value: String,
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct PathChanged {
    pub path: String,
    pub query: HashMap<String, String>
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct InputQuery {

}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ClientEvent { 
    Disconnected { id: usize},
    Connected { id: usize },
    PathChanged(PathChanged),
    Input(InputQuery),
    OnClick(OnClick),
    OnTextChanged(OnTextChanged),
}

pub type ItemPath = Vec<usize>;

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct Replace {
    pub path: ItemPath,
    pub item: Item
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct ReplaceAt {
    pub path: ItemPath,
    pub item: Item,
    pub inx: usize
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddBack {
    pub path: ItemPath,
    pub item: Item
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct AddFront {
    pub path: ItemPath,
    pub item: Item
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct InsertAt {
    pub path: ItemPath,
    pub item: Item,
    pub inx: usize
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub struct RemoveInx {
    pub path: ItemPath,
    pub inx: usize
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
    pub query: HashMap<String, String>
}

#[derive(Debug, PartialEq, Clone, serde::Serialize, serde::Deserialize)]
pub enum Value {
	String(String),
	Number(u32)
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
	SetProp {
		path: ItemPath,
		prop: String,
		value: String
	},
	SetStyle {
		path: ItemPath,
		prop: String,
		value: String
	}
}

pub enum ServerEvent {
    Connected { ch: mpsc::UnboundedSender<ClientEvent> },
    ClientEvent { id: usize, event: ClientEvent }
}

#[derive(Debug, Clone)]
pub enum Command {
    Render(Item),
}

pub type Clients = Arc<RwLock<HashMap<usize, mpsc::UnboundedSender<Command>>>>;