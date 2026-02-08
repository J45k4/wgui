import { MessageSender } from "./message_sender.ts";
import { MessageToSrv, SrvMessage } from "./types.ts";

type OnMessage = (sender: MessageSender, msgs: SrvMessage[]) => void
type OnOpen = (sender: MessageSender) => void

export const connectWebsocket = (args: {
    onMessage: OnMessage
    onOpen: OnOpen
}) => {
    let ws: WebSocket | undefined
    const sessionStorageKey = "wgui.sid"

    const sender = new MessageSender((msgs: MessageToSrv[]) => {
        if (!ws) {
            return
        }

        ws.send(JSON.stringify(msgs))
    })

    const getSessionId = () => {
        const existing = window.sessionStorage.getItem(sessionStorageKey)
        if (existing) {
            return existing
        }
        const sid = (window.crypto?.randomUUID?.() ?? `sid-${Date.now()}-${Math.floor(Math.random() * 1_000_000_000)}`).replace(/[^a-zA-Z0-9_-]/g, "")
        window.sessionStorage.setItem(sessionStorageKey, sid)
        return sid
    }

    const createConnection = () => {
        const href = window.location.href
        const url = new URL(href)
        const wsProtocol = url.protocol === "https:" ? "wss" : "ws"
        const sid = encodeURIComponent(getSessionId())
        const wsUrl = `${wsProtocol}://${url.host}/ws?sid=${sid}`
        ws = new WebSocket(wsUrl)

        ws.onmessage = (e) => {
            const data = e.data.toString()
            const messages = JSON.parse(data) as SrvMessage[]
            args.onMessage(sender, messages)
        }
    
        ws.onopen = () => {
            args.onOpen(sender)
        }
    
        ws.onclose = () => {
            setTimeout(() => {
                createConnection()
            }, 1000)
        }

        ws.onerror = (e) => {
            console.error("error", e)
        }
    }

    createConnection()

    return {
        close: () => {
            if (!ws) {
                return
            }

            ws.close()
        },
        sender
    }
}
