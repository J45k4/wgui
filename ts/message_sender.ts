import { MessageToSrv } from "./types.ts";

type SendMsgs = (msg: MessageToSrv[]) => void

export class MessageSender {
    private sender: SendMsgs
    private queue: MessageToSrv[] = []
    private timeout = 0
    constructor(send: SendMsgs) {
        this.sender = send
    }

    public send(msg: MessageToSrv) {
		this.queue = this.queue.filter((m) => {
			if (m.type === msg.type) {
				return false
			}
			return true
		})
        this.queue.push(msg)
        this.sendNext()
    }

    private sendNext() {
        if (this.timeout) {
            clearTimeout(this.timeout)
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