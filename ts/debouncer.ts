import { createLogger } from "./logger.ts"

const logger = createLogger("debouncer")

export class Deboncer {
    private timeout: any
    private value = ""
    public valueChanged = false
    private cb: ((text: string) => void) | null = null

    public change(text: string) {
        logger.info("change", text)

        this.valueChanged = true
        this.value = text
        clearTimeout(this.timeout)

        this.timeout = setTimeout(() => {
            logger.info("timeout")

            this.trigger()
        }, 500)
    }

    public unregister() {
        logger.info("unregister")

        this.cb = null
    }

    public register(cb: (v: string) => void) {
        logger.info("register")

        this.cb = cb
    }

    public trigger() {
        logger.info("trigger", this.value, this.valueChanged)

        if (this.timeout) {
            clearTimeout(this.timeout)
            this.timeout = null
            logger.info("timeout cleared")
        }

        if (!this.valueChanged) {
            logger.info("value is not changed")

            return
        }

        this.valueChanged = false
        
        if (this.cb) {
            logger.info("debouncer is triggered with", this.value)
            this.cb(this.value)  
        }

        this.value = ""
    }
}