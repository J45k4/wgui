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
		checkbox.classList.add("retro-checkbox")
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
		
		element.classList.add("retro-panel")

		if (payload.spacing) {
			element.style.gap = payload.spacing + "px"
		}
		if (payload.wrap) {
			element.classList.add("flex-wrap")
		}
		if (payload.flex) {
			element.style.display = "flex"
			element.style.flexDirection = payload.flex
			element.classList.add(payload.flex === "row" ? "flex-row" : "flex-col")
		}
		const horizontal = payload.horizontalResize || payload.horizontal_resize || payload.hresize
		const vertical = payload.vresize
		if (horizontal || vertical) {
			if (!element.style.overflow) {
				element.style.overflow = "auto"
			}
		}
		if (horizontal) {
			element.style.position = element.style.position || "relative"
			element.style.resize = "none"
			element.style.flexShrink = "0"
			let handle = element.querySelector(".wgui-resize-handle") as HTMLDivElement | null
			if (!handle) {
				handle = document.createElement("div")
				handle.className = "wgui-resize-handle"
				element.appendChild(handle)
			}
			handle.style.position = "absolute"
			handle.style.top = "0"
			handle.style.right = "0"
			handle.style.bottom = "0"
			handle.style.width = "8px"
			handle.style.cursor = "col-resize"
			handle.style.zIndex = "2"
			handle.style.background = "transparent"
			handle.onmousedown = (e: MouseEvent) => {
				e.preventDefault()
				const startX = e.clientX
				const startWidth = element.getBoundingClientRect().width
				const minWidth = item.minWidth || 0
				const maxWidth = item.maxWidth || 0
				const onMove = (moveEvent: MouseEvent) => {
					const next = startWidth + (moveEvent.clientX - startX)
					let width = next
					if (minWidth && width < minWidth) width = minWidth
					if (maxWidth && width > maxWidth) width = maxWidth
					element.style.width = `${width}px`
				}
				const onUp = () => {
					document.removeEventListener("mousemove", onMove)
					document.removeEventListener("mouseup", onUp)
					document.body.style.userSelect = ""
					document.body.style.cursor = ""
				}
				document.body.style.userSelect = "none"
				document.body.style.cursor = "col-resize"
				document.addEventListener("mousemove", onMove)
				document.addEventListener("mouseup", onUp)
			}
		}
		return element
	}

	if (payload.type === "select") {
		let select: HTMLSelectElement
		if (old instanceof HTMLSelectElement) {
			select = old
			// Use slice for broad compatibility instead of Array.from
			const existingOptions = Array.prototype.slice.call(old.options) as HTMLOptionElement[]
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

		select.classList.add("retro-input")
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
		button.classList.add("retro-button")
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

	if (payload.type === "img") {
		let image: HTMLImageElement
		if (old instanceof HTMLImageElement) {
			image = old
		} else {
			image = document.createElement("img")
			if (old) old.replaceWith(image)
		}
		image.src = payload.src
		image.alt = payload.alt ?? ""
		image.style.maxWidth = "100%"
		image.style.maxHeight = "100%"
		image.style.objectFit = payload.objectFit ?? "contain"
		image.loading = "lazy"
		image.classList.add("retro-panel")
		return image
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
		slider.classList.add("retro-input")
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
		input.classList.add("retro-input")
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
		textarea.classList.add("retro-input")
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
		table.classList.add("retro-table")
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
		element.classList.add("retro-text")
		if (item.id) {
			element.onclick = () => {
				ctx.sender.send({
					type: "onClick",
					id: item.id,
					inx: item.inx,
				})
				ctx.sender.sendNow()
			}
			element.classList.add("retro-clickable")
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

	if (payload.type === "modal") {
		let overlay: HTMLDivElement
		if (old instanceof HTMLDivElement && old.dataset.modal === "overlay") {
			overlay = old
			overlay.innerHTML = ""
		} else {
			overlay = document.createElement("div")
			overlay.dataset.modal = "overlay"
			overlay.setAttribute("role", "dialog")
			overlay.setAttribute("aria-modal", "true")
			if (old) old.replaceWith(overlay)
		}

		overlay.style.position = "fixed"
		overlay.style.left = "0"
		overlay.style.top = "0"
		overlay.style.width = "100vw"
		overlay.style.height = "100vh"
		overlay.style.display = payload.open ? "flex" : "none"
		overlay.style.alignItems = "center"
		overlay.style.justifyContent = "center"
		overlay.style.padding = "32px"
		overlay.style.boxSizing = "border-box"
		overlay.style.backgroundColor = "rgba(0, 0, 0, 0.45)"
		overlay.style.backdropFilter = "blur(2px)"
		overlay.style.zIndex = "1000"
		overlay.style.pointerEvents = payload.open ? "auto" : "none"
		overlay.setAttribute("aria-hidden", payload.open ? "false" : "true")

		renderChildren(overlay, payload.body, ctx)

		if (item.id) {
			overlay.onclick = (event: MouseEvent) => {
				if (event.target === overlay) {
					ctx.sender.send({
						type: "onClick",
						id: item.id,
						inx: item.inx,
					})
					ctx.sender.sendNow()
				}
			}
		} else {
			overlay.onclick = null
		}

		return overlay
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
		element.classList.add("grow")
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
