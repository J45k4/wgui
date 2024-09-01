import { Context, Item } from "./types.ts";

export const renderItem = (item: Item, ctx: Context, old?: Element) => {
	let element: HTMLElement = old as HTMLElement

	const payload = item.payload
	switch (payload.type) {
		case "checkbox": {
			if (old instanceof HTMLInputElement) {
				element = old
				old.type = "checkbox"
				old.checked = payload.checked
				element = old
			} else {
				const checkbox = document.createElement("input")
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
				element = checkbox
			}
			break
		}
		case "layout": { 
			if (old instanceof HTMLDivElement) {
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
                element.style.flexDirection = payload.flex as any
                
                // if (flex.grow) {
                //     div.style.flexGrow = flex.grow.toString()
                // }
            }
			break
		}
		case "select": {
			if (old instanceof HTMLSelectElement) {
				element = old
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
				const select = document.createElement("select")
		
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
		
				element = select
			}
			break
		}
		case "button": {
            if (old instanceof HTMLButtonElement) {
				element = old
                old.textContent = payload.title
                element = old
            } else {
				const button = document.createElement("button")
				button.textContent = payload.title
				button.onclick = () => {
					ctx.sender.send({
						type: "onClick",
						id: item.id,
						inx: item.inx,
					})
					ctx.sender.sendNow()
				}
				element = button
			}
			break
		}
		case "slider": {
			if (old instanceof HTMLInputElement) {
				element = old
				old.min = payload.min.toString()
				old.max = payload.max.toString()
				old.type = "range"
				old.value = payload.value.toString()
				old.step = payload.step.toString()
			}
			break
		}
		case "text": {
			if (old instanceof HTMLSpanElement) {
				element = old
				old.innerText = payload.value + ""
			} else {
				element = document.createElement("span")
				element.innerText = payload.value + ""
			}
			break
		}
		case "textInput": {
			if (old instanceof HTMLInputElement) {
				element = old
				old.value = payload.value
				old.placeholder = payload.placeholder
			} else {
				const input = document.createElement("input")
				input.placeholder = payload.placeholder
				input.value = payload.value
				input.oninput = (e: any) => {
					ctx.sender.send({
						type: "onTextChanged",
						id: item.id,
						inx: item.inx,
						value: e.target.value,
					})
				}
				element = input
			}
			
			break
		}
		case "table": {
			if (old instanceof HTMLTableElement) {
				element = old
				element.innerHTML = ""

			} else {
				element = document.createElement("table")
			}

			const head = document.createElement("thead")
			const headRow = document.createElement("tr")
			for (const i of payload.head) {
				const th = document.createElement("th")
				th.appendChild(renderItem(i, ctx))
				headRow.appendChild(th)
			}
			head.appendChild(headRow)
			element.appendChild(head)
			const body = document.createElement("tbody")
			for (const row of payload.body) {
				const tr = document.createElement("tr")
				for (const i of row) {
					const td = document.createElement("td")
					td.appendChild(renderItem(i, ctx))
					tr.appendChild(td)
				}
				body.appendChild(tr)
			}
			element.appendChild(body)
			break
		}
		case "none": {
			element = document.createElement("div")
			element.innerText = "None"
			break
		}
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

	return element
}