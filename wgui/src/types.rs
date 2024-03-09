use std::collections::HashMap;

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
    Disconnected,
    Connected { id: usize },
    PathChanged(PathChanged),
    Input(InputQuery)
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
}