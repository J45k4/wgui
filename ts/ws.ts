import { createLogger } from "./logger.ts";
import { MessageSender } from "./message_sender.ts";
import { MessageToSrv, SrvMessage } from "./types.ts";

const logger = createLogger("ws")

type OnMessage = (sender: MessageSender, msgs: SrvMessage[]) => void
type OnOpen = (sender: MessageSender) => void

export const connectWebsocket = (args: {
    onMessage: OnMessage
    onOpen: OnOpen
}) => {
    let ws: WebSocket | undefined

    const sender = new MessageSender((msgs: MessageToSrv[]) => {
        if (!ws) {
            return
        }

        ws.send(JSON.stringify(msgs))
    })

    const createConnection = () => {
        ws = new WebSocket("ws://localhost:33445/ui")

        ws.onmessage = (e) => {
            const data = e.data.toString()
    
            logger.info("rawdata", data)
            const messages = JSON.parse(data) as SrvMessage[]
            logger.info("received", messages)
    
            args.onMessage(sender, messages)
        }
    
        ws.onopen = () => {
            logger.info("connected")

            args.onOpen(sender)
        }
    
        ws.onclose = () => {
            logger.info("disconnected")
    
            setTimeout(() => {
                createConnection()
            }, 1000)
        }
    }

    createConnection()

    return {
        close: () => {
            logger.debug("close")

            if (!ws) {
                return
            }

            ws.close()
        },
        sender
    }
}