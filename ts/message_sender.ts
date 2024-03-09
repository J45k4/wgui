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
        logger.info("send", msg)

        this.queue.push(msg)
        this.sendNext()
    }

    private sendNext() {
        logger.info("sendNext")

        if (this.timeout) {
            logger.info("timeout already exist")

            return
        }

        this.timeout = setTimeout(() => {
            logger.info("timeout")

            this.sendNow()
        }, 500)
    }

    public sendNow() {
        logger.info("sendNow")

        clearInterval(this.timeout)
        this.timeout = 0

        if (this.queue.length === 0) {
            logger.info("queue is empty")

            return
        }

        logger.info("sendingNow", this.queue)
        
        this.sender(this.queue)
        this.queue = []
    }
}