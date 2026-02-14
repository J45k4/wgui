import { Context, Item, ItemPayload } from "./types.ts";
import { applyThreeTree, disposeThreeHost } from "./three_host.ts";



const renderChildren = (element: HTMLElement, items: Item[], ctx: Context) => {
	for (const item of items) {
		const child = renderItem(item, ctx)
		if (child) {
			element.appendChild(child)
		}
	}
}

const fileToDataUrl = (file: File): Promise<string> =>
	new Promise<string>((resolve, reject) => {
		const reader = new FileReader()
		reader.onload = () => resolve((reader.result as string) || "")
		reader.onerror = () => reject(reader.error)
		reader.readAsDataURL(file)
	})

const setImageDropActive = (input: HTMLInputElement, active: boolean) => {
	if (active) {
		input.style.outline = "2px dashed #2f7dd1"
		input.style.outlineOffset = "2px"
		input.style.backgroundColor = "rgba(47, 125, 209, 0.08)"
		return
	}
	input.style.outline = ""
	input.style.outlineOffset = ""
	input.style.backgroundColor = ""
}

const hasFileDragPayload = (event: DragEvent): boolean => {
	const dt = event.dataTransfer
	if (!dt) {
		return false
	}
	if (dt.files && dt.files.length > 0) {
		return true
	}
	if (dt.items && dt.items.length > 0) {
		for (const item of dt.items) {
			if (item.kind === "file") {
				return true
			}
		}
	}
	if (dt.types && dt.types.length > 0) {
		for (const t of dt.types) {
			if (t === "Files") {
				return true
			}
		}
	}
	return false
}

const sendImageFileAsTextChanged = async (ctx: Context, id: number, inx: number | undefined, file: File) => {
	const value = await fileToDataUrl(file).catch(() => "")
	if (!value) {
		return
	}
	ctx.sender.send({
		type: "onTextChanged",
		id,
		inx,
		value,
	})
	ctx.sender.sendNow()
}

const bindAutoClick = (element: HTMLElement, item: Item, ctx: Context) => {
	const autoKey = "1"
	if (item.id) {
		if (!element.onclick) {
			element.dataset.wguiAutoClick = autoKey
			element.onclick = () => {
				ctx.sender.send({
					type: "onClick",
					id: item.id,
					inx: item.inx,
				})
				ctx.sender.sendNow()
			}
		}
		return
	}
	if (element.dataset.wguiAutoClick === autoKey) {
		element.onclick = null
		delete element.dataset.wguiAutoClick
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
			element.classList.add("flex-wrap")
		} else {
			element.style.flexWrap = ""
			element.classList.remove("flex-wrap")
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
		input.ondragover = (event: DragEvent) => {
			if (!hasFileDragPayload(event)) {
				setImageDropActive(input, false)
				return
			}
			event.preventDefault()
			event.stopPropagation()
			if (event.dataTransfer) {
				event.dataTransfer.dropEffect = "copy"
			}
			setImageDropActive(input, true)
		}
		input.ondragenter = (event: DragEvent) => {
			if (!hasFileDragPayload(event)) {
				return
			}
			event.preventDefault()
			event.stopPropagation()
			setImageDropActive(input, true)
		}
		input.ondragleave = () => {
			setImageDropActive(input, false)
		}
		input.ondrop = async (event: DragEvent) => {
			const dropped = event.dataTransfer?.files?.[0]
			if (!dropped || !dropped.type.startsWith("image/")) {
				setImageDropActive(input, false)
				return
			}
			event.preventDefault()
			event.stopPropagation()
			setImageDropActive(input, false)
			const picker = document.querySelector('input[data-wgui-role="folder-picker"]') as HTMLInputElement | null
			const pickerId = picker?.dataset.wguiId ? Number(picker.dataset.wguiId) : 0
			if (!pickerId) {
				return
			}
			await sendImageFileAsTextChanged(ctx, pickerId, undefined, dropped)
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
		element.webkitdirectory = false
		element.multiple = false
		element.accept = "image/*"
		element.dataset.wguiRole = "folder-picker"
		element.dataset.wguiId = item.id ? item.id.toString() : ""
		element.oninput = async (e: any) => {
			if (!item.id) {
				return
			}
			const file: File | undefined = e?.target?.files?.[0]
			if (!file) {
				return
			}
			await sendImageFileAsTextChanged(ctx, item.id, item.inx, file)
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
		for (const child of overlay.children) {
			if (child instanceof HTMLElement) {
				child.style.maxWidth = "calc(100vw - 64px)"
				child.style.maxHeight = "calc(100vh - 64px)"
			}
		}

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

	if (payload.type === "threeView") {
		let canvas: HTMLCanvasElement
		if (old instanceof HTMLCanvasElement) {
			canvas = old
		} else {
			canvas = document.createElement("canvas")
			if (old) old.replaceWith(canvas)
		}
		canvas.dataset.wguiThree = "true"
		canvas.style.display = "block"
		canvas.style.width = "100%"
		canvas.style.height = "100%"
		applyThreeTree(canvas, payload.root)
		return canvas
	}
}

export const renderItem = (item: Item, ctx: Context, old?: Element | null) => {
	if (old instanceof HTMLCanvasElement && item.payload.type !== "threeView") {
		disposeThreeHost(old)
	}
	const element = renderPayload(item, ctx, old)

	if (!element) {
		return
	}

	element.style.width = item.width ? item.width + "px" : ""
	element.style.height = item.height ? item.height + "px" : ""
	element.style.minWidth = item.minWidth ? item.minWidth + "px" : ""
	element.style.maxWidth = item.maxWidth ? item.maxWidth + "px" : ""
	element.style.minHeight = item.minHeight ? item.minHeight + "px" : ""
	element.style.maxHeight = item.maxHeight ? item.maxHeight + "px" : ""
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
	if (item.overflow) {
		element.style.overflow = item.overflow
	} else {
		const isLayoutWithAutoOverflow =
			item.payload.type === "layout" &&
			(item.payload.horizontalResize ||
				item.payload.horizontal_resize ||
				item.payload.hresize ||
				item.payload.vresize)
		if (!isLayoutWithAutoOverflow) {
			element.style.overflow = ""
		}
	}
	if (
		item.payload.type !== "modal" &&
		!(element instanceof HTMLInputElement) &&
		!(element instanceof HTMLSelectElement) &&
		!(element instanceof HTMLTextAreaElement)
	) {
		bindAutoClick(element, item, ctx)
	}
	return element
}
