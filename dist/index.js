// ts/logger.ts
var loglevel = 2 /* Info */;
var createLogger = (name) => {
  return {
    info: (...data) => {
      if (loglevel < 2 /* Info */) {
        return;
      }
      console.log(`[${name}]`, ...data);
    },
    error: (...data) => {
      if (loglevel < 4 /* Error */) {
        return;
      }
      console.error(`[${name}]`, ...data);
    },
    warn: (...data) => {
      if (loglevel < 3 /* Warn */) {
        return;
      }
      console.warn(`[${name}]`, ...data);
    },
    debug: (...data) => {
      if (loglevel < 1 /* Debug */) {
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
    this.valueChanged = true;
    this.value = text;
    clearTimeout(this.timeout);
    this.timeout = setTimeout(() => {
      this.trigger();
    }, 500);
  }
  unregister() {
    this.cb = null;
  }
  register(cb) {
    this.cb = cb;
  }
  trigger() {
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
  }
}

// ts/path.ts
var logger4 = createLogger("path");
var getPathItem = (path, element) => {
  const p = path[0];
  if (p == null) {
    return element;
  }
  const child = element.children[p];
  if (!child) {
    return;
  }
  return getPathItem(path.slice(1), child);
};

// ts/render.ts
var outerLogger = createLogger("render");
var renderItem = (item, ctx, old) => {
  console.log("renderItem", item, old);
  let element = old;
  const payload = item.payload;
  switch (payload.type) {
    case "checkbox": {
      if (old instanceof HTMLInputElement) {
        element = old;
        old.type = "checkbox";
        old.checked = payload.checked;
        element = old;
      } else {
        const checkbox = document.createElement("input");
        checkbox.type = "checkbox";
        checkbox.checked = payload.checked;
        checkbox.onclick = () => {
          ctx.sender.send({
            type: "onClick",
            id: item.id,
            inx: item.inx
          });
          ctx.sender.sendNow();
        };
        element = checkbox;
      }
      break;
    }
    case "layout": {
      if (old instanceof HTMLDivElement) {
        old.innerHTML = "";
        for (const i of payload.body) {
          const el = renderItem(i, ctx);
          if (el) {
            old.appendChild(el);
          }
        }
      } else {
        console.log("create layout", payload);
        const div = document.createElement("div");
        for (const i of payload.body) {
          const el = renderItem(i, ctx);
          if (el) {
            div.appendChild(el);
          }
        }
        element = div;
      }
      if (payload.spacing) {
        element.style.gap = payload.spacing + "px";
      }
      if (payload.wrap) {
        element.style.flexWrap = "wrap";
      }
      if (payload.flex) {
        element.style.display = "flex";
        element.style.flexDirection = payload.flex;
      }
      break;
    }
    case "select": {
      if (old instanceof HTMLSelectElement) {
        element = old;
        const existingOptions = Array.from(old.options);
        const newOptions = payload.options.map((option) => option.value);
        if (existingOptions.length !== payload.options.length || !existingOptions.every((opt, index) => opt.value === newOptions[index])) {
          old.innerHTML = "";
          for (const option of payload.options) {
            const opt = document.createElement("option");
            opt.value = option.value;
            opt.text = option.name;
            old.add(opt);
          }
        }
      } else {
        const select = document.createElement("select");
        for (const option of payload.options) {
          const opt = document.createElement("option");
          opt.value = option.value;
          opt.text = option.name;
          select.add(opt);
        }
        select.value = payload.value;
        select.onchange = () => {
          ctx.sender.send({
            type: "onSelect",
            id: item.id,
            value: select.value
          });
          ctx.sender.sendNow();
        };
        element = select;
      }
      break;
    }
    case "button": {
      if (old instanceof HTMLButtonElement) {
        element = old;
        old.textContent = payload.title;
        element = old;
      } else {
        const button = document.createElement("button");
        button.textContent = payload.title;
        button.onclick = () => {
          ctx.sender.send({
            type: "onClick",
            id: item.id,
            inx: item.inx
          });
          ctx.sender.sendNow();
        };
        element = button;
      }
      break;
    }
    case "slider": {
      if (old instanceof HTMLInputElement) {
        element = old;
        old.min = payload.min.toString();
        old.max = payload.max.toString();
        old.type = "range";
        old.value = payload.value.toString();
        old.step = payload.step.toString();
      }
      break;
    }
    case "text": {
      if (old instanceof HTMLSpanElement) {
        element = old;
        old.innerText = payload.value + "";
      } else {
        console.log("create text", payload);
        element = document.createElement("span");
        element.innerText = payload.value + "";
      }
      break;
    }
    case "textInput": {
      if (old instanceof HTMLInputElement) {
        console.log("it already exists");
        element = old;
        old.value = payload.value;
        old.placeholder = payload.placeholder;
      } else {
        const input = document.createElement("input");
        input.placeholder = payload.placeholder;
        input.value = payload.value;
        input.oninput = (e) => {
          ctx.sender.send({
            type: "onTextChanged",
            id: item.id,
            inx: item.inx,
            value: e.target.value
          });
        };
        element = input;
      }
      break;
    }
  }
  if (item.width) {
    element.style.width = item.width + "px";
  }
  if (item.height) {
    element.style.height = item.height + "px";
  }
  if (item.maxWidth) {
    element.style.maxWidth = item.maxWidth + "px";
  }
  if (item.maxHeight) {
    element.style.maxHeight = item.maxHeight + "px";
  }
  return element;
};

// ts/message_sender.ts
class MessageSender {
  sender;
  queue = [];
  timeout = 0;
  constructor(send) {
    this.sender = send;
  }
  send(msg) {
    this.queue.push(msg);
    this.sendNext();
  }
  sendNext() {
    if (this.timeout) {
      clearTimeout(this.timeout);
    }
    this.timeout = setTimeout(() => {
      this.sendNow();
    }, 500);
  }
  sendNow() {
    clearInterval(this.timeout);
    this.timeout = 0;
    if (this.queue.length === 0) {
      return;
    }
    this.sender(this.queue);
    this.queue = [];
  }
}

// ts/ws.ts
var logger7 = createLogger("ws");
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
      const messages = JSON.parse(data);
      args.onMessage(sender, messages);
    };
    ws.onopen = () => {
      args.onOpen(sender);
    };
    ws.onclose = () => {
      setTimeout(() => {
        createConnection();
      }, 1000);
    };
    ws.onerror = (e) => {
      logger7.error("error", e);
    };
  };
  createConnection();
  return {
    close: () => {
      logger7.debug("close");
      if (!ws) {
        return;
      }
      ws.close();
    },
    sender
  };
};

// ts/app.ts
var logger9 = createLogger("app");
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
  const debouncer2 = new Deboncer;
  const {
    sender
  } = connectWebsocket({
    onMessage: (sender2, msgs) => {
      const ctx = {
        sender: sender2,
        debouncer: debouncer2
      };
      for (const message of msgs) {
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
        if (!element) {
          continue;
        }
        if (message.type === "replace") {
          const newEl = renderItem(message.item, ctx, element);
          if (newEl) {
            element.replaceWith(newEl);
          }
        }
        if (message.type === "replaceAt") {
          const newEl = renderItem(message.item, ctx);
          if (newEl) {
            element.children.item(message.inx)?.replaceWith(newEl);
          }
        }
        if (message.type === "addFront") {
          const newEl = renderItem(message.item, ctx);
          if (newEl) {
            element.prepend(newEl);
          }
        }
        if (message.type === "addBack") {
          const newEl = renderItem(message.item, ctx);
          if (newEl) {
            element.appendChild(newEl);
          }
        }
        if (message.type === "insertAt") {
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
