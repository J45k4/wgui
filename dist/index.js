// ts/debouncer.ts
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
var renderChildren = (element, items, ctx) => {
  for (const item of items) {
    const child = renderItem(item, ctx);
    if (child) {
      element.appendChild(child);
    }
  }
};
var renderPayload = (item, ctx, old) => {
  const payload = item.payload;
  if (payload.type === "checkbox") {
    let checkbox;
    if (old instanceof HTMLInputElement) {
      checkbox = old;
    } else {
      checkbox = document.createElement("input");
    }
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
    return checkbox;
  }
  if (payload.type === "layout") {
    let element;
    if (old instanceof HTMLDivElement) {
      element = old;
      old.innerHTML = "";
      for (const i of payload.body) {
        const el = renderItem(i, ctx);
        if (el) {
          old.appendChild(el);
        }
      }
    } else {
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
    return element;
  }
  if (payload.type === "select") {
    let select;
    if (old instanceof HTMLSelectElement) {
      select = old;
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
      select = document.createElement("select");
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
          inx: item.inx,
          value: select.value
        });
        ctx.sender.sendNow();
      };
    }
    return select;
  }
  if (payload.type === "button") {
    let button;
    if (old instanceof HTMLButtonElement) {
      button = old;
    } else {
      button = document.createElement("button");
    }
    button.textContent = payload.title;
    button.onclick = () => {
      ctx.sender.send({
        type: "onClick",
        id: item.id,
        inx: item.inx
      });
      ctx.sender.sendNow();
    };
    return button;
  }
  if (payload.type === "slider") {
    let slider;
    if (old instanceof HTMLInputElement) {
      slider = old;
    } else {
      slider = document.createElement("input");
    }
    slider.min = payload.min.toString();
    slider.max = payload.max.toString();
    slider.type = "range";
    slider.value = payload.value.toString();
    slider.step = payload.step.toString();
    slider.oninput = (e) => {
      ctx.sender.send({
        type: "onSliderChange",
        id: item.id,
        inx: item.inx,
        value: parseInt(e.target.value)
      });
      ctx.sender.sendNow();
    };
    return slider;
  }
  if (payload.type === "textInput") {
    let input;
    if (old instanceof HTMLInputElement) {
      input = old;
      console.log("old input");
    } else {
      input = document.createElement("input");
      input.oninput = (e) => {
        ctx.sender.send({
          type: "onTextChanged",
          id: item.id,
          inx: item.inx,
          value: e.target.value
        });
      };
    }
    input.placeholder = payload.placeholder;
    input.value = payload.value;
    return input;
  }
  if (payload.type === "table") {
    let table;
    if (old instanceof HTMLTableElement) {
      table = old;
    } else {
      table = document.createElement("table");
    }
    renderChildren(table, payload.items, ctx);
    return table;
  }
  if (payload.type === "thead") {
    let thead;
    if (old instanceof HTMLTableSectionElement) {
      thead = old;
    } else {
      thead = document.createElement("thead");
    }
    renderChildren(thead, payload.items, ctx);
    return thead;
  }
  if (payload.type === "tbody") {
    let tbody;
    if (old instanceof HTMLTableSectionElement) {
      tbody = old;
    } else {
      tbody = document.createElement("tbody");
    }
    renderChildren(tbody, payload.items, ctx);
    return tbody;
  }
  if (payload.type === "tr") {
    let tr;
    if (old instanceof HTMLTableRowElement) {
      tr = old;
    } else {
      tr = document.createElement("tr");
    }
    renderChildren(tr, payload.items, ctx);
    return tr;
  }
  if (payload.type === "th") {
    let th;
    if (old instanceof HTMLTableCellElement) {
      th = old;
    } else {
      th = document.createElement("th");
    }
    renderChildren(th, [payload.item], ctx);
    return th;
  }
  if (payload.type === "td") {
    let td;
    if (old instanceof HTMLTableCellElement) {
      td = old;
    } else {
      td = document.createElement("td");
    }
    renderChildren(td, [payload.item], ctx);
    return td;
  }
  if (payload.type === "text") {
    let element;
    if (old instanceof HTMLSpanElement) {
      element = old;
      element.innerText = payload.value + "";
    } else {
      element = document.createElement("span");
      element.innerText = payload.value + "";
    }
    if (item.id) {
      element.onclick = () => {
        ctx.sender.send({
          type: "onClick",
          id: item.id,
          inx: item.inx
        });
        ctx.sender.sendNow();
      };
    }
    return element;
  }
};
var renderItem = (item, ctx, old) => {
  const element = renderPayload(item, ctx, old);
  if (!element) {
    return;
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
  if (item.grow) {
    element.style.flexGrow = item.grow.toString();
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
    this.queue = this.queue.filter((m) => {
      if (m.type === msg.type) {
        return false;
      }
      return true;
    });
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
      console.error("error", e);
    };
  };
  createConnection();
  return {
    close: () => {
      if (!ws) {
        return;
      }
      ws.close();
    },
    sender
  };
};

// ts/app.ts
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
          renderItem(message.item, ctx, element);
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
