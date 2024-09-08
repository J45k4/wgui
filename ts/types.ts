import { Deboncer } from "./debouncer.ts";
import { MessageSender } from "./message_sender.ts";

export type Text = {
    type: "text"
    value: string
	placeholder?: string
}

export enum FlexDirection {
    Row = "row",
    Column = "column"
}

export type Flex = {
    grow: number
    flexDirection: FlexDirection
}

export type Layout = {
    type: "layout"
    flex?: FlexDirection
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
    title: string
}

export type TextInput = {
    type: "textInput"
    placeholder: string
    value: string
}

export type Checkbox = {
    type: "checkbox"
    checked: boolean
}

export type Title = {
	type: "title"
	title: string
}

export type Slider = {
	type: "slider"
	min: number
	max: number
	value: number
	step: number
}

export type Select = {
	type: "select"
	value: string
	options: {
		value: string
		name: string
	}[]
}

export type Table = {
	type: "table"
	items: Item[]
}

export type Thead = {
	type: "thead"
	items: Item[]
}

export type Tbody = {
	type: "tbody"
	items: Item[]
}

export type Tr = {
	type: "tr"
	items: Item[]
}

export type Th = {
	type: "th"
	item: Item
}

export type Td = {
	type: "td"
	item: Item
}

export type None = {
	type: "none"
}

export type ItemPayload = Text |
	TextInput | 
	Table |
	Thead |
	Tbody |
	Tr |
	Th | 
	Td |
	Select | 
	Checkbox | 
	Slider |
	Layout |
	Button |
	None 

export type Item = {
	id: number
	inx?: number
	typ: number
	height: number
	width: number
	maxHeight: number
	maxWidth: number
	grow?: number
	backgroundColor?: string
	textAlign?: string
	cursor?: string
	margin?: number
	padding?: number
	border?: string
	marginLeft?: number
	marginRight?: number
	marginTop?: number
	marginBottom?: number
	paddingLeft?: number
	paddingRight?: number
	paddingTop?: number
	paddingBottom?: number
	editable?: boolean
	overflow?: string
	payload: ItemPayload
}

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
	sets: {
		prop: string
		value: string
	}[]
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
	SetProp

export type OnClick = {
    type: "onClick"
    id: number
	inx?: number
}

export type OnTextChange = {
    type: "onTextChanged"
    id: number
	inx?: number
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
	id: number
	inx?: number
	value: number
}

export type OnSelect = {
	type: "onSelect"
	id: number
	inx?: number
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