import { MessageToSrv } from "./types.ts";

type SendMsgs = (msg: MessageToSrv[]) => void

const messageKey = (msg: MessageToSrv) => {
	const id = "id" in msg ? msg.id : ""
	const inx = "inx" in msg ? (msg.inx ?? "") : ""
	return `${msg.type}:${id}:${inx}`
}

export class MessageSender {
    private sender: SendMsgs
    private queue: MessageToSrv[] = []
    private timeout = 0
    constructor(send: SendMsgs) {
        this.sender = send
    }

    public send(msg: MessageToSrv) {
		const key = messageKey(msg)
		this.queue = this.queue.filter((m) => {
			if (messageKey(m) === key) {
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

	public sendImmediate(msg: MessageToSrv) {
		this.sender([msg])
	}
}
