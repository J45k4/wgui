import { MessageSender } from "./message_sender.ts";
import { MessageToSrv, SrvMessage } from "./types.ts";

type OnMessage = (sender: MessageSender, msgs: SrvMessage[]) => void
type OnOpen = (sender: MessageSender) => void
type OnConnectionChange = (connected: boolean) => void

export const connectWebsocket = (args: {
    onMessage: OnMessage
    onOpen: OnOpen
    onConnectionChange?: OnConnectionChange
}) => {
    let ws: WebSocket | undefined
    const sessionStorageKey = "wgui.sid"
    let inMemorySid: string | undefined

    const sender = new MessageSender((msgs: MessageToSrv[]) => {
        if (!ws || ws.readyState !== WebSocket.OPEN) {
            return
        }

        ws.send(JSON.stringify(msgs))
    })

    const getSessionId = () => {
        try {
            const existing = window.localStorage.getItem(sessionStorageKey)
            if (existing) {
                return existing
            }
            // Backward-compat: migrate previous sid storage.
            const legacy = window.sessionStorage.getItem(sessionStorageKey)
            if (legacy) {
                window.localStorage.setItem(sessionStorageKey, legacy)
                return legacy
            }
        } catch (_) {}
        if (inMemorySid) {
            return inMemorySid
        }
        const sid = (window.crypto?.randomUUID?.() ?? `sid-${Date.now()}-${Math.floor(Math.random() * 1_000_000_000)}`).replace(/[^a-zA-Z0-9_-]/g, "")
        try {
            window.localStorage.setItem(sessionStorageKey, sid)
        } catch (_) {
            inMemorySid = sid
        }
        return sid
    }

    const createConnection = () => {
        args.onConnectionChange?.(false)
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
            args.onConnectionChange?.(true)
            args.onOpen(sender)
        }
    
        ws.onclose = () => {
            args.onConnectionChange?.(false)
            setTimeout(() => {
                createConnection()
            }, 1000)
        }

        ws.onerror = (e) => {
            args.onConnectionChange?.(false)
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
