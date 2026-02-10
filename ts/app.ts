import { Deboncer } from "./debouncer.ts";
import { getPathItem } from "./path.ts";
import { renderItem } from "./render.ts";
import { applyThreePatch } from "./three_host.ts";
import { Context, PropValue, SetPropSet, SrvMessage } from "./types.ts";
import { connectWebsocket } from "./ws.ts";

const getSetPropValue = (value?: PropValue) => {
    if (!value) {
        return undefined
    }
    if (value.String != null) {
        return value.String
    }
    if (value.Number != null) {
        return value.Number.toString()
    }
    return undefined
}

const applySetProp = (element: Element, set: SetPropSet) => {
    const value = getSetPropValue(set.value)
    if (value == null) {
        return
    }

    if (!(element instanceof HTMLElement)) {
        return
    }

    switch (set.key) {
        case "BackgroundColor":
            element.style.backgroundColor = value
            break
        case "Border":
            element.style.border = value
            break
        case "Spacing": {
            const parsed = Number(value)
            element.style.gap = isNaN(parsed) ? value : `${parsed}px`
            break
        }
        case "FlexDirection":
            element.style.display = "flex"
            element.style.flexDirection = value
            break
        case "Grow":
            element.style.flexGrow = value
            break
        case "Width":
            element.style.width = value === "0" ? "" : `${value}px`
            break
        case "Height":
            element.style.height = value === "0" ? "" : `${value}px`
            break
        case "MinWidth":
            element.style.minWidth = value === "0" ? "" : `${value}px`
            break
        case "MaxWidth":
            element.style.maxWidth = value === "0" ? "" : `${value}px`
            break
        case "MinHeight":
            element.style.minHeight = value === "0" ? "" : `${value}px`
            break
        case "MaxHeight":
            element.style.maxHeight = value === "0" ? "" : `${value}px`
            break
        case "Padding":
            element.style.padding = value === "0" ? "" : `${value}px`
            break
        case "Overflow":
            element.style.overflow = value
            break
        case "ID":
            element.id = value
            break
    }
}

window.onload = () => {
    const res = document.querySelector("body")

    if (!res) {
        return
    }

    res.style.display = "flex"
    res.style.flexDirection = "row"
    res.style.height = "100vh"
    res.style.margin = "0"
    res.style.width = "100%"

    let root = res.querySelector("#wgui-root") as HTMLDivElement | null

    if (!root) {
        res.innerHTML = ""
        root = document.createElement("div")
        root.id = "wgui-root"
        res.appendChild(root)
    }
    root.style.display = "flex"
    root.style.flexDirection = "column"
    root.style.flexGrow = "1"
    root.style.minHeight = "100vh"
    root.style.width = "100%"
    const debouncer = new Deboncer()

    const {
        sender
    } = connectWebsocket({
        onMessage:  (sender, msgs: SrvMessage[]) => { 
            const ctx: Context = {
                sender,
                debouncer
            }
            
            for (const message of msgs) {
                if (message.type === "pushState") {
                    history.pushState({}, "", message.url)

                    sender.send({
                        type: "pathChanged",
                        path: location.pathname,
                        query: {}
                    })
                    sender.sendNow()

                    continue
                }

                if (message.type === "replaceState") {
                    history.replaceState({}, "", message.url)
                    continue
                }

                if (message.type === "setQuery") {
                    const params = new URLSearchParams(location.search)
                    for (const key of Object.keys(message.query)) {
                        const value = message.query[key]

                        if (value != null) {
                            params.set(key, value)
                        }
                    }
                    history.replaceState({}, "", `${params.toString()}`)
                    continue   
                }

                if (message.type === "setTitle") {
                    document.title = message.title
                    continue
                }

				if (message.type === "threePatch") {
					const target = getPathItem(message.path, root)
					if (target) {
						applyThreePatch(target, message.ops)
					}
					continue
				}

                if (message.type === "setProp") {
                    const target = getPathItem(message.path, root)

                    if (!target) {
                        continue
                    }

                    for (const set of message.sets) {
                        applySetProp(target, set)
                    }

                    continue
                }
    
                const element = getPathItem(message.path, root)
    
                if (!element) {
                    continue
                }
    
                if (message.type === "replace") {
                    renderItem(message.item, ctx, element)
                }
                
                if (message.type === "replaceAt") {
                    renderItem(message.item, ctx, element.children.item(message.inx))
                }
                
                if (message.type === "addFront") {
                    const newEl = renderItem(message.item, ctx)
    
                    if (newEl) {
                        element.prepend(newEl)
                    }
                }
                
                if (message.type === "addBack") {
                    const newEl = renderItem(message.item, ctx)
    
                    if (newEl) {
                        element.appendChild(newEl)
                    }
                }

                if (message.type === "insertAt") {
                    const newEl = renderItem(message.item, ctx)
    
                    if (newEl) {
                        const child = element.children.item(message.inx)

                        child?.after(newEl)
                    }
                }
    
                if (message.type === "removeInx") {
                    element.children.item(message.inx)?.remove()
                }

            }
        },
        onOpen: (sender) => {
            const params = new URLSearchParams(location.search)
            const query: { [key: string]: string } = {}
            params.forEach((value, key) => {
                query[key] = value
            })
            sender.send({
                type: "pathChanged",
                path: location.pathname,
                query: query
            })

            sender.sendNow()
        }
    })

    window.addEventListener("popstate", (evet) => {
        const params = new URLSearchParams(location.search)
        const query: { [key: string]: string } = {}
        params.forEach((value, key) => {
            query[key] = value
        })
        sender.send({
            type: "pathChanged",
            path: location.pathname,
            query,
        })

        sender.sendNow()
    })        
}
