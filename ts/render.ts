import { Context, Item, ItemPayload } from "./types.ts";



const renderChildren = (element: HTMLElement, items: Item[], ctx: Context) => {
	for (const item of items) {
		const child = renderItem(item, ctx)
		if (child) {
			element.appendChild(child)
		}
	}
}

const renderPayload = (item: Item, ctx: Context, old?: Element) => {
	const payload = item.payload
	if (payload.type === "checkbox") {
		let checkbox: HTMLInputElement
		if (old instanceof HTMLInputElement) {
			checkbox = old
		} else {
			checkbox = document.createElement("input")
		}
		checkbox.type = "checkbox"
		checkbox.checked = payload.checked
		checkbox.onclick = () => {
			ctx.sender.send({
				type: "onClick",
				id: item.id,
				inx: item.inx,
			})
			ctx.sender.sendNow()
		}
		return checkbox
	}

	if (payload.type === "layout") {
		let element: HTMLDivElement
		if (old instanceof HTMLDivElement) {
			element = old
			old.innerHTML = ""
			for (const i of payload.body) {
				const el = renderItem(i, ctx)
				if (el) {
					old.appendChild(el as any)
				}
			}
		} else {
			const div = document.createElement("div")
			for (const i of payload.body) {
				const el = renderItem(i, ctx)
				if (el) {
					div.appendChild(el as any)
				}
			}
			element = div
		}

		if (payload.spacing) {
			element.style.gap = payload.spacing + "px"
		}
		if (payload.wrap) {
			element.style.flexWrap = "wrap"
		}
		if (payload.flex) {
			element.style.display = "flex"
			element.style.flexDirection = payload.flex
		}
		return element
	}

	if (payload.type === "select") {
		let select: HTMLSelectElement
		if (old instanceof HTMLSelectElement) {
			select = old
			const existingOptions = Array.from(old.options)
			const newOptions = payload.options.map(option => option.value)
	
			// Update the options only if they differ
			if (existingOptions.length !== payload.options.length ||
				!existingOptions.every((opt, index) => opt.value === newOptions[index])) {
	
				old.innerHTML = ""
				for (const option of payload.options) {
					const opt = document.createElement("option")
					opt.value = option.value
					opt.text = option.name
					old.add(opt)
				}
			}
		} else {
			select = document.createElement("select")
	
			for (const option of payload.options) {
				const opt = document.createElement("option")
				opt.value = option.value
				opt.text = option.name
				select.add(opt)
			}
	
			// Set the value of the new select element
			select.value = payload.value
	
			select.onchange = () => {
				ctx.sender.send({
					type: "onSelect",
					id: item.id,
					inx: item.inx,
					value: select.value,
				})
	
				ctx.sender.sendNow()
			}
		}

		return select
	}

	if (payload.type === "button") {
		let button: HTMLButtonElement
		if (old instanceof HTMLButtonElement) {
			button = old
		} else {
			button = document.createElement("button")
		}
		button.textContent = payload.title
		button.onclick = () => {
			ctx.sender.send({
				type: "onClick",
				id: item.id,
				inx: item.inx,
			})
			ctx.sender.sendNow()
		}
		return button
	}

	if (payload.type === "slider") {
		let slider: HTMLInputElement
		if (old instanceof HTMLInputElement) {
			slider = old
		} else {
			slider = document.createElement("input")
		}
		slider.min = payload.min.toString()
		slider.max = payload.max.toString()
		slider.type = "range"
		slider.value = payload.value.toString()
		slider.step = payload.step.toString()
		slider.oninput = (e: any) => {
			ctx.sender.send({
				type: "onSliderChange",
				id: item.id,
				inx: item.inx,
				value: parseInt(e.target.value)
			})
		}
		return slider
	}

	if (payload.type === "textInput") {
		let input: HTMLInputElement
		if (old instanceof HTMLInputElement) {
			input = old
		} else {
			input = document.createElement("input")
		}
		input.placeholder = payload.placeholder as string
		input.value = payload.value
		input.oninput = (e: any) => {
			ctx.sender.send({
				type: "onTextChanged",
				id: item.id,
				inx: item.inx,
				value: e.target.value,
			})
		}
		return input
	}

	if (payload.type === "table") {
		let table: HTMLTableElement
		if (old instanceof HTMLTableElement) {
			table = old
		} else {
			table = document.createElement("table")
		}
		renderChildren(table, payload.items, ctx)
		return table
	}

	if (payload.type === "thead") {
		let thead: HTMLTableSectionElement
		if (old instanceof HTMLTableSectionElement) {
			thead = old
		} else {
			thead = document.createElement("thead")
		}
		renderChildren(thead, payload.items, ctx)
		return thead
	}

	if (payload.type === "tbody") {
		let tbody: HTMLTableSectionElement
		if (old instanceof HTMLTableSectionElement) {
			tbody = old
		} else {
			tbody = document.createElement("tbody")
		}
		renderChildren(tbody, payload.items, ctx)
		return tbody
	}

	if (payload.type === "tr") {
		let tr: HTMLTableRowElement
		if (old instanceof HTMLTableRowElement) {
			tr = old
		} else {
			tr = document.createElement("tr")
		}
		renderChildren(tr, payload.items, ctx)
		return tr
	}

	if (payload.type === "th") {
		let th: HTMLTableCellElement
		if (old instanceof HTMLTableCellElement) {
			th = old
		} else {
			th = document.createElement("th")
		}
		renderChildren(th, [payload.item], ctx)
		return th
	}

	if (payload.type === "td") {
		let td: HTMLTableCellElement
		if (old instanceof HTMLTableCellElement) {
			td = old
		} else {
			td = document.createElement("td")
		}
		renderChildren(td, [payload.item], ctx)
		return td
	}

	if (payload.type === "text") {
		let element: HTMLSpanElement
		if (old instanceof HTMLSpanElement) {
			element = old
			element.innerText = payload.value + ""
		} else {
			element = document.createElement("span")
			element.innerText = payload.value + ""
		}
		if (item.id) {
			element.onclick = () => {
				ctx.sender.send({
					type: "onClick",
					id: item.id,
					inx: item.inx,
				})
				ctx.sender.sendNow()
			}
		}
		return element
	}

	// switch (payload.type) {
	// 	case "text": {
	// 		// TODO
	// 		// let element: Text
	// 		// if (old instanceof HTMLSpanElement) {
	// 		// 	element = old
	// 		// 	old.innerText = payload.value + ""
	// 		// } else {
	// 		// 	element = document.createElement("span")
	// 		// 	element.innerText = payload.value + ""
	// 		// }
	// 		// break
	// 	}
	// 	case "table": {
	// 		let element: HTMLTableElement
	// 		if (old instanceof HTMLTableElement) {
	// 			element = old
	// 			element.innerHTML = ""
	// 		} else {
	// 			element = document.createElement("table")
	// 		}
	// 		renderChildren(element, payload.items, ctx)
	// 		return element
	// 	}
	// 	case "thead": {
	// 		let element: HTMLTableSectionElement
	// 		if (old instanceof HTMLTableSectionElement) {
	// 			element = old
	// 			element.innerHTML = ""
	// 		} else {
	// 			element = document.createElement("thead")
	// 		}
	// 		break	
	// 	}
	// 	case "tbody": {
	// 		if (old instanceof HTMLTableSectionElement) {
	// 			element = old
	// 			element.innerHTML = ""
	// 		} else {
	// 			element = document.createElement("tbody")
	// 		}
	// 		break	
	// 	}
	// 	case "tr": {
	// 		break	
	// 	}
	// 	case "th": {
	// 		break	
	// 	}
	// 	case "td": {
	// 		break	
	// 	}
	// 	case "none": {
	// 		element = document.createElement("div")
	// 		element.innerText = "None"
	// 		break
	// 	}
	// 	default: {
	// 		const _: never = payload
	// 		throw new Error("Unknown payload type: " + payload)
	// 	}
	// }
}

export const renderItem = (item: Item, ctx: Context, old?: Element) => {
	const element = renderPayload(item, ctx, old)

	if (!element) {
		return
	}

	if (item.width) {
		element.style.width = item.width + "px"
	}
	if (item.height) {
		element.style.height = item.height + "px"
	}
	if (item.maxWidth) {
		element.style.maxWidth = item.maxWidth + "px"
	}
	if (item.maxHeight) {
		element.style.maxHeight = item.maxHeight + "px"
	}
	if (item.grow) {
		element.style.flexGrow = item.grow.toString()
	}
	if (item.backgroundColor) {
		element.style.backgroundColor = item.backgroundColor
	}
	if (item.textAlign) {
		element.style.textAlign = item.textAlign
	}
	if (item.cursor) {
		element.style.cursor = item.cursor
	}
	if (item.margin) {
		element.style.margin = item.margin + "px"
	}
	if (item.padding) {
		element.style.padding = item.padding + "px"
	}
	if (item.border) {
		element.style.border = item.border
	}

	return element
}