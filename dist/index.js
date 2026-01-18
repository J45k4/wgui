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
      if (old)
        old.replaceWith(checkbox);
    }
    checkbox.type = "checkbox";
    checkbox.checked = payload.checked;
    checkbox.classList.add("retro-checkbox");
    if (item.id) {
      checkbox.onclick = () => {
        ctx.sender.send({
          type: "onClick",
          id: item.id,
          inx: item.inx
        });
        ctx.sender.sendNow();
      };
    }
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
      if (old)
        old.replaceWith(element);
    }
    element.classList.add("retro-panel");
    if (payload.spacing) {
      element.style.gap = payload.spacing + "px";
    }
    if (payload.wrap) {
      element.classList.add("flex-wrap");
    }
    if (payload.flex) {
      element.style.display = "flex";
      element.style.flexDirection = payload.flex;
      element.classList.add(payload.flex === "row" ? "flex-row" : "flex-col");
    }
    return element;
  }
  if (payload.type === "select") {
    let select;
    if (old instanceof HTMLSelectElement) {
      select = old;
      const existingOptions = Array.prototype.slice.call(old.options);
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
      if (old)
        old.replaceWith(select);
    }
    select.classList.add("retro-input");
    select.oninput = (e) => {
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
    let button;
    if (old instanceof HTMLButtonElement) {
      button = old;
    } else {
      button = document.createElement("button");
      if (old)
        old.replaceWith(button);
    }
    button.textContent = payload.title;
    button.classList.add("retro-button");
    if (item.id) {
      button.onclick = () => {
        ctx.sender.send({
          type: "onClick",
          id: item.id,
          inx: item.inx
        });
        ctx.sender.sendNow();
      };
    }
    return button;
  }
  if (payload.type === "slider") {
    let slider;
    if (old instanceof HTMLInputElement) {
      slider = old;
    } else {
      slider = document.createElement("input");
      if (old)
        old.replaceWith(slider);
    }
    slider.min = payload.min.toString();
    slider.max = payload.max.toString();
    slider.type = "range";
    slider.value = payload.value.toString();
    slider.step = payload.step.toString();
    slider.classList.add("retro-input");
    if (item.id) {
      slider.oninput = (e) => {
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
    let input;
    if (old instanceof HTMLInputElement) {
      input = old;
    } else {
      input = document.createElement("input");
      if (old)
        old.replaceWith(input);
    }
    input.placeholder = payload.placeholder;
    input.value = payload.value;
    input.classList.add("retro-input");
    if (item.id) {
      input.oninput = (e) => {
        ctx.sender.send({
          type: "onTextChanged",
          id: item.id,
          inx: item.inx,
          value: e.target.value
        });
      };
    }
    return input;
  }
  if (payload.type === "textarea") {
    let textarea;
    if (old instanceof HTMLTextAreaElement) {
      textarea = old;
    } else {
      textarea = document.createElement("textarea");
      if (old)
        old.replaceWith(textarea);
    }
    textarea.placeholder = payload.placeholder;
    textarea.wrap = "off";
    textarea.style.resize = "none";
    textarea.style.overflowY = "hidden";
    textarea.style.minHeight = "20px";
    textarea.style.lineHeight = "20px";
    textarea.value = payload.value;
    const rowCount = payload.value.split(`
`).length;
    textarea.style.height = rowCount * 20 + "px";
    textarea.classList.add("retro-input");
    textarea.oninput = (e) => {
      const value = e.target.value;
      const rowCount2 = value.split(`
`).length;
      textarea.style.height = (rowCount2 + 1) * 20 + "px";
      if (item.id) {
        ctx.sender.send({
          type: "onTextChanged",
          id: item.id,
          inx: item.inx,
          value: e.target.value
        });
      }
    };
    return textarea;
  }
  if (payload.type === "table") {
    let table;
    if (old instanceof HTMLTableElement) {
      table = old;
    } else {
      table = document.createElement("table");
      if (old)
        old.replaceWith(table);
    }
    table.classList.add("retro-table");
    renderChildren(table, payload.items, ctx);
    return table;
  }
  if (payload.type === "thead") {
    let thead;
    if (old instanceof HTMLTableSectionElement) {
      thead = old;
    } else {
      thead = document.createElement("thead");
      if (old)
        old.replaceWith(thead);
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
      if (old)
        old.replaceWith(tbody);
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
      if (old)
        old.replaceWith(tr);
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
      if (old)
        old.replaceWith(th);
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
      if (old)
        old.replaceWith(td);
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
      if (old)
        old.replaceWith(element);
    }
    element.classList.add("retro-text");
    if (item.id) {
      element.onclick = () => {
        ctx.sender.send({
          type: "onClick",
          id: item.id,
          inx: item.inx
        });
        ctx.sender.sendNow();
      };
      element.classList.add("retro-clickable");
    }
    return element;
  }
  if (payload.type === "folderPicker") {
    let element;
    if (old instanceof HTMLInputElement) {
      element = old;
    } else {
      element = document.createElement("input");
      if (old)
        old.replaceWith(element);
    }
    element.type = "file";
    element.webkitdirectory = true;
    element.oninput = (e) => {
      console.log("oninput", e);
    };
    return element;
  }
  if (payload.type === "modal") {
    let overlay;
    if (old instanceof HTMLDivElement && old.dataset.modal === "overlay") {
      overlay = old;
      overlay.innerHTML = "";
    } else {
      overlay = document.createElement("div");
      overlay.dataset.modal = "overlay";
      overlay.setAttribute("role", "dialog");
      overlay.setAttribute("aria-modal", "true");
      if (old)
        old.replaceWith(overlay);
    }
    overlay.style.position = "fixed";
    overlay.style.left = "0";
    overlay.style.top = "0";
    overlay.style.width = "100vw";
    overlay.style.height = "100vh";
    overlay.style.display = payload.open ? "flex" : "none";
    overlay.style.alignItems = "center";
    overlay.style.justifyContent = "center";
    overlay.style.padding = "32px";
    overlay.style.boxSizing = "border-box";
    overlay.style.backgroundColor = "rgba(0, 0, 0, 0.45)";
    overlay.style.backdropFilter = "blur(2px)";
    overlay.style.zIndex = "1000";
    overlay.style.pointerEvents = payload.open ? "auto" : "none";
    overlay.setAttribute("aria-hidden", payload.open ? "false" : "true");
    renderChildren(overlay, payload.body, ctx);
    if (item.id) {
      overlay.onclick = (event) => {
        if (event.target === overlay) {
          ctx.sender.send({
            type: "onClick",
            id: item.id,
            inx: item.inx
          });
          ctx.sender.sendNow();
        }
      };
    } else {
      overlay.onclick = null;
    }
    return overlay;
  }
  if (payload.type === "flaotingLayout") {
    let element;
    if (old instanceof HTMLDivElement) {
      element = old;
    } else {
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

// ts/theme.ts
var retroCSS = `:root {
  --retro-bg: #1b1b24;
  --retro-panel: #222631;
  --retro-panel-alt: #2d3342;
  --retro-accent: #ffcc00;
  --retro-accent-hi: #ffe680;
  --retro-accent-lo: #b38f00;
  --retro-danger: #ff4d4d;
  --retro-success: #4dff88;
  --retro-fg: #f2f2f2;
  --retro-fg-dim: #c3c6cf;
  --retro-border: #4b5364;
  --retro-border-strong: #6d788f;
  --retro-font-stack: 'IBM Plex Mono', 'Courier New', monospace;
  --retro-radius: 2px;
  --retro-focus: 0 0 0 2px #ffcc00AA;
}

html, body {
  background: var(--retro-bg);
  color: var(--retro-fg);
  font-family: var(--retro-font-stack);
  font-size: 14px;
  line-height: 1.3;
  text-rendering: optimizeLegibility;
  -webkit-font-smoothing: none; /* embrace crispness */
}

body { margin:0; }

* { box-sizing: border-box; }

::-webkit-scrollbar { width: 12px; height: 12px; }
::-webkit-scrollbar-track { background: #15161d; }
::-webkit-scrollbar-thumb { background: var(--retro-panel-alt); border:2px solid #15161d; }
::-webkit-scrollbar-thumb:hover { background: var(--retro-accent); }

.retro-root { padding: 8px; display:flex; flex-direction:column; gap:8px; }

.retro-panel {
  background: linear-gradient(#272d39, #202530);
  border: 2px solid var(--retro-border);
  border-right-color: var(--retro-border-strong);
  border-bottom-color: var(--retro-border-strong);
  border-radius: var(--retro-radius);
  padding: 8px 10px;
  box-shadow: 0 0 0 1px #10141c, 0 2px 0 0 #12161f;
}

.retro-panel.inset {
  border-top-color: var(--retro-border-strong);
  border-left-color: var(--retro-border-strong);
  border-right-color: var(--retro-border);
  border-bottom-color: var(--retro-border);
  box-shadow: inset 0 0 0 1px #10141c, inset 0 2px 4px #111722;
}

.retro-text { font-size: 13px; letter-spacing: 0.5px; }
.retro-mono { font-family: var(--retro-font-stack); }

button, .retro-button {
  font: inherit;
  background: var(--retro-panel-alt);
  color: var(--retro-fg);
  border: 2px solid var(--retro-border);
  border-right-color: var(--retro-border-strong);
  border-bottom-color: var(--retro-border-strong);
  padding: 4px 12px;
  cursor: pointer;
  text-transform: uppercase;
  letter-spacing: 1px;
  position: relative;
  transition: background .12s, color .12s, transform .05s;
}
button:hover, .retro-button:hover { background: var(--retro-accent); color:#000; }
button:active, .retro-button:active { transform: translateY(1px); background: var(--retro-accent-lo); }
button:focus-visible, .retro-button:focus-visible { outline:none; box-shadow: var(--retro-focus); }

input[type=text], input[type=range], textarea, select, .retro-input {
  font: inherit;
  background: #161a22;
  color: var(--retro-fg);
  border: 2px solid var(--retro-border);
  padding: 4px 6px;
  border-radius: var(--retro-radius);
  outline: none;
}
input[type=text]:focus, textarea:focus, select:focus { box-shadow: var(--retro-focus); }

textarea { line-height: 1.2; }

input[type=checkbox] { width:16px; height:16px; cursor:pointer; }
input[type=checkbox] { accent-color: var(--retro-accent); }

select { cursor: pointer; }

.retro-slider-wrapper { display:flex; align-items:center; gap:6px; }
input[type=range] { width: 160px; }

/* Table styling */
.retro-table { border-collapse: collapse; width:100%; font-size:12px; }
.retro-table th, .retro-table td { border:1px solid var(--retro-border); padding:4px 6px; }
.retro-table thead th { background: var(--retro-panel-alt); text-align:left; }
.retro-table tbody tr:nth-child(even) { background:#1f232d; }
.retro-table tbody tr:hover { background: var(--retro-panel-alt); }

/* Modal overlay */
[data-modal=overlay] {
  font: inherit;
}
[data-modal=overlay] .retro-panel { min-width: 300px; }

/* Utility spacing */
.gap-xs { gap:4px; }
.gap-sm { gap:8px; }
.gap-md { gap:12px; }
.gap-lg { gap:16px; }

.flex-row { display:flex; flex-direction:row; }
.flex-col { display:flex; flex-direction:column; }
.flex-wrap { flex-wrap: wrap; }

.full-width { width:100%; }
.grow { flex-grow:1; }

/* Retro focus ring for clickable text spans */
.retro-clickable { cursor:pointer; }
.retro-clickable:focus-visible { outline:none; box-shadow: var(--retro-focus); }

`;
var injectRetroTheme = () => {
  if (document.getElementById("retro-theme"))
    return;
  const style = document.createElement("style");
  style.id = "retro-theme";
  style.textContent = retroCSS;
  document.head.appendChild(style);
};

// ts/app.ts
window.onload = () => {
  injectRetroTheme();
  const res = document.querySelector("body");
  if (!res) {
    return;
  }
  res.innerHTML = "";
  res.style.display = "flex";
  res.style.flexDirection = "row";
  res.classList.add("retro-root");
  const content = document.createElement("div");
  content.style.flexGrow = "1";
  res.appendChild(content);
  const root = document.createElement("div");
  content.appendChild(root);
  const debouncer = new Deboncer;
  const {
    sender
  } = connectWebsocket({
    onMessage: (sender2, msgs) => {
      const ctx = {
        sender: sender2,
        debouncer
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
          renderItem(message.item, ctx, element.children.item(message.inx));
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
