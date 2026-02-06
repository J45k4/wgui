"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.MessageSender = void 0;
var MessageSender = /** @class */ (function () {
    function MessageSender(send) {
        this.queue = [];
        this.timeout = 0;
        this.sender = send;
    }
    MessageSender.prototype.send = function (msg) {
        this.queue = this.queue.filter(function (m) {
            if (m.type === msg.type) {
                return false;
            }
            return true;
        });
        this.queue.push(msg);
        this.sendNext();
    };
    MessageSender.prototype.sendNext = function () {
        var _this = this;
        if (this.timeout) {
            clearTimeout(this.timeout);
        }
        this.timeout = setTimeout(function () {
            _this.sendNow();
        }, 500);
    };
    MessageSender.prototype.sendNow = function () {
        clearInterval(this.timeout);
        this.timeout = 0;
        if (this.queue.length === 0) {
            return;
        }
        this.sender(this.queue);
        this.queue = [];
    };
    return MessageSender;
}());
exports.MessageSender = MessageSender;
