"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.renderItem = void 0;
var three_host_ts_1 = require("./three_host.ts");
var renderChildren = function (element, items, ctx) {
    for (var _i = 0, items_1 = items; _i < items_1.length; _i++) {
        var item = items_1[_i];
        var child = (0, exports.renderItem)(item, ctx);
        if (child) {
            element.appendChild(child);
        }
    }
};
var renderPayload = function (item, ctx, old) {
    var _a, _b;
    var payload = item.payload;
    if (payload.type === "checkbox") {
        var checkbox = void 0;
        if (old instanceof HTMLInputElement) {
            checkbox = old;
        }
        else {
            checkbox = document.createElement("input");
            if (old)
                old.replaceWith(checkbox);
        }
        checkbox.type = "checkbox";
        checkbox.checked = payload.checked;
        if (item.id) {
            checkbox.onclick = function () {
                ctx.sender.send({
                    type: "onClick",
                    id: item.id,
                    inx: item.inx,
                });
                ctx.sender.sendNow();
            };
        }
        return checkbox;
    }
    if (payload.type === "layout") {
        var element_1;
        if (old instanceof HTMLDivElement) {
            element_1 = old;
            old.innerHTML = "";
            for (var _i = 0, _c = payload.body; _i < _c.length; _i++) {
                var i = _c[_i];
                var el = (0, exports.renderItem)(i, ctx);
                if (el) {
                    old.appendChild(el);
                }
            }
        }
        else {
            var div = document.createElement("div");
            for (var _d = 0, _e = payload.body; _d < _e.length; _d++) {
                var i = _e[_d];
                var el = (0, exports.renderItem)(i, ctx);
                if (el) {
                    div.appendChild(el);
                }
            }
            element_1 = div;
            if (old)
                old.replaceWith(element_1);
        }
        if (payload.spacing) {
            element_1.style.gap = payload.spacing + "px";
        }
        if (payload.wrap) {
            element_1.classList.add("flex-wrap");
        }
        if (payload.flex) {
            element_1.style.display = "flex";
            element_1.style.flexDirection = payload.flex;
            element_1.classList.add(payload.flex === "row" ? "flex-row" : "flex-col");
        }
        var horizontal = payload.horizontalResize || payload.horizontal_resize || payload.hresize;
        var vertical = payload.vresize;
        if (horizontal || vertical) {
            if (!element_1.style.overflow) {
                element_1.style.overflow = "auto";
            }
        }
        if (horizontal) {
            element_1.style.position = element_1.style.position || "relative";
            element_1.style.resize = "none";
            element_1.style.flexShrink = "0";
            var handle = element_1.querySelector(".wgui-resize-handle");
            if (!handle) {
                handle = document.createElement("div");
                handle.className = "wgui-resize-handle";
                element_1.appendChild(handle);
            }
            handle.style.position = "absolute";
            handle.style.top = "0";
            handle.style.right = "0";
            handle.style.bottom = "0";
            handle.style.width = "8px";
            handle.style.cursor = "col-resize";
            handle.style.zIndex = "2";
            handle.style.background = "transparent";
            handle.onmousedown = function (e) {
                e.preventDefault();
                var startX = e.clientX;
                var startWidth = element_1.getBoundingClientRect().width;
                var minWidth = item.minWidth || 0;
                var maxWidth = item.maxWidth || 0;
                var onMove = function (moveEvent) {
                    var next = startWidth + (moveEvent.clientX - startX);
                    var width = next;
                    if (minWidth && width < minWidth)
                        width = minWidth;
                    if (maxWidth && width > maxWidth)
                        width = maxWidth;
                    element_1.style.width = "".concat(width, "px");
                };
                var onUp = function () {
                    document.removeEventListener("mousemove", onMove);
                    document.removeEventListener("mouseup", onUp);
                    document.body.style.userSelect = "";
                    document.body.style.cursor = "";
                };
                document.body.style.userSelect = "none";
                document.body.style.cursor = "col-resize";
                document.addEventListener("mousemove", onMove);
                document.addEventListener("mouseup", onUp);
            };
        }
        return element_1;
    }
    if (payload.type === "select") {
        var select = void 0;
        if (old instanceof HTMLSelectElement) {
            select = old;
            // Use slice for broad compatibility instead of Array.from
            var existingOptions = Array.prototype.slice.call(old.options);
            var newOptions_1 = payload.options.map(function (option) { return option.value; });
            // Update the options only if they differ
            if (existingOptions.length !== payload.options.length || !existingOptions.every(function (opt, index) { return opt.value === newOptions_1[index]; })) {
                old.innerHTML = "";
                for (var _f = 0, _g = payload.options; _f < _g.length; _f++) {
                    var option = _g[_f];
                    var opt = document.createElement("option");
                    opt.value = option.value;
                    opt.text = option.name;
                    old.add(opt);
                }
            }
        }
        else {
            select = document.createElement("select");
            for (var _h = 0, _j = payload.options; _h < _j.length; _h++) {
                var option = _j[_h];
                var opt = document.createElement("option");
                opt.value = option.value;
                opt.text = option.name;
                select.add(opt);
            }
            select.value = payload.value;
            if (old)
                old.replaceWith(select);
        }
        select.oninput = function (e) {
            ctx.sender.send({
                type: "onSelect",
                id: item.id,
                inx: item.inx,
                value: e.target.value
            });
            ctx.sender.sendNow();
        };
        return select;
    }
    if (payload.type === "button") {
        var button = void 0;
        if (old instanceof HTMLButtonElement) {
            button = old;
        }
        else {
            button = document.createElement("button");
            if (old)
                old.replaceWith(button);
        }
        button.textContent = payload.title;
        if (item.id) {
            button.onclick = function () {
                ctx.sender.send({
                    type: "onClick",
                    id: item.id,
                    inx: item.inx,
                });
                ctx.sender.sendNow();
            };
        }
        return button;
    }
    if (payload.type === "img") {
        var image = void 0;
        if (old instanceof HTMLImageElement) {
            image = old;
        }
        else {
            image = document.createElement("img");
            if (old)
                old.replaceWith(image);
        }
        image.src = payload.src;
        image.alt = (_a = payload.alt) !== null && _a !== void 0 ? _a : "";
        image.style.maxWidth = "100%";
        image.style.maxHeight = "100%";
        image.style.objectFit = (_b = payload.objectFit) !== null && _b !== void 0 ? _b : "contain";
        image.loading = "lazy";
        return image;
    }
    if (payload.type === "slider") {
        var slider = void 0;
        if (old instanceof HTMLInputElement) {
            slider = old;
        }
        else {
            slider = document.createElement("input");
            if (old)
                old.replaceWith(slider);
        }
        slider.min = payload.min.toString();
        slider.max = payload.max.toString();
        slider.type = "range";
        slider.value = payload.value.toString();
        slider.step = payload.step.toString();
        if (item.id) {
            slider.oninput = function (e) {
                ctx.sender.send({
                    type: "onSliderChange",
                    id: item.id,
                    inx: item.inx,
                    value: parseInt(e.target.value)
                });
                ctx.sender.sendNow();
            };
        }
        return slider;
    }
    if (payload.type === "textInput") {
        var input = void 0;
        if (old instanceof HTMLInputElement) {
            input = old;
        }
        else {
            input = document.createElement("input");
            if (old)
                old.replaceWith(input);
        }
        input.placeholder = payload.placeholder;
        input.value = payload.value;
        if (item.id) {
            input.oninput = function (e) {
                ctx.sender.send({
                    type: "onTextChanged",
                    id: item.id,
                    inx: item.inx,
                    value: e.target.value,
                });
            };
        }
        return input;
    }
    if (payload.type === "textarea") {
        var textarea_1;
        if (old instanceof HTMLTextAreaElement) {
            textarea_1 = old;
        }
        else {
            textarea_1 = document.createElement("textarea");
            if (old)
                old.replaceWith(textarea_1);
        }
        textarea_1.placeholder = payload.placeholder;
        textarea_1.wrap = "off";
        textarea_1.style.resize = "none";
        textarea_1.style.overflowY = "hidden";
        textarea_1.style.minHeight = "20px";
        textarea_1.style.lineHeight = "20px";
        textarea_1.value = payload.value;
        var rowCount = payload.value.split("\n").length;
        textarea_1.style.height = rowCount * 20 + "px";
        textarea_1.oninput = function (e) {
            var value = e.target.value;
            var rowCount = value.split("\n").length;
            textarea_1.style.height = (rowCount + 1) * 20 + "px";
            if (item.id) {
                ctx.sender.send({
                    type: "onTextChanged",
                    id: item.id,
                    inx: item.inx,
                    value: e.target.value,
                });
            }
        };
        return textarea_1;
    }
    if (payload.type === "table") {
        var table = void 0;
        if (old instanceof HTMLTableElement) {
            table = old;
        }
        else {
            table = document.createElement("table");
            if (old)
                old.replaceWith(table);
        }
        renderChildren(table, payload.items, ctx);
        return table;
    }
    if (payload.type === "thead") {
        var thead = void 0;
        if (old instanceof HTMLTableSectionElement) {
            thead = old;
        }
        else {
            thead = document.createElement("thead");
            if (old)
                old.replaceWith(thead);
        }
        renderChildren(thead, payload.items, ctx);
        return thead;
    }
    if (payload.type === "tbody") {
        var tbody = void 0;
        if (old instanceof HTMLTableSectionElement) {
            tbody = old;
        }
        else {
            tbody = document.createElement("tbody");
            if (old)
                old.replaceWith(tbody);
        }
        renderChildren(tbody, payload.items, ctx);
        return tbody;
    }
    if (payload.type === "tr") {
        var tr = void 0;
        if (old instanceof HTMLTableRowElement) {
            tr = old;
        }
        else {
            tr = document.createElement("tr");
            if (old)
                old.replaceWith(tr);
        }
        renderChildren(tr, payload.items, ctx);
        return tr;
    }
    if (payload.type === "th") {
        var th = void 0;
        if (old instanceof HTMLTableCellElement) {
            th = old;
        }
        else {
            th = document.createElement("th");
            if (old)
                old.replaceWith(th);
        }
        renderChildren(th, [payload.item], ctx);
        return th;
    }
    if (payload.type === "td") {
        var td = void 0;
        if (old instanceof HTMLTableCellElement) {
            td = old;
        }
        else {
            td = document.createElement("td");
            if (old)
                old.replaceWith(td);
        }
        renderChildren(td, [payload.item], ctx);
        return td;
    }
    if (payload.type === "text") {
        var element = void 0;
        if (old instanceof HTMLSpanElement) {
            element = old;
            element.innerText = payload.value + "";
        }
        else {
            element = document.createElement("span");
            element.innerText = payload.value + "";
            if (old)
                old.replaceWith(element);
        }
        if (item.id) {
            element.onclick = function () {
                ctx.sender.send({
                    type: "onClick",
                    id: item.id,
                    inx: item.inx,
                });
                ctx.sender.sendNow();
            };
        }
        return element;
    }
    if (payload.type === "folderPicker") {
        var element = void 0;
        if (old instanceof HTMLInputElement) {
            element = old;
        }
        else {
            element = document.createElement("input");
            if (old)
                old.replaceWith(element);
        }
        element.type = "file";
        element.webkitdirectory = true;
        // element.multiple = true
        element.oninput = function (e) {
            console.log("oninput", e);
        };
        return element;
    }
    if (payload.type === "modal") {
        var overlay_1;
        if (old instanceof HTMLDivElement && old.dataset.modal === "overlay") {
            overlay_1 = old;
            overlay_1.innerHTML = "";
        }
        else {
            overlay_1 = document.createElement("div");
            overlay_1.dataset.modal = "overlay";
            overlay_1.setAttribute("role", "dialog");
            overlay_1.setAttribute("aria-modal", "true");
            if (old)
                old.replaceWith(overlay_1);
        }
        overlay_1.style.position = "fixed";
        overlay_1.style.left = "0";
        overlay_1.style.top = "0";
        overlay_1.style.width = "100vw";
        overlay_1.style.height = "100vh";
        overlay_1.style.display = payload.open ? "flex" : "none";
        overlay_1.style.alignItems = "center";
        overlay_1.style.justifyContent = "center";
        overlay_1.style.padding = "32px";
        overlay_1.style.boxSizing = "border-box";
        overlay_1.style.backgroundColor = "rgba(0, 0, 0, 0.45)";
        overlay_1.style.backdropFilter = "blur(2px)";
        overlay_1.style.zIndex = "1000";
        overlay_1.style.pointerEvents = payload.open ? "auto" : "none";
        overlay_1.setAttribute("aria-hidden", payload.open ? "false" : "true");
        renderChildren(overlay_1, payload.body, ctx);
        if (item.id) {
            overlay_1.onclick = function (event) {
                if (event.target === overlay_1) {
                    ctx.sender.send({
                        type: "onClick",
                        id: item.id,
                        inx: item.inx,
                    });
                    ctx.sender.sendNow();
                }
            };
        }
        else {
            overlay_1.onclick = null;
        }
        return overlay_1;
    }
    if (payload.type === "flaotingLayout") {
        var element = void 0;
        if (old instanceof HTMLDivElement) {
            element = old;
        }
        else {
            element = document.createElement("div");
            if (old)
                old.replaceWith(element);
        }
        element.style.position = "absolute";
        element.style.left = payload.x + "px";
        element.style.top = payload.y + "px";
        element.style.width = payload.width + "px";
        element.style.height = payload.height + "px";
        return element;
    }
    if (payload.type === "threeView") {
        var canvas = void 0;
        if (old instanceof HTMLCanvasElement) {
            canvas = old;
        }
        else {
            canvas = document.createElement("canvas");
            if (old)
                old.replaceWith(canvas);
        }
        canvas.dataset.wguiThree = "true";
        canvas.style.display = "block";
        canvas.style.width = "100%";
        canvas.style.height = "100%";
        (0, three_host_ts_1.applyThreeTree)(canvas, payload.root);
        return canvas;
    }
};
var renderItem = function (item, ctx, old) {
    if (old instanceof HTMLCanvasElement && item.payload.type !== "threeView") {
        (0, three_host_ts_1.disposeThreeHost)(old);
    }
    var element = renderPayload(item, ctx, old);
    if (!element) {
        return;
    }
    if (item.width) {
        element.style.width = item.width + "px";
    }
    if (item.height) {
        element.style.height = item.height + "px";
    }
    if (item.minWidth)
        element.style.minWidth = item.minWidth + "px";
    if (item.maxWidth) {
        element.style.maxWidth = item.maxWidth + "px";
    }
    if (item.minHeight)
        element.style.minHeight = item.minHeight + "px";
    if (item.maxHeight) {
        element.style.maxHeight = item.maxHeight + "px";
    }
    if (item.grow) {
        element.style.flexGrow = item.grow.toString();
        element.classList.add("grow");
    }
    if (item.backgroundColor) {
        element.style.backgroundColor = item.backgroundColor;
    }
    if (item.textAlign) {
        element.style.textAlign = item.textAlign;
    }
    if (item.cursor) {
        element.style.cursor = item.cursor;
    }
    if (item.margin) {
        element.style.margin = item.margin + "px";
    }
    if (item.marginLeft) {
        element.style.marginLeft = item.marginLeft + "px";
    }
    if (item.marginRight) {
        element.style.marginRight = item.marginRight + "px";
    }
    if (item.marginTop) {
        element.style.marginTop = item.marginTop + "px";
    }
    if (item.marginBottom) {
        element.style.marginBottom = item.marginBottom + "px";
    }
    if (item.padding) {
        element.style.padding = item.padding + "px";
    }
    if (item.paddingLeft) {
        element.style.paddingLeft = item.paddingLeft + "px";
    }
    if (item.paddingRight) {
        element.style.paddingRight = item.paddingRight + "px";
    }
    if (item.paddingTop) {
        element.style.paddingTop = item.paddingTop + "px";
    }
    if (item.paddingBottom) {
        element.style.paddingBottom = item.paddingBottom + "px";
    }
    if (item.border) {
        element.style.border = item.border;
    }
    if (item.editable) {
        element.contentEditable = "true";
    }
    if (item.overflow)
        element.style.overflow = item.overflow;
    return element;
};
exports.renderItem = renderItem;
