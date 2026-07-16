import { ButtonEvents, Context, Item, ItemPayload } from "./types.ts";
import { disposeCustomComponentTree, mountCustomComponent } from "./custom_components.ts";
import { applyThreeTree, disposeThreeHost } from "./three_host.ts";



const renderChildren = (element: HTMLElement, items: Item[], ctx: Context) => {
	for (const item of items) {
		const child = renderItem(item, ctx)
		if (child) {
			element.appendChild(child)
		}
	}
}

const connectionStatusElements = () =>
	document.querySelectorAll<HTMLElement>("[data-wgui-connection-status]")

const applyConnectionStatus = (element: HTMLElement) => {
	const connected = document.documentElement.dataset.wguiSocketConnected === "true"
	const wantsConnected = element.dataset.wguiConnectionStatus === "connected"
	element.style.display = connected === wantsConnected ? element.dataset.wguiConnectionDisplay ?? "" : "none"
}

export const setConnectionStatus = (connected: boolean) => {
	document.documentElement.dataset.wguiSocketConnected = connected ? "true" : "false"
	for (const element of connectionStatusElements()) {
		applyConnectionStatus(element)
	}
}

const reconcileChildren = (element: HTMLElement, items: Item[], ctx: Context) => {
	for (let i = 0; i < items.length; i++) {
		const child = renderItem(items[i], ctx, element.children.item(i))
		if (child && !child.parentElement) {
			element.appendChild(child)
		}
	}
	while (element.children.length > items.length) {
		const child = element.children.item(items.length)
		disposeCustomComponentTree(child)
		child?.remove()
	}
}

const clearModalState = (element: HTMLElement, item: Item) => {
	if (item.payload.type === "modal" || element.dataset.modal !== "overlay") {
		return
	}

	delete element.dataset.modal
	element.removeAttribute("role")
	element.removeAttribute("aria-modal")
	element.removeAttribute("aria-hidden")
	element.onclick = null
	element.style.position = ""
	element.style.left = ""
	element.style.top = ""
	element.style.alignItems = ""
	element.style.justifyContent = ""
	element.style.backgroundColor = ""
	element.style.backdropFilter = ""
	element.style.zIndex = ""
	element.style.pointerEvents = ""
	element.style.overscrollBehavior = ""
	element.onwheel = null
	element.ontouchmove = null
}

const applyModalOverlayStyles = (overlay: HTMLDivElement, open: boolean, fillsViewport: boolean, padding: number) => {
	overlay.style.position = "fixed"
	overlay.style.left = "0"
	overlay.style.top = "0"
	overlay.style.width = "100vw"
	overlay.style.height = "100vh"
	overlay.style.display = open ? "flex" : "none"
	overlay.style.alignItems = fillsViewport ? "stretch" : "center"
	overlay.style.justifyContent = "center"
	overlay.style.padding = `${padding}px`
	overlay.style.boxSizing = "border-box"
	overlay.style.backgroundColor = "rgba(0, 0, 0, 0.45)"
	overlay.style.backdropFilter = "blur(2px)"
	overlay.style.zIndex = "1000"
	overlay.style.pointerEvents = open ? "auto" : "none"
	overlay.style.overscrollBehavior = "contain"
	overlay.setAttribute("aria-hidden", open ? "false" : "true")
}

