package fi.puppycorp.wgui.renderer

import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.FlowColumn
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.offset
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.BasicTextField
import androidx.compose.foundation.verticalScroll
import androidx.compose.foundation.border
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Slider
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.layout.ContentScale
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.Dp
import androidx.compose.ui.unit.dp
import coil.compose.AsyncImage
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import kotlin.OptIn

@Composable
fun RenderTree(
    state: RendererState,
    modifier: Modifier = Modifier,
    client: WguiClient?
) {
    val node = state.treeState.value
    Box(modifier = modifier.fillMaxSize()) {
        if (node != null) {
            RenderNode(node, client)
        }
    }
}

@Composable
private fun RenderNode(
    node: Node,
    client: WguiClient?,
    modifier: Modifier = Modifier
) {
    val item = node.item
    val payload = item.payload
    when (payload) {
        is Payload.Layout -> {
            RenderLayout(node, client, modifier)
        }
        is Payload.Text -> {
            val textAlign = item.textAlign?.let { parseTextAlign(it) } ?: TextAlign.Start
            val clickModifier = if (item.id != 0) {
                Modifier.clickable { stateClick(client, item) }
            } else Modifier
            Text(
                text = payload.value,
                textAlign = textAlign,
                modifier = applyModifiers(item, modifier).then(clickModifier)
            )
        }
        is Payload.TextInput -> {
            TextField(
                value = payload.value,
                onValueChange = { stateTextChanged(client, item, it) },
                placeholder = { Text(payload.placeholder) },
                modifier = applyModifiers(item, modifier)
            )
        }
        is Payload.Textarea -> {
            BasicTextField(
                value = payload.value,
                onValueChange = { stateTextChanged(client, item, it) },
                modifier = applyModifiers(item, modifier)
            )
        }
        is Payload.Button -> {
            Button(
                onClick = { stateClick(client, item) },
                modifier = applyModifiers(item, modifier)
            ) {
                Text(payload.title)
            }
        }
        is Payload.Checkbox -> {
            Checkbox(
                checked = payload.checked,
                onCheckedChange = { stateClick(client, item) },
                modifier = applyModifiers(item, modifier)
            )
        }
        is Payload.Slider -> {
            Slider(
                value = payload.value.toFloat(),
                onValueChange = { stateSliderChanged(client, item, it.toInt()) },
                valueRange = payload.min.toFloat()..payload.max.toFloat(),
                steps = (payload.max - payload.min - 1).coerceAtLeast(0),
                modifier = applyModifiers(item, modifier)
            )
        }
        is Payload.Select -> {
            SelectRenderer(item, payload, client, modifier)
        }
        is Payload.Image -> {
            AsyncImage(
                model = payload.src,
                contentDescription = payload.alt,
                contentScale = when (payload.objectFit) {
                    "cover" -> ContentScale.Crop
                    "contain" -> ContentScale.Fit
                    else -> ContentScale.Fit
                },
                modifier = applyModifiers(item, modifier)
            )
        }
        is Payload.Table -> {
            Column(modifier = applyModifiers(item, modifier)) {
                node.children.forEach { child ->
                    RenderNode(child, client)
                }
            }
        }
        is Payload.Thead -> {
            Column(modifier = applyModifiers(item, modifier)) {
                node.children.forEach { child ->
                    RenderNode(child, client)
                }
            }
        }
        is Payload.Tbody -> {
            Column(modifier = applyModifiers(item, modifier)) {
                node.children.forEach { child ->
                    RenderNode(child, client)
                }
            }
        }
        is Payload.Tr -> {
            Row(modifier = applyModifiers(item, modifier)) {
                node.children.forEach { child ->
                    RenderNode(child, client)
                }
            }
        }
        is Payload.Th -> {
            Column(modifier = applyModifiers(item, modifier)) {
                node.children.forEach { child ->
                    RenderNode(child, client)
                }
            }
        }
        is Payload.Td -> {
            Column(modifier = applyModifiers(item, modifier)) {
                node.children.forEach { child ->
                    RenderNode(child, client)
                }
            }
        }
        is Payload.Modal -> {
            if (payload.open) {
                Box(
                    modifier = Modifier
                        .fillMaxSize()
                        .background(Color(0x73000000))
                        .clickable { stateClick(client, item) },
                    contentAlignment = Alignment.Center
                ) {
                    Column(
                        modifier = applyModifiers(item, Modifier),
                        verticalArrangement = Arrangement.spacedBy(8.dp)
                    ) {
                        node.children.forEach { child ->
                            RenderNode(child, client)
                        }
                    }
                }
            }
        }
        is Payload.FloatingLayout -> {
            Box(
                modifier = applyModifiers(item, modifier)
                    .offset(payload.x.dp, payload.y.dp)
                    .width(payload.width.dp)
                    .height(payload.height.dp)
            ) {
                node.children.forEach { child ->
                    RenderNode(child, client)
                }
            }
        }
        Payload.FolderPicker -> {
            Button(
                onClick = { stateClick(client, item) },
                modifier = applyModifiers(item, modifier)
            ) {
                Text("Pick folder")
            }
        }
        Payload.None -> Spacer(modifier = applyModifiers(item, modifier).size(0.dp))
    }
}

