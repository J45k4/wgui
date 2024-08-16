import { createLogger } from "./logger.ts";
import { MessageToSrv } from "./types.ts";

const logger = createLogger("message_sender")

type SendMsgs = (msg: MessageToSrv[]) => void

export class MessageSender {
    private sender: SendMsgs
    private queue: MessageToSrv[] = []
    private timeout = 0
    constructor(send: SendMsgs) {
        this.sender = send
    }

    public send(msg: MessageToSrv) {
        this.queue.push(msg)
        this.sendNext()
    }

    private sendNext() {
        if (this.timeout) {
            logger.info("timeout already exist")

            return
        }

        this.timeout = setTimeout(() => {
            this.sendNow()
        }, 500)
    }

    public sendNow() {
        clearInterval(this.timeout)
        this.timeout = 0
        if (this.queue.length === 0) {
            return
        }     
        this.sender(this.queue)
        this.queue = []
    }
}