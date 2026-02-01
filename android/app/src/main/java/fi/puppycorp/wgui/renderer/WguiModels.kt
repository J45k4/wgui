package fi.puppycorp.wgui.renderer

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.JsonArray
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObject
import kotlinx.serialization.json.JsonPrimitive
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

private val json = Json { ignoreUnknownKeys = true }

sealed class Payload {
    data class Text(val value: String, val placeholder: String?) : Payload()
    data class TextInput(val value: String, val placeholder: String) : Payload()
    data class Textarea(val value: String, val placeholder: String) : Payload()
    data class Button(val title: String) : Payload()
    data class Checkbox(val checked: Boolean) : Payload()
    data class Slider(val min: Int, val max: Int, val value: Int, val step: Int) : Payload()
    data class Select(val value: String, val options: List<SelectOption>) : Payload()
    data class Layout(
        val body: List<Item>,
        val flex: String?,
        val spacing: Int?,
        val wrap: Boolean?,
        val horizontalResize: Boolean?,
        val verticalResize: Boolean?
    ) : Payload()
    data class Table(val items: List<Item>) : Payload()
    data class Thead(val items: List<Item>) : Payload()
    data class Tbody(val items: List<Item>) : Payload()
    data class Tr(val items: List<Item>) : Payload()
    data class Th(val item: Item) : Payload()
    data class Td(val item: Item) : Payload()
    data class Image(val src: String, val alt: String?, val objectFit: String?) : Payload()
    data class Modal(val open: Boolean, val body: List<Item>) : Payload()
    data class FloatingLayout(val x: Int, val y: Int, val width: Int, val height: Int) : Payload()
    object FolderPicker : Payload()
    object None : Payload()
}

data class SelectOption(
    val value: String,
    val name: String
)

data class Item(
    val id: Int,
    val inx: Int?,
    val height: Int,
    val width: Int,
    val minHeight: Int,
    val maxHeight: Int,
    val minWidth: Int,
    val maxWidth: Int,
    val grow: Int?,
    val backgroundColor: String?,
    val textAlign: String?,
    val cursor: String?,
    val margin: Int?,
    val padding: Int?,
    val border: String?,
    val marginLeft: Int?,
    val marginRight: Int?,
    val marginTop: Int?,
    val marginBottom: Int?,
    val paddingLeft: Int?,
    val paddingRight: Int?,
    val paddingTop: Int?,
    val paddingBottom: Int?,
    val editable: Boolean?,
    val overflow: String?,
    val payload: Payload
)

data class Node(
    val item: Item,
    val children: List<Node>
) {
    companion object {
        fun fromItem(item: Item, payloadChildren: List<Item>): Node {
            return Node(
                item = item,
                children = payloadChildren.map { fromItem(it, payloadChildrenFor(it)) }
            )
        }

        fun fromItem(item: Item): Node {
            return fromItem(item, payloadChildrenFor(item))
        }

        private fun payloadChildrenFor(item: Item): List<Item> {
            return when (val payload = item.payload) {
                is Payload.Layout -> payload.body
                is Payload.Table -> payload.items
                is Payload.Thead -> payload.items
                is Payload.Tbody -> payload.items
                is Payload.Tr -> payload.items
                is Payload.Th -> listOf(payload.item)
                is Payload.Td -> listOf(payload.item)
                is Payload.Modal -> payload.body
                else -> emptyList()
            }
        }
    }
}

sealed class SrvMessage {
    data class Replace(val path: List<Int>, val item: Item) : SrvMessage()
    data class ReplaceAt(val path: List<Int>, val inx: Int, val item: Item) : SrvMessage()
    data class AddBack(val path: List<Int>, val item: Item) : SrvMessage()
    data class AddFront(val path: List<Int>, val item: Item) : SrvMessage()
    data class InsertAt(val path: List<Int>, val inx: Int, val item: Item) : SrvMessage()
    data class RemoveInx(val path: List<Int>, val inx: Int) : SrvMessage()
    data class PushState(val url: String) : SrvMessage()
    data class ReplaceState(val url: String) : SrvMessage()
    data class SetQuery(val query: Map<String, String>) : SrvMessage()
    data class SetProp(val path: List<Int>, val sets: List<SetPropSet>) : SrvMessage()
    data class SetTitle(val title: String) : SrvMessage()
}

data class SetPropSet(
    val key: String,
    val value: PropValue?
)

data class PropValue(
    val string: String?,
    val number: Double?
)

fun parseSrvMessages(raw: String): List<SrvMessage> {
    val root = json.parseToJsonElement(raw)
    val arr = root as? JsonArray ?: return emptyList()
    return arr.mapNotNull { parseSrvMessage(it) }
}

