"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.getPathItem = void 0;
var getPathItem = function (path, element) {
    var p = path[0];
    if (p == null) {
        return element;
    }
    var child = element.children[p];
    if (!child) {
        return;
    }
    return (0, exports.getPathItem)(path.slice(1), child);
};
exports.getPathItem = getPathItem;