@Composable
@OptIn(ExperimentalLayoutApi::class)
private fun RenderLayout(node: Node, client: WguiClient?, modifier: Modifier = Modifier) {
    val item = node.item
    val payload = item.payload as Payload.Layout
    val spacing = payload.spacing?.dp ?: 0.dp
    val overflow = item.overflow ?: ""
    val scrollState = rememberScrollState()
    val baseModifier = applyModifiers(item, modifier)
    val scrollModifier = when {
        overflow == "scroll" && payload.flex == "row" -> baseModifier.horizontalScroll(scrollState)
        overflow == "scroll" -> baseModifier.verticalScroll(scrollState)
        else -> baseModifier
    }

    val children = node.children
    val wrap = payload.wrap == true
    val flex = payload.flex ?: "column"

    if (flex == "row") {
        if (wrap) {
            FlowRow(
                horizontalArrangement = Arrangement.spacedBy(spacing),
                verticalArrangement = Arrangement.spacedBy(spacing),
                modifier = scrollModifier
            ) {
                children.forEach { child ->
                    RenderNode(child, client)
                }
            }
        } else {
            Row(
                horizontalArrangement = Arrangement.spacedBy(spacing),
                modifier = scrollModifier
            ) {
                children.forEach { child ->
                    RenderNodeInRow(child, client)
                }
            }
        }
    } else {
        if (wrap) {
            FlowColumn(
                verticalArrangement = Arrangement.spacedBy(spacing),
                horizontalArrangement = Arrangement.spacedBy(spacing),
                modifier = scrollModifier
            ) {
                children.forEach { child ->
                    RenderNode(child, client)
                }
            }
        } else {
            Column(
                verticalArrangement = Arrangement.spacedBy(spacing),
                modifier = scrollModifier
            ) {
                children.forEach { child ->
                    RenderNodeInColumn(child, client)
                }
            }
        }
    }
}

@Composable
private fun RowScope.RenderNodeInRow(node: Node, client: WguiClient?) {
    val base = applyModifiers(node.item, Modifier, inRow = true)
    val modifier = if (node.item.grow != null && node.item.grow > 0) {
        base.weight(node.item.grow.toFloat(), fill = true)
    } else {
        base
    }
    RenderNode(node, client, modifier)
}

@Composable
private fun ColumnScope.RenderNodeInColumn(node: Node, client: WguiClient?) {
    val base = applyModifiers(node.item, Modifier, inColumn = true)
    val modifier = if (node.item.grow != null && node.item.grow > 0) {
        base.weight(node.item.grow.toFloat(), fill = true)
    } else {
        base
    }
    RenderNode(node, client, modifier)
}

@Composable
private fun SelectRenderer(item: Item, payload: Payload.Select, client: WguiClient?, modifier: Modifier) {
    var expanded by remember(item.id) { mutableStateOf(false) }
    Box(modifier = applyModifiers(item, modifier)) {
        Button(onClick = { expanded = true }) {
            Text(payload.value)
        }
        DropdownMenu(expanded = expanded, onDismissRequest = { expanded = false }) {
            payload.options.forEach { option ->
                DropdownMenuItem(
                    text = { Text(option.name) },
                    onClick = {
                        expanded = false
                        stateSelectChanged(client, item, option.value)
                    }
                )
            }
        }
    }
}

private fun stateClick(client: WguiClient?, item: Item) {
    if (client == null) return
    if (item.id == 0) return
    client.sendOnClick(item.id, item.inx)
}

