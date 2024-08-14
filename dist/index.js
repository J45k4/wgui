// ts/logger.ts
var LogLevel;
(function(LogLevel2) {
  LogLevel2[LogLevel2["Debug"] = 1] = "Debug";
  LogLevel2[LogLevel2["Info"] = 2] = "Info";
  LogLevel2[LogLevel2["Warn"] = 3] = "Warn";
  LogLevel2[LogLevel2["Error"] = 4] = "Error";
})(LogLevel || (LogLevel = {}));
var loglevel = LogLevel.Info;
var createLogger = (name) => {
  return {
    info: (...data) => {
      if (loglevel < LogLevel.Info) {
        return;
      }
      console.log(`[${name}]`, ...data);
    },
    error: (...data) => {
      if (loglevel < LogLevel.Error) {
        return;
      }
      console.error(`[${name}]`, ...data);
    },
    warn: (...data) => {
      if (loglevel < LogLevel.Warn) {
        return;
      }
      console.warn(`[${name}]`, ...data);
    },
    debug: (...data) => {
      if (loglevel < LogLevel.Debug) {
        return;
      }
      console.debug(`[${name}]`, ...data);
    },
    child: (childName) => {
      return createLogger(`${name}:${childName}`);
    }
  };
};

// ts/debouncer.ts
var logger2 = createLogger("debouncer");

class Deboncer {
  timeout;
  value = "";
  valueChanged = false;
  cb = null;
  change(text) {
    logger2.info("change", text);
    this.valueChanged = true;
    this.value = text;
    clearTimeout(this.timeout);
    this.timeout = setTimeout(() => {
      logger2.info("timeout");
      this.trigger();
    }, 500);
  }
  unregister() {
    logger2.info("unregister");
    this.cb = null;
  }
  register(cb) {
    logger2.info("register");
    this.cb = cb;
  }
  trigger() {
    logger2.info("trigger", this.value, this.valueChanged);
    if (this.timeout) {
      clearTimeout(this.timeout);
      this.timeout = null;
      logger2.info("timeout cleared");
    }
    if (!this.valueChanged) {
      logger2.info("value is not changed");
      return;
    }
    this.valueChanged = false;
    if (this.cb) {
      logger2.info("debouncer is triggered with", this.value);
      this.cb(this.value);
    }
    this.value = "";
  }
}

// ts/path.ts
var logger4 = createLogger("path");
var getPathItem = (path, element) => {
  logger4.info(`getPathItem`, { path, element });
  const p = path[0];
  logger4.info(`first path item: ${p}`);
  if (p == null) {
    logger4.info("returning element", element);
    return element;
  }
  const child = element.children[p];
  logger4.info("child", child);
  if (!child) {
    logger4.info(`child not found with path ${p}`);
    return;
  }
  logger4.info(`child found: ${p}`);
  return getPathItem(path.slice(1), child);
};

