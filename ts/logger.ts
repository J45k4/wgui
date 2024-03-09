
export enum LogLevel {
    Debug = 1,
    Info = 2,
    Warn = 3,
    Error = 4
}

let loglevel: LogLevel = LogLevel.Info

export const setLogLevel = (level: LogLevel) => {
    loglevel = level
}

export const createLogger = (name: string) => {
    return {
        info: (...data: any[]) => {
            if (loglevel < LogLevel.Info) {
                return
            }

            console.log(`[${name}]`, ...data)
        },
        error: (...data: any[]) => {
            if (loglevel < LogLevel.Error) {
                return
            }

            console.error(`[${name}]`, ...data)
        },
        warn: (...data: any[]) => {
            if (loglevel < LogLevel.Warn) {
                return
            }

            console.warn(`[${name}]`, ...data)
        },
        debug: (...data: any[]) => {
            if (loglevel < LogLevel.Debug) {
                return
            }

            console.debug(`[${name}]`, ...data)
        },
        child: (childName: string) => {
            return createLogger(`${name}:${childName}`)
        }
    }
}


export class UILogger {
    public root: HTMLDivElement

    constructor() {
        this.root = document.createElement("div")

        this.root.id = "logger"
        this.root.style.border = "1px solid black"
        this.root.style.minWidth = "200px"
        this.root.style.overflow = "auto"
    }

    public log(text: string, meta?: any) {
        const row = document.createElement("div")

        row.innerText = text

        this.root.prepend(row)
    }
}