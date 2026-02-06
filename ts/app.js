"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
var debouncer_ts_1 = require("./debouncer.ts");
var path_ts_1 = require("./path.ts");
var render_ts_1 = require("./render.ts");
var three_host_ts_1 = require("./three_host.ts");
var ws_ts_1 = require("./ws.ts");
var getSetPropValue = function (value) {
    if (!value) {
        return undefined;
    }
    if (value.String != null) {
        return value.String;
    }
    if (value.Number != null) {
        return value.Number.toString();
    }
    return undefined;
};
var applySetProp = function (element, set) {
    var value = getSetPropValue(set.value);
    if (value == null) {
        return;
    }
    if (!(element instanceof HTMLElement)) {
        return;
    }
    switch (set.key) {
        case "BackgroundColor":
            element.style.backgroundColor = value;
            break;
        case "Border":
            element.style.border = value;
            break;
        case "Spacing": {
            var parsed = Number(value);
            element.style.gap = isNaN(parsed) ? value : "".concat(parsed, "px");
            break;
        }
        case "FlexDirection":
            element.style.display = "flex";
            element.style.flexDirection = value;
            break;
        case "Grow":
            element.style.flexGrow = value;
            break;
        case "Width":
            element.style.width = value === "0" ? "" : "".concat(value, "px");
            break;
        case "Height":
            element.style.height = value === "0" ? "" : "".concat(value, "px");
            break;
        case "MinWidth":
            element.style.minWidth = value === "0" ? "" : "".concat(value, "px");
            break;
        case "MaxWidth":
            element.style.maxWidth = value === "0" ? "" : "".concat(value, "px");
            break;
        case "MinHeight":
            element.style.minHeight = value === "0" ? "" : "".concat(value, "px");
            break;
        case "MaxHeight":
            element.style.maxHeight = value === "0" ? "" : "".concat(value, "px");
            break;
        case "Padding":
            element.style.padding = value === "0" ? "" : "".concat(value, "px");
            break;
        case "ID":
            element.id = value;
            break;
    }
};
window.onload = function () {
    var res = document.querySelector("body");
    if (!res) {
        return;
    }
    res.style.display = "flex";
    res.style.flexDirection = "row";
    res.style.height = "100vh";
    res.style.margin = "0";
    res.style.width = "100%";
    var root = res.querySelector("#wgui-root");
    if (!root) {
        res.innerHTML = "";
        root = document.createElement("div");
        root.id = "wgui-root";
        res.appendChild(root);
    }
    root.style.display = "flex";
    root.style.flexDirection = "column";
    root.style.flexGrow = "1";
    root.style.minHeight = "100vh";
    root.style.width = "100%";
    var debouncer = new debouncer_ts_1.Deboncer();
    var sender = (0, ws_ts_1.connectWebsocket)({
        onMessage: function (sender, msgs) {
            var _a;
            var ctx = {
                sender: sender,
                debouncer: debouncer
            };
            for (var _i = 0, msgs_1 = msgs; _i < msgs_1.length; _i++) {
                var message = msgs_1[_i];
                if (message.type === "pushState") {
                    history.pushState({}, "", message.url);
                    sender.send({
                        type: "pathChanged",
                        path: location.pathname,
                        query: {}
                    });
                    sender.sendNow();
                    continue;
                }
                if (message.type === "replaceState") {
                    history.replaceState({}, "", message.url);
                    continue;
                }
                if (message.type === "setQuery") {
                    var params = new URLSearchParams(location.search);
                    for (var _b = 0, _c = Object.keys(message.query); _b < _c.length; _b++) {
                        var key = _c[_b];
                        var value = message.query[key];
                        if (value != null) {
                            params.set(key, value);
                        }
                    }
                    history.replaceState({}, "", "".concat(params.toString()));
                    continue;
                }
                if (message.type === "setTitle") {
                    document.title = message.title;
                    continue;
                }
                if (message.type === "threePatch") {
                    var target = (0, path_ts_1.getPathItem)(message.path, root);
                    if (target) {
                        (0, three_host_ts_1.applyThreePatch)(target, message.ops);
                    }
                    continue;
                }
                if (message.type === "setProp") {
                    var target = (0, path_ts_1.getPathItem)(message.path, root);
                    if (!target) {
                        continue;
                    }
                    for (var _d = 0, _e = message.sets; _d < _e.length; _d++) {
                        var set = _e[_d];
                        applySetProp(target, set);
                    }
                    continue;
                }
                var element = (0, path_ts_1.getPathItem)(message.path, root);
                if (!element) {
                    continue;
                }
                if (message.type === "replace") {
                    (0, render_ts_1.renderItem)(message.item, ctx, element);
                }
                if (message.type === "replaceAt") {
                    (0, render_ts_1.renderItem)(message.item, ctx, element.children.item(message.inx));
                }
                if (message.type === "addFront") {
                    var newEl = (0, render_ts_1.renderItem)(message.item, ctx);
                    if (newEl) {
                        element.prepend(newEl);
                    }
                }
                if (message.type === "addBack") {
                    var newEl = (0, render_ts_1.renderItem)(message.item, ctx);
                    if (newEl) {
                        element.appendChild(newEl);
                    }
                }
                if (message.type === "insertAt") {
                    var newEl = (0, render_ts_1.renderItem)(message.item, ctx);
                    if (newEl) {
                        var child = element.children.item(message.inx);
                        child === null || child === void 0 ? void 0 : child.after(newEl);
                    }
                }
                if (message.type === "removeInx") {
                    (_a = element.children.item(message.inx)) === null || _a === void 0 ? void 0 : _a.remove();
                }
            }
        },
        onOpen: function (sender) {
            var params = new URLSearchParams(location.href);
            var query = {};
            params.forEach(function (value, key) {
                query[key] = value;
            });
            sender.send({
                type: "pathChanged",
                path: location.pathname,
                query: query
            });
            sender.sendNow();
        }
    }).sender;
    window.addEventListener("popstate", function (evet) {
        var params = new URLSearchParams(location.href);
        var query = {};
        params.forEach(function (value, key) {
            query[key] = value;
        });
        sender.send({
            type: "pathChanged",
            path: location.pathname,
            query: query,
        });
        sender.sendNow();
    });
};
