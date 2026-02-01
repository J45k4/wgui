package fi.puppycorp.wgui.renderer

import androidx.compose.runtime.mutableStateOf

class RendererState {
    private var rootNode: Node? = null
    val title = mutableStateOf("wgui")
    val currentPath = mutableStateOf("/")
    val currentQuery = mutableStateOf<Map<String, String>>(emptyMap())
    val treeState = mutableStateOf<Node?>(null)

    fun setTitle(next: String) {
        title.value = next
    }

    fun applyMessages(messages: List<SrvMessage>) {
        var root = rootNode
        for (msg in messages) {
            when (msg) {
                is SrvMessage.Replace -> {
                    root = replaceAtPath(root, msg.path, Node.fromItem(msg.item))
                }
                is SrvMessage.ReplaceAt -> {
                    root = replaceChildAt(root, msg.path, msg.inx, Node.fromItem(msg.item))
                }
                is SrvMessage.AddBack -> {
                    root = insertChild(root, msg.path, Int.MAX_VALUE, Node.fromItem(msg.item))
                }
                is SrvMessage.AddFront -> {
                    root = insertChild(root, msg.path, 0, Node.fromItem(msg.item))
                }
                is SrvMessage.InsertAt -> {
                    root = insertChild(root, msg.path, msg.inx + 1, Node.fromItem(msg.item))
                }
                is SrvMessage.RemoveInx -> {
                    root = removeChild(root, msg.path, msg.inx)
                }
                is SrvMessage.SetTitle -> {
                    setTitle(msg.title)
                }
                is SrvMessage.SetProp -> {
                    root = applySetProp(root, msg.path, msg.sets)
                }
                is SrvMessage.PushState -> {
                    currentPath.value = msg.url
                }
                is SrvMessage.ReplaceState -> {
                    currentPath.value = msg.url
                }
                is SrvMessage.SetQuery -> {
                    currentQuery.value = msg.query
                }
            }
        }
        rootNode = root
        treeState.value = root
    }

    private fun replaceAtPath(root: Node?, path: List<Int>, newNode: Node): Node? {
        if (path.isEmpty()) return newNode
        val current = root ?: return null
        val index = path.first()
        if (index !in current.children.indices) return current
        val updatedChild = replaceAtPath(current.children[index], path.drop(1), newNode) ?: return current
        val nextChildren = current.children.toMutableList()
        nextChildren[index] = updatedChild
        return current.copy(children = nextChildren)
    }

    private fun replaceChildAt(root: Node?, path: List<Int>, inx: Int, newNode: Node): Node? {
        val parent = nodeAtPath(root, path) ?: return root
        if (inx !in parent.children.indices) return root
        return replaceAtPath(root, path + inx, newNode)
    }

    private fun insertChild(root: Node?, path: List<Int>, index: Int, newNode: Node): Node? {
        if (root == null) return root
        if (path.isEmpty()) {
            return root
        }
        val parent = nodeAtPath(root, path) ?: return root
        val nextChildren = parent.children.toMutableList()
        val insertAt = index.coerceAtMost(nextChildren.size)
        nextChildren.add(insertAt, newNode)
        return replaceAtPath(root, path, parent.copy(children = nextChildren))
    }

    private fun removeChild(root: Node?, path: List<Int>, inx: Int): Node? {
        if (root == null) return root
        val parent = nodeAtPath(root, path) ?: return root
        if (inx !in parent.children.indices) return root
        val nextChildren = parent.children.toMutableList()
        nextChildren.removeAt(inx)
        return replaceAtPath(root, path, parent.copy(children = nextChildren))
    }

    private fun nodeAtPath(root: Node?, path: List<Int>): Node? {
        var current = root ?: return null
        for (idx in path) {
            if (idx !in current.children.indices) return null
            current = current.children[idx]
        }
        return current
    }

    private fun applySetProp(root: Node?, path: List<Int>, sets: List<SetPropSet>): Node? {
        val target = nodeAtPath(root, path) ?: return root
        var updatedItem = target.item
        for (set in sets) {
            val value = set.value
            when (set.key) {
                "ID" -> {
                    val id = value?.number?.toInt() ?: value?.string?.toIntOrNull()
                    if (id != null) updatedItem = updatedItem.copy(id = id)
                }
                "Border" -> {
                    updatedItem = updatedItem.copy(border = value?.string)
                }
                "BackgroundColor" -> {
                    updatedItem = updatedItem.copy(backgroundColor = value?.string)
                }
                "Spacing" -> {
                    if (updatedItem.payload is Payload.Layout) {
                        val spacing = value?.number?.toInt() ?: value?.string?.toIntOrNull()
                        val payload = updatedItem.payload as Payload.Layout
                        updatedItem = updatedItem.copy(payload = payload.copy(spacing = spacing))
                    }
                }
                "FlexDirection" -> {
                    if (updatedItem.payload is Payload.Layout) {
                        val payload = updatedItem.payload as Payload.Layout
                        updatedItem = updatedItem.copy(payload = payload.copy(flex = value?.string))
                    }
                }
                "Grow" -> {
                    val grow = value?.number?.toInt() ?: value?.string?.toIntOrNull()
                    updatedItem = updatedItem.copy(grow = grow)
                }
                "Width" -> {
                    val width = value?.number?.toInt() ?: value?.string?.toIntOrNull()
                    if (width != null) updatedItem = updatedItem.copy(width = width)
                }
                "Height" -> {
                    val height = value?.number?.toInt() ?: value?.string?.toIntOrNull()
                    if (height != null) updatedItem = updatedItem.copy(height = height)
                }
                "MinWidth" -> {
                    val minWidth = value?.number?.toInt() ?: value?.string?.toIntOrNull()
                    if (minWidth != null) updatedItem = updatedItem.copy(minWidth = minWidth)
                }
                "MaxWidth" -> {
                    val maxWidth = value?.number?.toInt() ?: value?.string?.toIntOrNull()
                    if (maxWidth != null) updatedItem = updatedItem.copy(maxWidth = maxWidth)
                }
                "MinHeight" -> {
                    val minHeight = value?.number?.toInt() ?: value?.string?.toIntOrNull()
                    if (minHeight != null) updatedItem = updatedItem.copy(minHeight = minHeight)
                }
                "MaxHeight" -> {
                    val maxHeight = value?.number?.toInt() ?: value?.string?.toIntOrNull()
                    if (maxHeight != null) updatedItem = updatedItem.copy(maxHeight = maxHeight)
                }
                "Padding" -> {
                    val padding = value?.number?.toInt() ?: value?.string?.toIntOrNull()
                    if (padding != null) updatedItem = updatedItem.copy(padding = padding)
                }
            }
        }
        val updatedNode = target.copy(item = updatedItem)
        return replaceAtPath(root, path, updatedNode)
    }
}