// ts/render.ts
var outerLogger = createLogger("render");
var renderItem = (item, ctx, old) => {
  outerLogger.debug("renderItem", item, old);
  switch (item.type) {
    case "text": {
      if (old instanceof HTMLSpanElement) {
        old.innerHTML = item.text;
        return;
      }
      const span = document.createElement("span");
      span.innerText = item.text;
      return span;
    }
    case "view": {
      outerLogger.debug("render view");
      let div = old;
      if (old instanceof HTMLDivElement) {
        div.innerHTML = "";
        for (let i = 0;i < item.body.length; i++) {
          const el = renderItem(item.body[i], ctx);
          if (el) {
            div.appendChild(el);
          }
        }
      } else {
        div = document.createElement("div");
        for (const i of item.body) {
          const el = renderItem(i, ctx);
          if (el) {
            div.appendChild(el);
          }
        }
      }
      if (item.width != null) {
        div.style.width = item.width + "px";
      }
      if (item.height != null) {
        div.style.height = item.height + "px";
      }
      if (item.margin != null) {
        outerLogger.debug("setMargin", item.margin + "px");
        div.style.margin = item.margin + "px";
      }
      if (item.marginTop != null) {
        div.style.marginTop = item.marginTop + "px";
      }
      if (item.marginRight != null) {
        div.style.marginRight = item.marginRight + "px";
      }
      if (item.marginBottom != null) {
        div.style.marginBottom = item.marginBottom + "px";
      }
      if (item.marginLeft != null) {
        div.style.marginLeft = item.marginLeft + "px";
      }
      if (item.paddingTop != null) {
        div.style.paddingTop = item.paddingTop + "px";
      }
      if (item.paddingRight != null) {
        div.style.paddingRight = item.paddingRight + "px";
      }
      if (item.paddingBottom != null) {
        div.style.paddingBottom = item.paddingBottom + "px";
      }
      if (item.paddingLeft != null) {
        div.style.paddingLeft = item.paddingLeft + "px";
      }
      if (item.padding != null) {
        div.style.padding = item.padding + "px";
      }
      if (item.spacing != null) {
        div.style.gap = item.spacing + "px";
      }
      if (item.border != null) {
        div.style.border = item.border;
      }
      div.style.overflow = "auto";
      if (item.flex) {
        div.style.display = "flex";
        const flex = item.flex;
        div.style.flexDirection = flex.flexDirection;
        if (flex.grow) {
          div.style.flexGrow = flex.grow.toString();
        }
      }
      return div;
    }
    case "button": {
      const logger6 = outerLogger.child(`button:${item.name}:${item.id}`);
      logger6.debug("render button");
      if (old instanceof HTMLButtonElement) {
        old.textContent = item.title;
        return;
      }
      const button = document.createElement("button");
      button.innerText = item.title;
      if (item.flex != null) {
        button.style.display = "flex";
        const flex = item.flex;
        button.style.flexDirection = flex.flexDirection;
        if (flex.grow) {
          button.style.flexGrow = flex.grow.toString();
        }
      }
      button.onclick = () => {
        logger6.debug("button clicked");
        ctx.sender.send({
          type: "onClick",
          id: item.id,
          name: item.name
        });
        ctx.sender.sendNow();
      };
      return button;
    }
    case "textInput": {
      const logger6 = outerLogger.child(`textInput:${item.name}:${item.id}`);
      logger6.debug(`render textInput`, item);
      let registered = false;
      if (old instanceof HTMLInputElement) {
        if (!registered || !ctx.debouncer.valueChanged) {
          old.value = item.value;
        }
        return;
      }
      const input = document.createElement("input");
      input.placeholder = item.placeholder;
      input.value = item.value;
      if (item.flex != null) {
        input.style.display = "flex";
        const flex = item.flex;
        input.style.flexDirection = flex.flexDirection;
        if (flex.grow) {
          input.style.flexGrow = flex.grow.toString();
        }
      }
      input.oninput = (e) => {
        logger6.debug(`oninput ${input.value}`);
        ctx.debouncer.change(e.target.value);
      };
      input.onkeydown = (e) => {
        logger6.debug(`keydown: ${e.key}`);
        if (e.key === "Enter") {
          ctx.debouncer.trigger();
          ctx.sender.send({
            type: "onKeyDown",
            id: item.id,
            name: item.name,
            keycode: e.key
          });
          ctx.sender.sendNow();
        }
      };
      input.onfocus = () => {
        logger6.debug("focus");
        ctx.debouncer.register((v) => {
          logger6.debug(`changed to ${v}`);
          ctx.sender.send({
            type: "onTextChanged",
            id: item.id,
            name: item.name,
            value: v
          });
          ctx.sender.sendNow();
        });
        registered = true;
      };
      input.onblur = () => {
        logger6.debug("blur");
        ctx.debouncer.trigger();
        ctx.debouncer.unregister();
        registered = false;
      };
      return input;
    }
    case "checkbox": {
      const logger6 = outerLogger.child(`checkbox:${item.name}:${item.id}`);
      logger6.debug("render checkbox");
      if (old instanceof HTMLInputElement) {
        old.checked = item.checked;
        return;
      }
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.checked = item.checked;
      checkbox.name = item.name;
      checkbox.onclick = () => {
        ctx.sender.send({
          type: "onClick",
          id: item.id,
          name: item.name
        });
        ctx.sender.sendNow();
      };
      return checkbox;
    }
    case "h1": {
      const logger6 = outerLogger.child(`h1:${item.text}`);
      logger6.debug("render h1");
      if (old instanceof HTMLHeadingElement) {
        old.innerText = item.text;
        return;
      }
      const h1 = document.createElement("h1");
      h1.innerText = item.text;
      return h1;
    }
    case "title": {
      document.title = item.title;
      return;
    }
    default:
      return document.createTextNode("Unknown item type");
  }
};

// ts/message_sender.ts
var logger7 = createLogger("message_sender");

class MessageSender {
  sender;
  queue = [];
  timeout = 0;
  constructor(send) {
    this.sender = send;
  }
  send(msg) {
    logger7.info("send", msg);
    this.queue.push(msg);
    this.sendNext();
  }
  sendNext() {
    logger7.info("sendNext");
    if (this.timeout) {
      logger7.info("timeout already exist");
      return;
    }
    this.timeout = setTimeout(() => {
      logger7.info("timeout");
      this.sendNow();
    }, 500);
  }
  sendNow() {
    logger7.info("sendNow");
    clearInterval(this.timeout);
    this.timeout = 0;
    if (this.queue.length === 0) {
      logger7.info("queue is empty");
      return;
    }
    logger7.info("sendingNow", this.queue);
    this.sender(this.queue);
    this.queue = [];
  }
}

