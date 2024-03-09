import { Deboncer } from "./debouncer.ts";
import { createLogger } from "./logger.ts";
import { getPathItem } from "./path.ts";
import { renderItem } from "./render.ts";
import { Context, SrvMessage } from "./types.ts";
import { connectWebsocket } from "./ws.ts";

const logger = createLogger("app")

window.onload = () => {
    const res = document.querySelector("body")

    if (!res) {
        return
    }

    res.innerHTML = ""

    res.style.display = "flex"
    res.style.flexDirection = "row"

    const content = document.createElement("div")
    content.style.border = "1px solid black"
    content.style.flexGrow = "1"

    res.appendChild(content)

    const root = document.createElement("div")
    content.appendChild(root)

    logger.info("root", res)

    const debouncer = new Deboncer()

    const {
        sender
    } = connectWebsocket({
        onMessage:  (sender, msgs: SrvMessage[]) => { 
            logger.info("root", root)

            const ctx: Context = {
                sender,
                debouncer
            }
            
            for (const message of msgs) {
                logger.info("process", message)

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
    
                logger.info("element", element)
    
                if (!element) {
                    logger.info(`cannot find element with path ${message.path}`)
                    continue
                }
    
                if (message.type === "replace") {
                    logger.info("replace", message)
                    const newEl = renderItem(message.item, ctx, element)
                
                    if (newEl) {
                        element.replaceWith(newEl)
                    }
                }
                
                if (message.type === "replaceAt") {
                    logger.info("replaceAt", message)
                    const newEl = renderItem(message.item, ctx)
    
                    if (newEl) {
                        element.children.item(message.inx)?.replaceWith(newEl)
                    }
                }
                
                if (message.type === "addFront") {
                    logger.info("addFront", message)
                    const newEl = renderItem(message.item, ctx)
    
                    if (newEl) {
                        element.prepend(newEl)
                    }
                }
                
                if (message.type === "addBack") {
                    logger.info("addBack", message)
                    const newEl = renderItem(message.item, ctx)
    
                    if (newEl) {
                        element.appendChild(newEl)
                    }
                }

                if (message.type === "insertAt") {
                    logger.info("insertAt", message)
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
            const params = new URLSearchParams(location.href)

            logger.info("onOpen", params)

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

        logger.info("url changed", location.href)

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