import { Deboncer } from "./debouncer.ts";
import { getPathItem } from "./path.ts";
import { renderItem } from "./render.ts";
import { Context, SrvMessage } from "./types.ts";
import { connectWebsocket } from "./ws.ts";
import { injectRetroTheme } from "./theme.ts"; // added retro theme injection

window.onload = () => {
    injectRetroTheme(); // ensure theme styles are present
    const res = document.querySelector("body")

    if (!res) {
        return
    }

    res.innerHTML = ""

    res.style.display = "flex"
    res.style.flexDirection = "row"
    res.classList.add("retro-root")

    const content = document.createElement("div")
    content.style.flexGrow = "1"

    res.appendChild(content)

    const root = document.createElement("div")
    content.appendChild(root)
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

				// if (message.type === "setProp") {
				// 	element.setAttribute(message.prop, message.value)
				// }
            }
        },
        onOpen: (sender) => {
            const params = new URLSearchParams(location.href)
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
        const params = new URLSearchParams(location.href)
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