private fun stateTextChanged(client: WguiClient?, item: Item, value: String) {
    if (client == null) return
    if (item.id == 0) return
    client.sendOnTextChanged(item.id, item.inx, value)
}

private fun stateSliderChanged(client: WguiClient?, item: Item, value: Int) {
    if (client == null) return
    if (item.id == 0) return
    client.sendOnSliderChange(item.id, item.inx, value)
}

private fun stateSelectChanged(client: WguiClient?, item: Item, value: String) {
    if (client == null) return
    if (item.id == 0) return
    client.sendOnSelect(item.id, item.inx, value)
}

private fun parseTextAlign(value: String): TextAlign {
    return when (value.lowercase()) {
        "center" -> TextAlign.Center
        "right" -> TextAlign.End
        else -> TextAlign.Start
    }
}

private fun applyModifiers(item: Item, base: Modifier, inRow: Boolean = false, inColumn: Boolean = false): Modifier {
    var modifier = base
    val margin = item.margin?.dp ?: 0.dp
    if (margin > 0.dp) {
        modifier = modifier.padding(margin)
    }
    val padding = item.padding?.dp ?: 0.dp
    if (padding > 0.dp) {
        modifier = modifier.padding(padding)
    }
    item.marginLeft?.let { modifier = modifier.padding(start = it.dp) }
    item.marginRight?.let { modifier = modifier.padding(end = it.dp) }
    item.marginTop?.let { modifier = modifier.padding(top = it.dp) }
    item.marginBottom?.let { modifier = modifier.padding(bottom = it.dp) }
    item.paddingLeft?.let { modifier = modifier.padding(start = it.dp) }
    item.paddingRight?.let { modifier = modifier.padding(end = it.dp) }
    item.paddingTop?.let { modifier = modifier.padding(top = it.dp) }
    item.paddingBottom?.let { modifier = modifier.padding(bottom = it.dp) }
    if (item.width > 0) modifier = modifier.width(item.width.dp)
    if (item.height > 0) modifier = modifier.height(item.height.dp)
    if (item.minWidth > 0 || item.maxWidth > 0) {
        modifier = modifier.widthIn(
            min = if (item.minWidth > 0) item.minWidth.dp else Dp.Unspecified,
            max = if (item.maxWidth > 0) item.maxWidth.dp else Dp.Unspecified
        )
    }
    if (item.minHeight > 0 || item.maxHeight > 0) {
        modifier = modifier.heightIn(
            min = if (item.minHeight > 0) item.minHeight.dp else Dp.Unspecified,
            max = if (item.maxHeight > 0) item.maxHeight.dp else Dp.Unspecified
        )
    }
    // grow handled by RowScope/ColumnScope wrappers
    item.backgroundColor?.let { color ->
        parseColor(color)?.let { modifier = modifier.background(it) }
    }
    item.border?.let { border ->
        parseBorder(border)?.let { (width, color) ->
            modifier = modifier.border(width, color)
        }
    }
    return modifier
}

private fun parseBorder(border: String): Pair<Dp, Color>? {
    val parts = border.trim().split(" ")
    val widthPart = parts.firstOrNull() ?: return null
    val colorPart = parts.lastOrNull() ?: return null
    val width = widthPart.removeSuffix("px").toFloatOrNull() ?: return null
    val color = parseColor(colorPart) ?: return null
    return width.dp to color
}

private fun parseColor(value: String): Color? {
    val v = value.trim()
    if (v.startsWith("#")) {
        val hex = v.removePrefix("#")
        return when (hex.length) {
            3 -> {
                val r = hex[0].toString().repeat(2)
                val g = hex[1].toString().repeat(2)
                val b = hex[2].toString().repeat(2)
                Color(android.graphics.Color.parseColor("#$r$g$b"))
            }
            6, 8 -> Color(android.graphics.Color.parseColor("#$hex"))
            else -> null
        }
    }
    if (v.startsWith("rgb")) {
        val open = v.indexOf('(')
        val close = v.indexOf(')')
        if (open != -1 && close != -1) {
            val parts = v.substring(open + 1, close).split(",")
            val nums = parts.mapNotNull { it.trim().toFloatOrNull() }
            return if (nums.size >= 3) {
                val alpha = if (nums.size == 4) nums[3] else 1f
                Color(nums[0] / 255f, nums[1] / 255f, nums[2] / 255f, alpha)
            } else null
        }
    }
    return null
}