// ts/ws.ts
var logger9 = createLogger("ws");
var connectWebsocket = (args) => {
  let ws;
  const sender = new MessageSender((msgs) => {
    if (!ws) {
      return;
    }
    ws.send(JSON.stringify(msgs));
  });
  const createConnection = () => {
    const href = window.location.href;
    const url = new URL(href);
    const wsProtocol = url.protocol === "https:" ? "wss" : "ws";
    const wsUrl = `${wsProtocol}://${url.host}/ws`;
    ws = new WebSocket(wsUrl);
    ws.onmessage = (e) => {
      const data = e.data.toString();
      logger9.info("rawdata", data);
      const messages = JSON.parse(data);
      logger9.info("received", messages);
      args.onMessage(sender, messages);
    };
    ws.onopen = () => {
      logger9.info("connected");
      args.onOpen(sender);
    };
    ws.onclose = () => {
      logger9.info("disconnected");
      setTimeout(() => {
        createConnection();
      }, 1000);
    };
    ws.onerror = (e) => {
      logger9.error("error", e);
    };
  };
  createConnection();
  return {
    close: () => {
      logger9.debug("close");
      if (!ws) {
        return;
      }
      ws.close();
    },
    sender
  };
};

// ts/app.ts
var logger11 = createLogger("app");
window.onload = () => {
  const res = document.querySelector("body");
  if (!res) {
    return;
  }
  res.innerHTML = "";
  res.style.display = "flex";
  res.style.flexDirection = "row";
  const content = document.createElement("div");
  content.style.flexGrow = "1";
  res.appendChild(content);
  const root = document.createElement("div");
  content.appendChild(root);
  logger11.debug("root", res);
  const debouncer2 = new Deboncer;
  const {
    sender
  } = connectWebsocket({
    onMessage: (sender2, msgs) => {
      logger11.info("root", root);
      const ctx = {
        sender: sender2,
        debouncer: debouncer2
      };
      for (const message of msgs) {
        logger11.info("process", message);
        if (message.type === "pushState") {
          history.pushState({}, "", message.url);
          sender2.send({
            type: "pathChanged",
            path: location.pathname,
            query: {}
          });
          sender2.sendNow();
          continue;
        }
        if (message.type === "replaceState") {
          history.replaceState({}, "", message.url);
          continue;
        }
        if (message.type === "setQuery") {
          const params = new URLSearchParams(location.search);
          for (const key of Object.keys(message.query)) {
            const value = message.query[key];
            if (value != null) {
              params.set(key, value);
            }
          }
          history.replaceState({}, "", `${params.toString()}`);
          continue;
        }
        const element = getPathItem(message.path, root);
        logger11.info("element", element);
        if (!element) {
          logger11.info(`cannot find element with path ${message.path}`);
          continue;
        }
        if (message.type === "replace") {
          logger11.info("replace", message);
          const newEl = renderItem(message.item, ctx, element);
          if (newEl) {
            element.replaceWith(newEl);
          }
        }
        if (message.type === "replaceAt") {
          logger11.info("replaceAt", message);
          const newEl = renderItem(message.item, ctx);
          if (newEl) {
            element.children.item(message.inx)?.replaceWith(newEl);
          }
        }
        if (message.type === "addFront") {
          logger11.info("addFront", message);
          const newEl = renderItem(message.item, ctx);
          if (newEl) {
            element.prepend(newEl);
          }
        }
        if (message.type === "addBack") {
          logger11.info("addBack", message);
          const newEl = renderItem(message.item, ctx);
          if (newEl) {
            element.appendChild(newEl);
          }
        }
        if (message.type === "insertAt") {
          logger11.info("insertAt", message);
          const newEl = renderItem(message.item, ctx);
          if (newEl) {
            const child = element.children.item(message.inx);
            child?.after(newEl);
          }
        }
        if (message.type === "removeInx") {
          element.children.item(message.inx)?.remove();
        }
      }
    },
    onOpen: (sender2) => {
      const params = new URLSearchParams(location.href);
      logger11.info("onOpen", params);
      const query = {};
      params.forEach((value, key) => {
        query[key] = value;
      });
      sender2.send({
        type: "pathChanged",
        path: location.pathname,
        query
      });
      sender2.sendNow();
    }
  });
  window.addEventListener("popstate", (evet) => {
    const params = new URLSearchParams(location.href);
    logger11.info("url changed", location.href);
    const query = {};
    params.forEach((value, key) => {
      query[key] = value;
    });
    sender.send({
      type: "pathChanged",
      path: location.pathname,
      query
    });
    sender.sendNow();
  });
};
