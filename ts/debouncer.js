"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.Deboncer = void 0;
var Deboncer = /** @class */ (function () {
    function Deboncer() {
        this.value = "";
        this.valueChanged = false;
        this.cb = null;
    }
    Deboncer.prototype.change = function (text) {
        var _this = this;
        this.valueChanged = true;
        this.value = text;
        clearTimeout(this.timeout);
        this.timeout = setTimeout(function () {
            _this.trigger();
        }, 500);
    };
    Deboncer.prototype.unregister = function () {
        this.cb = null;
    };
    Deboncer.prototype.register = function (cb) {
        this.cb = cb;
    };
    Deboncer.prototype.trigger = function () {
        if (this.timeout) {
            clearTimeout(this.timeout);
            this.timeout = null;
        }
        if (!this.valueChanged) {
            return;
        }
        this.valueChanged = false;
        if (this.cb) {
            this.cb(this.value);
        }
        this.value = "";
    };
    return Deboncer;
}());
exports.Deboncer = Deboncer;