const bindModalScrollBarrier = (overlay: HTMLDivElement) => {
	overlay.onwheel = (event: WheelEvent) => {
		event.stopPropagation()
		if (event.target === overlay) {
			event.preventDefault()
		}
	}
	overlay.ontouchmove = (event: TouchEvent) => {
		event.stopPropagation()
		if (event.target === overlay) {
			event.preventDefault()
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

type ButtonHoldConfig = {
	item: Item
	events: ButtonEvents | undefined
	ctx: Context
}

type ButtonHoldState = {
	active: boolean
	activePointer: number | null
	repeatTimer: number | null
	config: ButtonHoldConfig
}

const buttonHoldStates = new WeakMap<HTMLButtonElement, ButtonHoldState>()

const buttonEventId = (item: Item, events: ButtonEvents | undefined, name: keyof ButtonEvents): number | undefined => {
	if (events && typeof events[name] === "number") {
		return events[name] as number
	}
	if (!events && name === "click" && item.id) {
		return item.id
	}
	return undefined
}

const sendButtonEvent = (
	type: "onClick" | "onPress" | "onRelease" | "onRepeat",
	id: number | undefined,
	item: Item,
	ctx: Context,
) => {
	if (!id) {
		return
	}
	ctx.sender.send({
		type,
		id,
		inx: item.inx ?? undefined,
	})
	ctx.sender.sendNow()
}

const stopButtonHold = (state: ButtonHoldState, sendRelease: boolean) => {
	if (!state.active) {
		return
	}
	state.active = false
	state.activePointer = null
	if (state.repeatTimer !== null) {
		window.clearInterval(state.repeatTimer)
		state.repeatTimer = null
	}
	if (sendRelease) {
		const { item, events, ctx } = state.config
		sendButtonEvent("onRelease", buttonEventId(item, events, "release"), item, ctx)
	}
}

const configureButtonEvents = (button: HTMLButtonElement, item: Item, events: ButtonEvents | undefined, ctx: Context) => {
	let state = buttonHoldStates.get(button)
	if (!state) {
		state = {
			active: false,
			activePointer: null,
			repeatTimer: null,
			config: { item, events, ctx },
		}
		buttonHoldStates.set(button, state)

		button.onclick = () => {
			const { item, events, ctx } = state!.config
			sendButtonEvent("onClick", buttonEventId(item, events, "click"), item, ctx)
		}
		button.onpointerdown = (event) => {
			const { item, events, ctx } = state!.config
			const pressId = buttonEventId(item, events, "press")
			const repeatId = buttonEventId(item, events, "repeat")
			if (!pressId && !repeatId) {
				return
			}
			if (event.button !== undefined && event.button !== 0) {
				return
			}
			event.preventDefault()
			if (state!.active) {
				return
			}
			state!.active = true
			state!.activePointer = event.pointerId
			button.setPointerCapture?.(event.pointerId)
			sendButtonEvent("onPress", pressId, item, ctx)
			if (repeatId) {
				const interval = Math.max(1, events?.repeatInterval ?? 250)
				state!.repeatTimer = window.setInterval(() => {
					const { item, events, ctx } = state!.config
					sendButtonEvent("onRepeat", buttonEventId(item, events, "repeat"), item, ctx)
				}, interval)
			}
		}
		button.onpointerup = (event) => {
			if (state!.activePointer !== null && event.pointerId !== state!.activePointer) {
				return
			}
			event.preventDefault()
			stopButtonHold(state!, true)
		}
		button.onpointercancel = () => stopButtonHold(state!, true)
		button.onlostpointercapture = () => stopButtonHold(state!, true)
		button.onblur = () => stopButtonHold(state!, true)
	}
	state.config = { item, events, ctx }
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

type LayoutScrollState = {
	nearBottom: boolean
	lastSentAt: number
}

const layoutScrollStates = new WeakMap<HTMLElement, LayoutScrollState>()
const scrollNearBottomThreshold = 240
const scrollNearBottomThrottleMs = 250

const configureLayoutEvents = (
	element: HTMLElement,
	item: Item,
	payload: Extract<ItemPayload, { type: "layout" }>,
	ctx: Context,
) => {
	const id = payload.events?.scrollNearBottom
	if (!id) {
		element.onscroll = null
		layoutScrollStates.delete(element)
		return
	}
	let state = layoutScrollStates.get(element)
	if (!state) {
		state = { nearBottom: false, lastSentAt: 0 }
		layoutScrollStates.set(element, state)
	} else {
		state.nearBottom = false
	}
	element.onscroll = () => {
		const remaining = element.scrollHeight - element.scrollTop - element.clientHeight
		const isNearBottom = remaining <= scrollNearBottomThreshold
		if (!isNearBottom) {
			state!.nearBottom = false
			return
		}
		if (state!.nearBottom) {
			return
		}
		const now = Date.now()
		if (now - state!.lastSentAt < scrollNearBottomThrottleMs) {
			return
		}
		state!.nearBottom = true
		state!.lastSentAt = now
		ctx.sender.send({
			type: "onScrollNearBottom",
			id,
			inx: item.inx ?? undefined,
		})
		ctx.sender.sendNow()
	}
}

const bindSliderControlTracking = (slider: HTMLInputElement) => {
	if (slider.dataset.wguiSliderTracking === "1") {
		return
	}
	slider.dataset.wguiSliderTracking = "1"
	const begin = () => {
		slider.dataset.wguiSliderActive = "1"
	}
	const end = () => {
		delete slider.dataset.wguiSliderActive
	}
	slider.addEventListener("pointerdown", begin)
	slider.addEventListener("pointerup", end)
	slider.addEventListener("pointercancel", end)
	slider.addEventListener("keydown", begin)
	slider.addEventListener("keyup", end)
	slider.addEventListener("blur", end)
}

const isSliderUserControlled = (slider: HTMLInputElement): boolean =>
	slider.dataset.wguiSliderActive === "1" || document.activeElement === slider

const textControlKey = (item: Item): string => `${item.id ?? ""}:${item.inx ?? ""}`

const isTextControlUserControlled = (control: HTMLInputElement | HTMLTextAreaElement): boolean =>
	document.activeElement === control

const syncTextControlValue = (
	control: HTMLInputElement | HTMLTextAreaElement,
	value: string,
	item: Item,
) => {
	const key = textControlKey(item)
	const sameControl = control.dataset.wguiTextControlKey === key
	control.dataset.wguiTextControlKey = key
	if (sameControl && isTextControlUserControlled(control)) {
		return
	}
	if (control.value !== value) {
		control.value = value
	}
}

const pathQuery = (search: string): { [key: string]: string } => {
	const params = new URLSearchParams(search)
	const query: { [key: string]: string } = {}
	params.forEach((value, key) => {
		query[key] = value
	})
	return query
}

const navigateLink = (event: MouseEvent, anchor: HTMLAnchorElement, ctx: Context) => {
	if (
		event.button !== 0 ||
		event.metaKey ||
		event.ctrlKey ||
		event.shiftKey ||
		event.altKey ||
		(anchor.target && anchor.target !== "_self") ||
		anchor.hasAttribute("download")
	) {
		return
	}
	const target = new URL(anchor.href, window.location.href)
	if (target.origin !== window.location.origin) {
		return
	}
	event.preventDefault()
	const next = `${target.pathname}${target.search}${target.hash}`
	if (next !== `${location.pathname}${location.search}${location.hash}`) {
		history.pushState({}, "", next)
	}
	ctx.sender.send({
		type: "pathChanged",
		path: location.pathname,
		query: pathQuery(location.search),
	})
	ctx.sender.sendNow()
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
			reconcileChildren(element, payload.body, ctx)
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
			let handle = Array.prototype.find.call(
				element.children,
				(child: Element) => child instanceof HTMLDivElement && child.dataset.wguiResizeHandle === "true",
			) as HTMLDivElement | undefined
			if (!handle) {
				handle = document.createElement("div")
				handle.className = "wgui-resize-handle"
				handle.dataset.wguiResizeHandle = "true"
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
					element.dataset.wguiResizedWidth = `${width}`
					element.style.width = `${width}px`
					element.style.flexBasis = `${width}px`
					element.style.setProperty("flex", `0 0 ${width}px`, "important")
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
		configureLayoutEvents(element, item, payload, ctx)
		return element
	}

	if (payload.type === "form") {
		let form: HTMLFormElement
		if (old instanceof HTMLFormElement) {
			form = old
			reconcileChildren(form, payload.body, ctx)
		} else {
			form = document.createElement("form")
			if (old) old.replaceWith(form)
			renderChildren(form, payload.body, ctx)
		}
		form.action = payload.action || item.action || ""
		form.method = payload.method || item.method || "post"
		form.style.display = "flex"
		form.style.flexDirection = "column"
		if (payload.spacing) {
			form.style.gap = payload.spacing + "px"
		}
		return form
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
			select.value = payload.value
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
		configureButtonEvents(button, item, payload.events, ctx)
		return button
	}

	if (payload.type === "link") {
		let anchor: HTMLAnchorElement
		if (old instanceof HTMLAnchorElement) {
			anchor = old
		} else {
			anchor = document.createElement("a")
			if (old) old.replaceWith(anchor)
		}
		anchor.href = payload.href
		anchor.textContent = payload.text
		anchor.style.color = "inherit"
		anchor.style.textDecoration = "none"
		anchor.onclick = (event) => navigateLink(event, anchor, ctx)
		return anchor
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

	if (payload.type === "video") {
		let video: HTMLVideoElement
		if (old instanceof HTMLVideoElement) {
			video = old
		} else {
			video = document.createElement("video")
			if (old) old.replaceWith(video)
		}
		video.dataset.wguiRtc = "video"
		video.dataset.wguiRtcRoom = payload.room
		video.dataset.wguiRtcLocal = payload.local ? "1" : "0"
		video.autoplay = payload.autoplay
		video.muted = payload.muted
		video.controls = payload.controls
		video.playsInline = true
		video.style.backgroundColor = "#000000"
		video.style.objectFit = "cover"
		return video
	}

	if (payload.type === "audio") {
		let audio: HTMLAudioElement
		if (old instanceof HTMLAudioElement) {
			audio = old
		} else {
			audio = document.createElement("audio")
			if (old) old.replaceWith(audio)
		}
		audio.dataset.wguiRtc = "audio"
		audio.dataset.wguiRtcRoom = payload.room
		audio.dataset.wguiRtcLocal = payload.local ? "1" : "0"
		audio.autoplay = payload.autoplay
		audio.muted = payload.muted
		audio.controls = payload.controls
		return audio
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
		slider.step = payload.step.toString()
		bindSliderControlTracking(slider)
		const sliderKey = `${item.id ?? ""}:${item.inx ?? ""}`
		const sameSlider = slider.dataset.wguiSliderKey === sliderKey
		slider.dataset.wguiSliderKey = sliderKey
		if (!sameSlider || !isSliderUserControlled(slider)) {
			slider.value = payload.value.toString()
		}
		if (item.id) {
			let sliderFlushTimeout = 0
			const flushSliderChange = () => {
				if (sliderFlushTimeout) {
					return
				}
				sliderFlushTimeout = setTimeout(() => {
					sliderFlushTimeout = 0
					ctx.sender.sendNow()
				}, 50)
			}
			const flushSliderChangeNow = () => {
				if (sliderFlushTimeout) {
					clearTimeout(sliderFlushTimeout)
					sliderFlushTimeout = 0
				}
				ctx.sender.sendNow()
			}
			const sendSliderChange = (value: number) => {
				ctx.sender.send({
					type: "onSliderChange",
					id: item.id,
					inx: item.inx,
					value
				})
			}
			slider.oninput = (e: any) => {
				sendSliderChange(parseInt(e.target.value))
				flushSliderChange()
			}
			slider.onchange = (e: any) => {
				sendSliderChange(parseInt(e.target.value))
				flushSliderChangeNow()
			}
		}
		return slider
	}

	if (payload.type === "datePicker") {
		let input: HTMLInputElement
		if (old instanceof HTMLInputElement) {
			input = old
		} else {
			input = document.createElement("input")
			if (old) old.replaceWith(input)
		}
		input.type = "date"
		input.placeholder = payload.placeholder as string
		syncTextControlValue(input, payload.value, item)
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

	if (payload.type === "textInput") {
		let input: HTMLInputElement
		if (old instanceof HTMLInputElement) {
			input = old
		} else {
			input = document.createElement("input")
			if (old) old.replaceWith(input)
		}
		input.type = payload.inputType || payload.input_type || "text"
		input.placeholder = payload.placeholder as string
		syncTextControlValue(input, payload.value, item)
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
		syncTextControlValue(textarea, payload.value, item)
		const rowCount = textarea.value.split("\n").length
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
			reconcileChildren(table, payload.items, ctx)
		} else {
			table = document.createElement("table")
			if (old) old.replaceWith(table)
			renderChildren(table, payload.items, ctx)
		}
		return table
	}

	if (payload.type === "thead") {
		let thead: HTMLTableSectionElement
		if (old instanceof HTMLTableSectionElement) {
			thead = old
			reconcileChildren(thead, payload.items, ctx)
		} else {
			thead = document.createElement("thead")
			if (old) old.replaceWith(thead)
			renderChildren(thead, payload.items, ctx)
		}
		return thead
	}

	if (payload.type === "tbody") {
		let tbody: HTMLTableSectionElement
		if (old instanceof HTMLTableSectionElement) {
			tbody = old
			reconcileChildren(tbody, payload.items, ctx)
		} else {
			tbody = document.createElement("tbody")
			if (old) old.replaceWith(tbody)
			renderChildren(tbody, payload.items, ctx)
		}
		return tbody
	}

	if (payload.type === "tr") {
		let tr: HTMLTableRowElement
		if (old instanceof HTMLTableRowElement) {
			tr = old
			reconcileChildren(tr, payload.items, ctx)
		} else {
			tr = document.createElement("tr")
			if (old) old.replaceWith(tr)
			renderChildren(tr, payload.items, ctx)
		}
		return tr
	}

	if (payload.type === "th") {
		let th: HTMLTableCellElement
		if (old instanceof HTMLTableCellElement) {
			th = old
			reconcileChildren(th, [payload.item], ctx)
		} else {
			th = document.createElement("th")
			if (old) old.replaceWith(th)
			renderChildren(th, [payload.item], ctx)
		}
		return th
	}

	if (payload.type === "td") {
		let td: HTMLTableCellElement
		if (old instanceof HTMLTableCellElement) {
			td = old
			reconcileChildren(td, [payload.item], ctx)
		} else {
			td = document.createElement("td")
			if (old) old.replaceWith(td)
			renderChildren(td, [payload.item], ctx)
		}
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
		} else {
			overlay = document.createElement("div")
			overlay.dataset.modal = "overlay"
			overlay.setAttribute("role", "dialog")
			overlay.setAttribute("aria-modal", "true")
			if (old) old.replaceWith(overlay)
		}

		const fillsViewport = payload.body.some(child => child.fill)
		applyModalOverlayStyles(overlay, payload.open, fillsViewport, item.padding || 32)

		if (old instanceof HTMLDivElement && old.dataset.modal === "overlay") {
			reconcileChildren(overlay, payload.body, ctx)
		} else {
			renderChildren(overlay, payload.body, ctx)
		}
		for (const [index, child] of Array.from(overlay.children).entries()) {
			if (child instanceof HTMLElement) {
				const fillsViewport = !!payload.body[index]?.fill
				child.style.maxWidth = fillsViewport ? "none" : "calc(100vw - 64px)"
				child.style.maxHeight = fillsViewport ? "none" : "calc(100vh - 64px)"
				child.style.overscrollBehavior = "contain"
			}
		}
		bindModalScrollBarrier(overlay)

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

	if (payload.type === "connectionStatus") {
		let element: HTMLDivElement
		if (old instanceof HTMLDivElement && old.dataset.wguiConnectionStatus) {
			element = old
			reconcileChildren(element, payload.body, ctx)
		} else {
			element = document.createElement("div")
			if (old) old.replaceWith(element)
			renderChildren(element, payload.body, ctx)
		}
		element.dataset.wguiConnectionStatus = payload.connected ? "connected" : "disconnected"
		element.dataset.wguiConnectionDisplay = "flex"
		element.style.flexDirection = payload.flex ?? "column"
		element.style.gap = payload.spacing ? `${payload.spacing}px` : ""
		element.style.flexWrap = payload.wrap ? "wrap" : ""
		applyConnectionStatus(element)
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

	if (payload.type === "custom") {
		let element: HTMLDivElement
		if (old instanceof HTMLDivElement && old.dataset.wguiCustom === "true") {
			element = old
		} else {
			disposeCustomComponentTree(old)
			element = document.createElement("div")
			if (old) old.replaceWith(element)
		}
		element.dataset.wguiCustom = "true"
		element.dataset.wguiCustomName = payload.name
		mountCustomComponent(element, item, payload, ctx)
		return element
	}
}

export const renderItem = (item: Item, ctx: Context, old?: Element | null) => {
	if (old instanceof HTMLCanvasElement && item.payload.type !== "threeView") {
		disposeThreeHost(old)
	}
	if (old instanceof HTMLElement && old.dataset.wguiCustom === "true" && item.payload.type !== "custom") {
		disposeCustomComponentTree(old)
	}
	const element = renderPayload(item, ctx, old)

	if (!element) {
		return
	}

	if (item.payload.type === "modal") {
		return element
	}

	clearModalState(element, item)

	element.style.width = item.fill ? "100%" : item.width ? item.width + "px" : ""
	if (
		element instanceof HTMLElement &&
		item.payload.type === "layout" &&
		(item.payload.horizontalResize || item.payload.horizontal_resize || item.payload.hresize) &&
		element.dataset.wguiResizedWidth
	) {
		element.style.width = `${element.dataset.wguiResizedWidth}px`
		element.style.flexBasis = `${element.dataset.wguiResizedWidth}px`
		element.style.setProperty("flex", `0 0 ${element.dataset.wguiResizedWidth}px`, "important")
	}
	element.style.boxSizing = item.fill ? "border-box" : ""
	element.style.height = item.height ? item.height + "px" : ""
	element.style.minWidth = item.minWidth !== undefined ? item.minWidth + "px" : ""
	element.style.maxWidth = item.maxWidth ? item.maxWidth + "px" : ""
	element.style.minHeight = item.minHeight ? item.minHeight + "px" : ""
	element.style.maxHeight = item.maxHeight ? item.maxHeight + "px" : ""
	element.style.flexGrow = item.grow ? item.grow.toString() : ""
	element.classList.toggle("grow", !!item.grow)
	element.style.backgroundColor = item.backgroundColor || ""
	element.style.color = item.color || ""
	if (item.breakWords) {
		element.style.overflowWrap = "anywhere"
		element.style.wordBreak = "break-word"
	} else {
		element.style.overflowWrap = ""
		element.style.wordBreak = ""
	}
	element.style.textAlign = item.textAlign || ""
	element.style.whiteSpace = item.whiteSpace || ""
	element.style.cursor = item.cursor || ""
	element.style.margin = ""
	element.style.marginLeft = ""
	element.style.marginRight = ""
	element.style.marginTop = ""
	element.style.marginBottom = ""
	if (item.margin) element.style.margin = item.margin + "px"
	if (item.marginLeft) element.style.marginLeft = item.marginLeft + "px"
	if (item.marginRight) element.style.marginRight = item.marginRight + "px"
	if (item.marginTop) element.style.marginTop = item.marginTop + "px"
	if (item.marginBottom) element.style.marginBottom = item.marginBottom + "px"
	element.style.padding = ""
	element.style.paddingLeft = ""
	element.style.paddingRight = ""
	element.style.paddingTop = ""
	element.style.paddingBottom = ""
	if (item.padding) element.style.padding = item.padding + "px"
	if (item.paddingLeft) element.style.paddingLeft = item.paddingLeft + "px"
	if (item.paddingRight) element.style.paddingRight = item.paddingRight + "px"
	if (item.paddingTop) element.style.paddingTop = item.paddingTop + "px"
	if (item.paddingBottom) element.style.paddingBottom = item.paddingBottom + "px"
	element.style.border = item.border || ""
	if (item.editable) {
		element.contentEditable = "true"
	} else {
		element.removeAttribute("contenteditable")
	}
	if (item.name) {
		element.setAttribute("name", item.name)
	} else {
		element.removeAttribute("name")
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
		!(element instanceof HTMLInputElement) &&
		!(element instanceof HTMLSelectElement) &&
		!(element instanceof HTMLTextAreaElement)
	) {
		bindAutoClick(element, item, ctx)
	}
	return element
}
