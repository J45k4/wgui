export class Deboncer {
    private timeout: any
    private value = ""
    public valueChanged = false
    private cb: ((text: string) => void) | null = null

    public change(text: string) {
        this.valueChanged = true
        this.value = text
        clearTimeout(this.timeout)

        this.timeout = setTimeout(() => {
            this.trigger()
        }, 500)
    }

    public unregister() {
        this.cb = null
    }

    public register(cb: (v: string) => void) {
        this.cb = cb
    }

    public trigger() {
        if (this.timeout) {
            clearTimeout(this.timeout)
            this.timeout = null
        }

        if (!this.valueChanged) {
            return
        }

        this.valueChanged = false
        
        if (this.cb) {
            this.cb(this.value)  
        }

        this.value = ""
    }
}