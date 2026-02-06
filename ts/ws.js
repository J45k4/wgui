"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.connectWebsocket = void 0;
var message_sender_ts_1 = require("./message_sender.ts");
var connectWebsocket = function (args) {
    var ws;
    var sender = new message_sender_ts_1.MessageSender(function (msgs) {
        if (!ws) {
            return;
        }
        ws.send(JSON.stringify(msgs));
    });
    var createConnection = function () {
        var href = window.location.href;
        var url = new URL(href);
        var wsProtocol = url.protocol === "https:" ? "wss" : "ws";
        var wsUrl = "".concat(wsProtocol, "://").concat(url.host, "/ws");
        ws = new WebSocket(wsUrl);
        ws.onmessage = function (e) {
            var data = e.data.toString();
            var messages = JSON.parse(data);
            args.onMessage(sender, messages);
        };
        ws.onopen = function () {
            args.onOpen(sender);
        };
        ws.onclose = function () {
            setTimeout(function () {
                createConnection();
            }, 1000);
        };
        ws.onerror = function (e) {
            console.error("error", e);
        };
    };
    createConnection();
    return {
        close: function () {
            if (!ws) {
                return;
            }
            ws.close();
        },
        sender: sender
    };
};
exports.connectWebsocket = connectWebsocket;
