import { createLogger } from "./logger.ts"

const logger = createLogger("path")

export const getPathItem = (path: number[], element: any): Element | undefined => {
    logger.info(`getPathItem`, { path, element })

    const p = path[0]

    logger.info(`first path item: ${p}`)

    if (p == null) {
        logger.info("returning element", element)

        return element
    }

    const child = element.children[p]

    logger.info("child", child)

    if (!child) {
        logger.info(`child not found with path ${p}`)

        return
    }

    logger.info(`child found: ${p}`)

    return getPathItem(path.slice(1), child)
}