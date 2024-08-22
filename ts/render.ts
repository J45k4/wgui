import { createLogger } from "./logger.ts";
import { Context, Item } from "./types.ts";

const outerLogger = createLogger("render")

export const renderItem = (item: Item, ctx: Context, old?: Element) => {
    outerLogger.debug("renderItem", item, old)

    switch (item.type) {
        case "text": {
            if (old instanceof HTMLSpanElement) {
                old.innerHTML = item.text

                return
            }

            const span = document.createElement("span")

            span.innerText = item.text
            return span
        }
		case "slider": {
			if (old instanceof HTMLInputElement) {
				old.min = item.min.toString()
				old.max = item.max.toString()
				old.type = "range"
				old.value = item.value.toString()
				old.step = item.step.toString()
				old.style.width = item.width + "px"
				old.style.height = item.height + "px"

				return
			}

			const slider = document.createElement("input")

			slider.min = item.min.toString()
			slider.max = item.max.toString()
			slider.type = "range"
			slider.value = item.value.toString()
			slider.step = item.step.toString()
			slider.style.width = item.width + "px"
			slider.style.height = item.height + "px"

			slider.oninput = (e: any) => {
				if (item.id) {
					ctx.sender.send({
						type: "onSliderChange",
						id: item.id,
						value: parseInt(e.target.value, 10),
					})
					ctx.sender.sendNow()
				}
			}

			return slider
		}
        case "view": {
            outerLogger.debug("render view")

            let div: HTMLDivElement = old as HTMLDivElement

            if (old instanceof HTMLDivElement) {
                div.innerHTML = ""

                for (let i = 0; i < item.body.length; i++) {
                    const el = renderItem(item.body[i], ctx)
					if (el) {
                    	div.appendChild(el as any)
					}
                }
            } else {
                div = document.createElement("div")

                for (const i of item.body) {
                    const el = renderItem(i, ctx)
                    if (el) {
						div.appendChild(el as any)
					}
                }
            }          

            if (item.width != null) {
                div.style.width = item.width + "px"
            }
            
            if (item.height != null) {
                div.style.height = item.height + "px"
            }

            if (item.margin != null) {
                outerLogger.debug("setMargin", item.margin + "px")

                div.style.margin = item.margin + "px"
            }

            if (item.marginTop != null) {
                div.style.marginTop = item.marginTop + "px"
            }

            if (item.marginRight != null) {
                div.style.marginRight = item.marginRight + "px"
            }

            if (item.marginBottom != null) {
                div.style.marginBottom = item.marginBottom + "px"
            }

            if (item.marginLeft != null) {
                div.style.marginLeft = item.marginLeft + "px"
            }

            if (item.paddingTop != null) {
                div.style.paddingTop = item.paddingTop + "px"
            }

            if (item.paddingRight != null) {
                div.style.paddingRight = item.paddingRight + "px"
            }

            if (item.paddingBottom != null) {
                div.style.paddingBottom = item.paddingBottom + "px"
            }

            if (item.paddingLeft != null) {
                div.style.paddingLeft = item.paddingLeft + "px"
            }

            if (item.padding != null) {
                div.style.padding = item.padding + "px"
            }

			if (item.spacing != null) {
				div.style.gap = item.spacing + "px"
			}

			if (item.border != null) {
				div.style.border = item.border
			}

			if (item.wrap) {
				div.style.flexWrap = "wrap"
			}

			if (item.backgroundColor) {
				div.style.backgroundColor = item.backgroundColor
			}

			if (item.cursor) {
				div.style.cursor = item.cursor
			}

			if (item.maxWidth) {
				div.style.maxWidth = item.maxWidth + "px"
			}

            div.style.overflow = "auto"
            
            if (item.flex) {
                div.style.display = "flex"

                const flex = item.flex

                div.style.flexDirection = flex.flexDirection
                
                if (flex.grow) {
                    div.style.flexGrow = flex.grow.toString()
                }
            }

			if (item.id) {
				div.onclick = () => {
					ctx.sender.send({
						type: "onClick",
						id: item.id as string,
						name: item.id as string,
					})

					ctx.sender.sendNow()
				}
			}

            return div
        }
        case "button": {
            const logger = outerLogger.child(`button:${item.name}:${item.id}`)

            logger.debug("render button")

            if (old instanceof HTMLButtonElement) {
				old.id = item.id
                old.textContent = item.title
                return
            }

            const button = document.createElement("button")
			button.id = item.id
            button.innerText = item.title

            if (item.flex != null) {
                button.style.display = "flex"

                const flex = item.flex

                button.style.flexDirection = flex.flexDirection
                
                if (flex.grow) {
                    button.style.flexGrow = flex.grow.toString()
                }
            }

            button.onclick = () => {
                ctx.sender.send({
                    type: "onClick",
                    id: button.id,
                    name: item.name,
                })
                ctx.sender.sendNow()
            }

            return button
        }
        case "textInput": {
            const logger = outerLogger.child(`textInput:${item.name}:${item.id}`)

            logger.debug(`render textInput`, item)

            let registered = false

            if (old instanceof HTMLInputElement) {
                if (!registered || !ctx.debouncer.valueChanged) {
                    old.value = item.value
                }

                return
            }

            const input = document.createElement("input")
            input.placeholder = item.placeholder
            input.value = item.value

            if (item.flex != null) {
                input.style.display = "flex"

                const flex = item.flex

                input.style.flexDirection = flex.flexDirection
                
                if (flex.grow) {
                    input.style.flexGrow = flex.grow.toString()
                }
            }


            input.oninput = (e: any) => {
                logger.debug(`oninput ${input.value}`)

                ctx.debouncer.change(e.target.value)
            }
            
            input.onkeydown = (e) => {
                logger.debug(`keydown: ${e.key}`)

                if (e.key === "Enter") {
                    ctx.debouncer.trigger()

                    ctx.sender.send({
                        type: "onKeyDown",
                        id: item.id,
                        name: item.name,
                        keycode: e.key,
                    })

                    ctx.sender.sendNow()
                }
            }

            input.onfocus = () => {
                logger.debug("focus")

                ctx.debouncer.register(v => {
                    logger.debug(`changed to ${v}`)

                    ctx.sender.send({
                        type: "onTextChanged",
                        id: item.id,
                        name: item.name,
                        value: v,
                    })

                    ctx.sender.sendNow()
                })

                registered = true
            }

            input.onblur = () => {
                logger.debug("blur")

                ctx.debouncer.trigger()
                ctx.debouncer.unregister()

                registered = false
            }

            return input
        }
        case "checkbox": {
            const logger = outerLogger.child(`checkbox:${item.name}:${item.id}`)

            logger.debug("render checkbox")

            if (old instanceof HTMLInputElement) {
                old.checked = item.checked
                
                return
            }

            const checkbox = document.createElement("input")
            checkbox.type = "checkbox"
            checkbox.checked = item.checked
            checkbox.name = item.name

            checkbox.onclick = () => {
                ctx.sender.send({
                    type: "onClick",
                    id: item.id,
                    name: item.name,
                })

                ctx.sender.sendNow()
            }

            return checkbox
        }
        case "h1": {
            const logger = outerLogger.child(`h1:${item.text}`)

            logger.debug("render h1")

            if (old instanceof HTMLHeadingElement) {
                old.innerText = item.text

                return
            }

            const h1 = document.createElement("h1")

            h1.innerText = item.text

            return h1
        }
		case "select": {
			const logger = outerLogger.child(`select:${item.id}`)
		
			logger.debug("render select")
		
			if (old instanceof HTMLSelectElement) {
				if (old.value !== item.value) {
					old.value = item.value
				}
				if (old.style.width !== item.width + "px") {
					old.style.width = item.width + "px"
				}
				if (old.style.height !== item.height + "px") {
					old.style.height = item.height + "px"
				}
		
				const existingOptions = Array.from(old.options)
				const newOptions = item.options.map(option => option.value)
		
				// Update the options only if they differ
				if (existingOptions.length !== item.options.length ||
					!existingOptions.every((opt, index) => opt.value === newOptions[index])) {
		
					old.innerHTML = ""
					for (const option of item.options) {
						const opt = document.createElement("option")
						opt.value = option.value
						opt.text = option.name
						old.add(opt)
					}
				}
		
				return
			}
		
			console.log("creating new select")
		
			const select = document.createElement("select")
		
			for (const option of item.options) {
				const opt = document.createElement("option")
				opt.value = option.value
				opt.text = option.name
				select.add(opt)
			}
		
			// Set the value of the new select element
			select.value = item.value
			select.style.width = item.width + "px"
			select.style.height = item.height + "px"
		
			select.onchange = () => {
				ctx.sender.send({
					type: "onSelect",
					id: item.id,
					value: select.value,
				})
		
				ctx.sender.sendNow()
			}
		
			return select
		}
		// case "title": {
		// 	document.title = item.title
		// 	return undefined
		// }
        default:
            return document.createTextNode("Unknown item type")
    }
}