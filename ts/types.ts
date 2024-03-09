import { Deboncer } from "./debouncer.ts";
import { MessageSender } from "./message_sender.ts";

export type Text = {
    type: "text"
    text: string
}

export enum FlexDirection {
    Row = "row",
    Column = "column"
}

export type Flex = {
    grow: number
    direction: FlexDirection
}

export type View = {
    type: "view"
    flex?: Flex
    height: number
    width: number
    marginTop?: number
    marginRight?: number
    marginBottom?: number
    marginLeft?: number
    margin?: number 
    paddingTop?: number
    paddingRight?: number
    paddingBottom?: number
    paddingLeft?: number
    padding?: number
    body: Item[]
}

export type Button = {
    type: "button"
    id: string
    name: string
    title: string
    flex?: Flex
}

export type TextInput = {
    type: "textInput"
    id: string
    name: string
    placeholder: string
    value: string
    flex?: Flex
}

export type Table = {
    type: "table"
    headers: string[]
    rows: Item[][]
}

export type Checkbox = {
    type: "checkbox"
    id: string
    name: string
    checked: boolean
}

export type Item = View | 
    Text | 
    Button | 
    TextInput | 
    Table | 
    Checkbox

export type Replace = {
    type: "replace"
    path: number[]
    item: Item
}

export type ReplaceAt = {
    type: "replaceAt"
    inx: number
    path: number[]
    item: Item
}

export type AddBack = {
    type: "addBack"
    path: number[]
    item: Item
}

export type AddFront = {
    type: "addFront"
    path: number[]
    item: Item
}

export type InsertAt = {
    type: "insertAt"
    inx: number
    item: Item
    path: number[]
}

export type RemoveInx = {
    type: "removeInx"
    inx: number
    path: number[]
}

export type PushState = {
    type: "pushState"
    url: string
}

export type ReplaceState = {
    type: "replaceState"
    url: string
}

export type SetQuery = {
    type: "setQuery"
    query: {
        [key: string]: string
    }
}

export type SrvMessage = Replace |
    ReplaceAt |
    AddBack | 
    AddFront | 
    InsertAt | 
    RemoveInx |
    PushState |
    ReplaceState |
    SetQuery


export type OnClick = {
    type: "onClick"
    id: string
    name: string
}

export type OnTextChange = {
    type: "onTextChanged"
    id: string
    name: string
    value: string
}

export type OnKeyDown = {
    type: "onKeyDown"
    id: string
    name: string
    keycode: string
}

export type PathChanged = {
    type: "pathChanged"
    path: string
    query: {
        [key: string]: string
    }
}

export type MessageToSrv = OnClick | 
    OnTextChange | 
    OnKeyDown | 
    PathChanged

export type MessagesToSrv = MessageToSrv[]

export type Context = {
    debouncer: Deboncer
    sender: MessageSender
}