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
    const sender = new MessageSender((msgs: MessageToSrv[]) => {
        if (!ws || ws.readyState !== WebSocket.OPEN) {
            return
        }

        ws.send(JSON.stringify(msgs))
    })

    const createConnection = () => {
        args.onConnectionChange?.(false)
        const href = window.location.href
        const url = new URL(href)
        const wsProtocol = url.protocol === "https:" ? "wss" : "ws"
        const wsUrl = `${wsProtocol}://${url.host}/ws`
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