private fun parseSrvMessage(element: JsonElement): SrvMessage? {
    val obj = element.jsonObject
    val type = obj["type"]?.jsonPrimitive?.content ?: return null
    return when (type) {
        "replace" -> SrvMessage.Replace(
            path = parsePath(obj["path"]),
            item = parseItem(obj["item"])
        )
        "replaceAt" -> SrvMessage.ReplaceAt(
            path = parsePath(obj["path"]),
            inx = obj["inx"]?.jsonPrimitive?.intOrNull() ?: 0,
            item = parseItem(obj["item"])
        )
        "addBack" -> SrvMessage.AddBack(
            path = parsePath(obj["path"]),
            item = parseItem(obj["item"])
        )
        "addFront" -> SrvMessage.AddFront(
            path = parsePath(obj["path"]),
            item = parseItem(obj["item"])
        )
        "insertAt" -> SrvMessage.InsertAt(
            path = parsePath(obj["path"]),
            inx = obj["inx"]?.jsonPrimitive?.intOrNull() ?: 0,
            item = parseItem(obj["item"])
        )
        "removeInx" -> SrvMessage.RemoveInx(
            path = parsePath(obj["path"]),
            inx = obj["inx"]?.jsonPrimitive?.intOrNull() ?: 0
        )
        "pushState" -> SrvMessage.PushState(obj["url"]?.jsonPrimitive?.content ?: "/")
        "replaceState" -> SrvMessage.ReplaceState(obj["url"]?.jsonPrimitive?.content ?: "/")
        "setQuery" -> SrvMessage.SetQuery(parseStringMap(obj["query"]))
        "setProp" -> SrvMessage.SetProp(
            path = parsePath(obj["path"]),
            sets = obj["sets"]?.jsonArray?.mapNotNull { parseSetProp(it) } ?: emptyList()
        )
        "setTitle" -> SrvMessage.SetTitle(obj["title"]?.jsonPrimitive?.content ?: "")
        else -> null
    }
}

private fun parseSetProp(element: JsonElement): SetPropSet? {
    val obj = element.jsonObject
    val key = obj["key"]?.jsonPrimitive?.content ?: return null
    val valueObj = obj["value"]?.jsonObject
    val value = if (valueObj == null) {
        null
    } else {
        PropValue(
            string = valueObj["String"]?.jsonPrimitive?.contentOrNull(),
            number = valueObj["Number"]?.jsonPrimitive?.doubleOrNull()
        )
    }
    return SetPropSet(key, value)
}

private fun parsePath(element: JsonElement?): List<Int> {
    if (element == null || element is JsonNull) return emptyList()
    return element.jsonArray.mapNotNull { it.jsonPrimitive.intOrNull() }
}

