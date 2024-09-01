export const getPathItem = (path: number[], element: any): Element | undefined => {
    const p = path[0]
    if (p == null) {
        return element
    }
    const child = element.children[p]
    if (!child) {
        return
    }
    return getPathItem(path.slice(1), child)
}