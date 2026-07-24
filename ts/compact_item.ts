import { FlexDirection, Item, ItemPayload, SrvMessage } from "./types.ts"

// Rust omits Item and Layout fields whose value is their normal default from
// both websocket actions and SSR hydration. Restore that complete shape here
// so the renderer can continue to treat every Item identically.
export const normalizeItem = (compact: Item): Item => {
	const raw = compact as Partial<Item>
	return {
		id: raw.id ?? 0,
		inx: raw.inx ?? 0,
		typ: raw.typ ?? 0,
		height: raw.height ?? 0,
		width: raw.width ?? 0,
		minHeight: raw.minHeight ?? 0,
		maxHeight: raw.maxHeight ?? 0,
		minWidth: raw.minWidth ?? 0,
		maxWidth: raw.maxWidth ?? 0,
		grow: raw.grow ?? 0,
		backgroundColor: raw.backgroundColor ?? "",
		color: raw.color ?? "",
		breakWords: raw.breakWords ?? false,
		fill: raw.fill ?? false,
		textAlign: raw.textAlign ?? "",
		whiteSpace: raw.whiteSpace ?? "",
		cursor: raw.cursor ?? "",
		margin: raw.margin ?? 0,
		padding: raw.padding ?? 0,
		border: raw.border ?? "",
		marginLeft: raw.marginLeft ?? 0,
		marginRight: raw.marginRight ?? 0,
		marginTop: raw.marginTop ?? 0,
		marginBottom: raw.marginBottom ?? 0,
		paddingLeft: raw.paddingLeft ?? 0,
		paddingRight: raw.paddingRight ?? 0,
		paddingTop: raw.paddingTop ?? 0,
		paddingBottom: raw.paddingBottom ?? 0,
		editable: raw.editable ?? false,
		overflow: raw.overflow ?? "",
		name: raw.name ?? "",
		action: raw.action ?? "",
		method: raw.method ?? "",
		partialAddr: raw.partialAddr ?? "",
		...(raw.formArg === undefined ? {} : { formArg: raw.formArg }),
		payload: normalizePayload(raw.payload ?? { type: "none" }),
	}
}

const normalizeItems = (items: Item[] | undefined): Item[] =>
	(items ?? []).map(normalizeItem)

const normalizePayload = (payload: ItemPayload): ItemPayload => {
	if (payload.type === "layout") {
		return {
			...payload,
			body: normalizeItems(payload.body),
			flex: payload.flex ?? FlexDirection.Column,
			spacing: payload.spacing ?? 0,
			wrap: payload.wrap ?? false,
			horizontalResize: payload.horizontalResize ?? false,
			horizontal_resize: payload.horizontal_resize ?? false,
			vresize: payload.vresize ?? false,
			hresize: payload.hresize ?? false,
		}
	}
	if (payload.type === "form") {
		return { ...payload, body: normalizeItems(payload.body) }
	}
	if (payload.type === "table" || payload.type === "thead" || payload.type === "tbody" || payload.type === "tr") {
		return { ...payload, items: normalizeItems(payload.items) }
	}
	if (payload.type === "th" || payload.type === "td") {
		return { ...payload, item: normalizeItem(payload.item) }
	}
	if (payload.type === "modal" || payload.type === "connectionStatus") {
		return { ...payload, body: normalizeItems(payload.body) }
	}
	return payload
}

export const normalizeServerMessage = (message: SrvMessage): SrvMessage => {
	switch (message.type) {
		case "replace":
		case "replaceAt":
		case "addBack":
		case "addFront":
		case "insertAt":
			return { ...message, item: normalizeItem(message.item) }
		default:
			return message
	}
}
