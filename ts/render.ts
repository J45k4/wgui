import { Context, Item, ItemPayload } from "./types.ts";



const renderChildren = (element: HTMLElement, items: Item[], ctx: Context) => {
	for (const item of items) {
		const child = renderItem(item, ctx)
		if (child) {
			element.appendChild(child)
		}
	}
}

const renderPayload = (item: Item, ctx: Context, old?: Element | null) => {
	const payload = item.payload
	if (payload.type === "checkbox") {
		let checkbox: HTMLInputElement
		if (old instanceof HTMLInputElement) {
			checkbox = old
		} else {
			checkbox = document.createElement("input")
			if (old) old.replaceWith(checkbox)
		}
		checkbox.type = "checkbox"
		checkbox.checked = payload.checked
		if (item.id) {
			checkbox.onclick = () => {
				ctx.sender.send({
					type: "onClick",
					id: item.id,
					inx: item.inx,
				})
				ctx.sender.sendNow()
			}
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
			if (old) old.replaceWith(element)
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
			if (existingOptions.length !== payload.options.length || !existingOptions.every((opt, index) => opt.value === newOptions[index])) {
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
			select.value = payload.value
			if (old) old.replaceWith(select)
		}

		select.oninput = (e: any) => {
			ctx.sender.send({
				type: "onSelect",
				id: item.id,
				inx: item.inx,
				value: e.target.value
			})
			ctx.sender.sendNow()
		}

		return select
	}

	if (payload.type === "button") {
		let button: HTMLButtonElement
		if (old instanceof HTMLButtonElement) {
			button = old
		} else {
			button = document.createElement("button")
			if (old) old.replaceWith(button)
		}
		button.textContent = payload.title
		if (item.id) {
			button.onclick = () => {
				ctx.sender.send({
					type: "onClick",
					id: item.id,
					inx: item.inx,
				})
				ctx.sender.sendNow()
			}
		}
		return button
	}

	if (payload.type === "slider") {
		let slider: HTMLInputElement
		if (old instanceof HTMLInputElement) {
			slider = old
		} else {
			slider = document.createElement("input")
			if (old) old.replaceWith(slider)
		}
		slider.min = payload.min.toString()
		slider.max = payload.max.toString()
		slider.type = "range"
		slider.value = payload.value.toString()
		slider.step = payload.step.toString()
		if (item.id) {
			slider.oninput = (e: any) => {
				ctx.sender.send({
					type: "onSliderChange",
					id: item.id,
					inx: item.inx,
					value: parseInt(e.target.value)
				})
				ctx.sender.sendNow()
			}
		}
		return slider
	}

	if (payload.type === "textInput") {
		let input: HTMLInputElement
		if (old instanceof HTMLInputElement) {
			input = old
		} else {
			input = document.createElement("input")
			if (old) old.replaceWith(input)
		}
		input.placeholder = payload.placeholder as string
		input.value = payload.value
		if (item.id) {
			input.oninput = (e: any) => {
				ctx.sender.send({
					type: "onTextChanged",
					id: item.id,
					inx: item.inx,
					value: e.target.value,
				})
			}
		}

		return input
	}

	if (payload.type === "textarea") {
		let textarea: HTMLTextAreaElement
		if (old instanceof HTMLTextAreaElement) {
			textarea = old
		} else {
			textarea = document.createElement("textarea")
			if (old) old.replaceWith(textarea)
		}
		textarea.placeholder = payload.placeholder as string
		textarea.wrap = "off"
		textarea.style.resize = "none"
		textarea.style.overflowY = "hidden"
		textarea.style.minHeight = "20px"
		textarea.style.lineHeight = "20px"
		textarea.value = payload.value
		const rowCount = payload.value.split("\n").length
		textarea.style.height = rowCount * 20 + "px"
		textarea.oninput = (e: any) => {
			const value = e.target.value
			const rowCount = value.split("\n").length
			textarea.style.height = (rowCount + 1) * 20 + "px"

			if (item.id) {
				ctx.sender.send({
					type: "onTextChanged",
					id: item.id,
					inx: item.inx,
					value: e.target.value,
				})
			}
		}
		return textarea
	}

	if (payload.type === "table") {
		let table: HTMLTableElement
		if (old instanceof HTMLTableElement) {
			table = old
		} else {
			table = document.createElement("table")
			if (old) old.replaceWith(table)
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
			if (old) old.replaceWith(thead)
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
			if (old) old.replaceWith(tbody)
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
			if (old) old.replaceWith(tr)
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
			if (old) old.replaceWith(th)
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
			if (old) old.replaceWith(td)
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
			if (old) old.replaceWith(element)
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

	if (payload.type === "folderPicker") {
		let element: HTMLInputElement
		if (old instanceof HTMLInputElement) {
			element = old
		} else {
			element = document.createElement("input")
			if (old) old.replaceWith(element)
		}
		element.type = "file"
		element.webkitdirectory = true
		// element.multiple = true
		element.oninput = (e: any) => {
			console.log("oninput", e)
		}
		return element
	}

	if (payload.type === "flaotingLayout") {
		let element: HTMLDivElement
		if (old instanceof HTMLDivElement) {
			element = old
		} else {
			element = document.createElement("div")
			if (old) old.replaceWith(element)
		}
		element.style.position = "absolute"
		element.style.left = payload.x + "px"
		element.style.top = payload.y + "px"
		element.style.width = payload.width + "px"
		element.style.height = payload.height + "px"
		return element
	}
}

export const renderItem = (item: Item, ctx: Context, old?: Element | null) => {
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
	if (item.minWidth) element.style.minWidth = item.minWidth + "px"
	if (item.maxWidth) {
		element.style.maxWidth = item.maxWidth + "px"
	}
	if (item.minHeight) element.style.minHeight = item.minHeight + "px"
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
	if (item.marginLeft) {
		element.style.marginLeft = item.marginLeft + "px"
	}
	if (item.marginRight) {
		element.style.marginRight = item.marginRight + "px"
	}
	if (item.marginTop) {
		element.style.marginTop = item.marginTop + "px"
	}
	if (item.marginBottom) {
		element.style.marginBottom = item.marginBottom + "px"
	}
	if (item.padding) {
		element.style.padding = item.padding + "px"
	}
	if (item.paddingLeft) {
		element.style.paddingLeft = item.paddingLeft + "px"
	}
	if (item.paddingRight) {
		element.style.paddingRight = item.paddingRight + "px"
	}
	if (item.paddingTop) {
		element.style.paddingTop = item.paddingTop + "px"
	}
	if (item.paddingBottom) {
		element.style.paddingBottom = item.paddingBottom + "px"
	}
	if (item.border) {
		element.style.border = item.border
	}
	if (item.editable) {
		element.contentEditable = "true"
	}
	if (item.overflow) element.style.overflow = item.overflow
	return element
}