private fun parseItem(element: JsonElement?): Item {
    val obj = element?.jsonObject ?: JsonObject(emptyMap())
    val payloadObj = obj["payload"]?.jsonObject ?: JsonObject(emptyMap())
    val payloadType = payloadObj["type"]?.jsonPrimitive?.content ?: "none"

    val payload = when (payloadType) {
        "text" -> Payload.Text(
            value = payloadObj["value"]?.jsonPrimitive?.content ?: "",
            placeholder = payloadObj["placeholder"]?.jsonPrimitive?.contentOrNull()
        )
        "textInput" -> Payload.TextInput(
            value = payloadObj["value"]?.jsonPrimitive?.content ?: "",
            placeholder = payloadObj["placeholder"]?.jsonPrimitive?.content ?: ""
        )
        "textarea" -> Payload.Textarea(
            value = payloadObj["value"]?.jsonPrimitive?.content ?: "",
            placeholder = payloadObj["placeholder"]?.jsonPrimitive?.content ?: ""
        )
        "button" -> Payload.Button(
            title = payloadObj["title"]?.jsonPrimitive?.content ?: ""
        )
        "checkbox" -> Payload.Checkbox(
            checked = payloadObj["checked"]?.jsonPrimitive?.booleanOrNull() ?: false
        )
        "slider" -> Payload.Slider(
            min = payloadObj["min"]?.jsonPrimitive?.intOrNull() ?: 0,
            max = payloadObj["max"]?.jsonPrimitive?.intOrNull() ?: 0,
            value = payloadObj["value"]?.jsonPrimitive?.intOrNull() ?: 0,
            step = payloadObj["step"]?.jsonPrimitive?.intOrNull() ?: 1
        )
        "select" -> Payload.Select(
            value = payloadObj["value"]?.jsonPrimitive?.content ?: "",
            options = payloadObj["options"]?.jsonArray?.mapNotNull { option ->
                val opt = option.jsonObject
                val value = opt["value"]?.jsonPrimitive?.content ?: return@mapNotNull null
                val name = opt["name"]?.jsonPrimitive?.content ?: value
                SelectOption(value, name)
            } ?: emptyList()
        )
        "layout" -> Payload.Layout(
            body = payloadObj["body"]?.jsonArray?.map { parseItem(it) } ?: emptyList(),
            flex = payloadObj["flex"]?.jsonPrimitive?.contentOrNull(),
            spacing = payloadObj["spacing"]?.jsonPrimitive?.intOrNull(),
            wrap = payloadObj["wrap"]?.jsonPrimitive?.booleanOrNull(),
            horizontalResize = payloadObj["horizontalResize"]?.jsonPrimitive?.booleanOrNull()
                ?: payloadObj["horizontal_resize"]?.jsonPrimitive?.booleanOrNull()
                ?: payloadObj["hresize"]?.jsonPrimitive?.booleanOrNull(),
            verticalResize = payloadObj["vresize"]?.jsonPrimitive?.booleanOrNull()
        )
        "table" -> Payload.Table(
            items = payloadObj["items"]?.jsonArray?.map { parseItem(it) } ?: emptyList()
        )
        "thead" -> Payload.Thead(
            items = payloadObj["items"]?.jsonArray?.map { parseItem(it) } ?: emptyList()
        )
        "tbody" -> Payload.Tbody(
            items = payloadObj["items"]?.jsonArray?.map { parseItem(it) } ?: emptyList()
        )
        "tr" -> Payload.Tr(
            items = payloadObj["items"]?.jsonArray?.map { parseItem(it) } ?: emptyList()
        )
        "th" -> Payload.Th(
            item = payloadObj["item"]?.let { parseItem(it) } ?: parseItem(null)
        )
        "td" -> Payload.Td(
            item = payloadObj["item"]?.let { parseItem(it) } ?: parseItem(null)
        )
        "img" -> Payload.Image(
            src = payloadObj["src"]?.jsonPrimitive?.content ?: "",
            alt = payloadObj["alt"]?.jsonPrimitive?.contentOrNull(),
            objectFit = payloadObj["objectFit"]?.jsonPrimitive?.contentOrNull()
        )
        "modal" -> Payload.Modal(
            open = payloadObj["open"]?.jsonPrimitive?.booleanOrNull() ?: false,
            body = payloadObj["body"]?.jsonArray?.map { parseItem(it) } ?: emptyList()
        )
        "flaotingLayout" -> Payload.FloatingLayout(
            x = payloadObj["x"]?.jsonPrimitive?.intOrNull() ?: 0,
            y = payloadObj["y"]?.jsonPrimitive?.intOrNull() ?: 0,
            width = payloadObj["width"]?.jsonPrimitive?.intOrNull() ?: 0,
            height = payloadObj["height"]?.jsonPrimitive?.intOrNull() ?: 0
        )
        "folderPicker" -> Payload.FolderPicker
        else -> Payload.None
    }

    return Item(
        id = obj["id"]?.jsonPrimitive?.intOrNull() ?: 0,
        inx = obj["inx"]?.jsonPrimitive?.intOrNull(),
        height = obj["height"]?.jsonPrimitive?.intOrNull() ?: 0,
        width = obj["width"]?.jsonPrimitive?.intOrNull() ?: 0,
        minHeight = obj["minHeight"]?.jsonPrimitive?.intOrNull() ?: 0,
        maxHeight = obj["maxHeight"]?.jsonPrimitive?.intOrNull() ?: 0,
        minWidth = obj["minWidth"]?.jsonPrimitive?.intOrNull() ?: 0,
        maxWidth = obj["maxWidth"]?.jsonPrimitive?.intOrNull() ?: 0,
        grow = obj["grow"]?.jsonPrimitive?.intOrNull(),
        backgroundColor = obj["backgroundColor"]?.jsonPrimitive?.contentOrNull(),
        textAlign = obj["textAlign"]?.jsonPrimitive?.contentOrNull(),
        cursor = obj["cursor"]?.jsonPrimitive?.contentOrNull(),
        margin = obj["margin"]?.jsonPrimitive?.intOrNull(),
        padding = obj["padding"]?.jsonPrimitive?.intOrNull(),
        border = obj["border"]?.jsonPrimitive?.contentOrNull(),
        marginLeft = obj["marginLeft"]?.jsonPrimitive?.intOrNull(),
        marginRight = obj["marginRight"]?.jsonPrimitive?.intOrNull(),
        marginTop = obj["marginTop"]?.jsonPrimitive?.intOrNull(),
        marginBottom = obj["marginBottom"]?.jsonPrimitive?.intOrNull(),
        paddingLeft = obj["paddingLeft"]?.jsonPrimitive?.intOrNull(),
        paddingRight = obj["paddingRight"]?.jsonPrimitive?.intOrNull(),
        paddingTop = obj["paddingTop"]?.jsonPrimitive?.intOrNull(),
        paddingBottom = obj["paddingBottom"]?.jsonPrimitive?.intOrNull(),
        editable = obj["editable"]?.jsonPrimitive?.booleanOrNull(),
        overflow = obj["overflow"]?.jsonPrimitive?.contentOrNull(),
        payload = payload
    )
}

private fun parseStringMap(element: JsonElement?): Map<String, String> {
    val obj = element?.jsonObject ?: return emptyMap()
    return obj.mapNotNull { (key, value) ->
        val stringValue = value.jsonPrimitive.contentOrNull() ?: return@mapNotNull null
        key to stringValue
    }.toMap()
}

private fun JsonPrimitive.intOrNull(): Int? {
    return this.content.toIntOrNull()
}

private fun JsonPrimitive.doubleOrNull(): Double? {
    return this.content.toDoubleOrNull()
}

private fun JsonPrimitive.booleanOrNull(): Boolean? {
    return when (this.content.lowercase()) {
        "true" -> true
        "false" -> false
        else -> null
    }
}

private fun JsonPrimitive.contentOrNull(): String? {
    return if (this is JsonNull) null else this.content
}
