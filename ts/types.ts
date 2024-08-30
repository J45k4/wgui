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
    flexDirection: FlexDirection
}

export type View = {
    type: "view"
	id?: string
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
	spacing?: number
	border?: string
	backgroundColor?: string
	cursor?: string
	wrap?: boolean
	maxWidth?: number
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

export type H1 = {
    type: "h1"
    text: string
}

export type Title = {
	type: "title"
	title: string
}

export type Slider = {
	type: "slider"
	id: string
	min: number
	max: number
	value: number
	step: number
	width: number
	height: number
}

export type Select = {
	type: "select"
	id: string
	value: string
	width: number
	height: number
	options: {
		value: string
		name: string
	}[]
}

export type Item = View | 
    Text | 
    Button | 
    TextInput | 
    Table | 
    Checkbox |
    H1 |
	Title |
	Slider |
	Select

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

export type SetProp = {
	type: "setProp"
	path: number[]
	prop: string
	value: string
}

export type SetStyle = {
	type: "setStyle"
	path: number[]
	prop: string
	value: string
}

export type SetID = {
	type: "setId"
	id: string
	path: number[]
}

export type SetSpacing = {
	type: "setSpacing"
	path: number[]
	spacing: number
}

export type SrvMessage = Replace |
    ReplaceAt |
    AddBack | 
    AddFront | 
    InsertAt | 
    RemoveInx |
    PushState |
    ReplaceState |
    SetQuery |
	SetProp |
	SetStyle |
	SetID |
	SetSpacing

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

export type OnSliderChange = {
	type: "onSliderChange"
	id: string
	value: number
}

export type OnSelect = {
	type: "onSelect"
	id: string
	value: string
}

export type MessageToSrv = OnClick | 
    OnTextChange | 
    OnKeyDown | 
    PathChanged |
	OnSliderChange |
	OnSelect

export type MessagesToSrv = MessageToSrv[]

export type Context = {
    debouncer: Deboncer
    sender: MessageSender
}