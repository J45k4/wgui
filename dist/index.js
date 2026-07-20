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

// ts/custom_components.ts
var modules = new Map;
var componentKey = (payload) => `${payload.name}
${payload.entry}`;
var loadModule = (entry) => {
  if (!modules.has(entry)) {
    const promise = import(entry).catch((err) => {
      if (modules.get(entry) === promise) {
        modules.delete(entry);
      }
      throw err;
    });
    modules.set(entry, promise);
  }
  return modules.get(entry);
};
var getState = (element) => element.__wguiCustomState;
var setState = (element, state) => {
  if (state) {
    element.__wguiCustomState = state;
  } else {
    delete element.__wguiCustomState;
  }
};
var controllerContext = (item, payload, ctx) => ({
  id: item.id,
  inx: item.inx ?? undefined,
  name: payload.name,
  emit: (name, eventPayload) => {
    const id = payload.events?.[name] ?? item.id;
    if (!id) {
      return;
    }
    ctx.sender.send({
      type: "onCustom",
      id,
      inx: item.inx ?? undefined,
      name,
      payload: eventPayload ?? null
    });
    ctx.sender.sendNow();
  }
});
var disposeState = (state) => {
  if (!state) {
    return;
  }
  state.cancelled = true;
  try {
    state.controller?.dispose?.();
  } catch (err) {
    console.warn("wgui custom component dispose failed", err);
  }
};
var dispatchData = (state, name, payload) => {
  if (!state.controller?.onData) {
    state.pendingData.push({ name, payload });
    return;
  }
  Promise.resolve(state.controller.onData(name, payload)).catch((err) => {
    console.warn("wgui custom component onData failed", err);
  });
};
var sendCustomData = (root, id, inx, name, payload) => {
  for (const element of Array.from(root.querySelectorAll("[data-wgui-custom='true']"))) {
    const state = getState(element);
    if (!state || state.id !== id) {
      continue;
    }
    if ((state.inx ?? undefined) !== (inx ?? undefined)) {
      continue;
    }
    dispatchData(state, name, payload);
    return;
  }
};
var disposeCustomComponent = (element) => {
  if (!(element instanceof HTMLElement)) {
    return;
  }
  disposeState(getState(element));
  setState(element, undefined);
};
var disposeCustomComponentTree = (element) => {
  if (!(element instanceof HTMLElement)) {
    return;
  }
  disposeCustomComponent(element);
  for (const child of Array.from(element.children)) {
    disposeCustomComponentTree(child);
  }
};
var mountCustomComponent = (element, item, payload, ctx) => {
  const key = componentKey(payload);
  const existing = getState(element);
  if (existing?.key === key) {
    existing.id = item.id;
    existing.inx = item.inx ?? undefined;
    existing.props = payload.props;
    if (existing.controller?.setProps) {
      Promise.resolve(existing.controller.setProps(payload.props)).catch((err) => {
        console.warn("wgui custom component setProps failed", err);
      });
    }
    return;
  }
  disposeState(existing);
  const state = {
    key,
    id: item.id,
    inx: item.inx ?? undefined,
    props: payload.props,
    pendingData: [],
    cancelled: false
  };
  setState(element, state);
  loadModule(payload.entry).then((module) => {
    if (state.cancelled) {
      return;
    }
    const Controller = module.default ?? module.Controller;
    if (!Controller) {
      throw new Error(`custom component ${payload.name} does not export a controller`);
    }
    const controller = new Controller(element, controllerContext(item, payload, ctx));
    state.controller = controller;
    return Promise.resolve(controller.mount?.(state.props)).then(() => {
      while (!state.cancelled && state.pendingData.length > 0) {
        const next = state.pendingData.shift();
        dispatchData(state, next.name, next.payload);
      }
      if (!state.cancelled && !controller.mount && controller.setProps) {
        return controller.setProps(state.props);
      }
    });
  }).catch((err) => {
    console.error(`wgui custom component ${payload.name} failed`, err);
    if (!state.cancelled && getState(element) === state) {
      element.textContent = `Failed to load component ${payload.name}`;
      setState(element, undefined);
    }
  });
};

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
var connectionStatusElements = () => document.querySelectorAll("[data-wgui-connection-status]");
var applyConnectionStatus = (element) => {
  const connected = document.documentElement.dataset.wguiSocketConnected === "true";
  const wantsConnected = element.dataset.wguiConnectionStatus === "connected";
  element.style.display = connected === wantsConnected ? element.dataset.wguiConnectionDisplay ?? "" : "none";
};
var setConnectionStatus = (connected) => {
  document.documentElement.dataset.wguiSocketConnected = connected ? "true" : "false";
  for (const element of connectionStatusElements()) {
    applyConnectionStatus(element);
  }
};
var reconcileChildren = (element, items, ctx) => {
  for (let i = 0;i < items.length; i++) {
    const child = renderItem(items[i], ctx, element.children.item(i));
    if (child && !child.parentElement) {
      element.appendChild(child);
    }
  }
  while (element.children.length > items.length) {
    const child = element.children.item(items.length);
    disposeCustomComponentTree(child);
    child?.remove();
  }
};
var clearModalState = (element, item) => {
  if (item.payload.type === "modal" || element.dataset.modal !== "overlay") {
    return;
  }
  delete element.dataset.modal;
  element.removeAttribute("role");
  element.removeAttribute("aria-modal");
  element.removeAttribute("aria-hidden");
  element.onclick = null;
  element.style.position = "";
  element.style.left = "";
  element.style.top = "";
  element.style.alignItems = "";
  element.style.justifyContent = "";
  element.style.backgroundColor = "";
  element.style.backdropFilter = "";
  element.style.zIndex = "";
  element.style.pointerEvents = "";
  element.style.overscrollBehavior = "";
  element.onwheel = null;
  element.ontouchmove = null;
};
var applyModalOverlayStyles = (overlay, open, fillsViewport, padding) => {
  overlay.style.position = "fixed";
  overlay.style.left = "0";
  overlay.style.top = "0";
  overlay.style.width = "100vw";
  overlay.style.height = "100vh";
  overlay.style.display = open ? "flex" : "none";
  overlay.style.alignItems = fillsViewport ? "stretch" : "center";
  overlay.style.justifyContent = "center";
  overlay.style.padding = `${padding}px`;
  overlay.style.boxSizing = "border-box";
  overlay.style.backgroundColor = "rgba(0, 0, 0, 0.45)";
  overlay.style.backdropFilter = "blur(2px)";
  overlay.style.zIndex = "1000";
  overlay.style.pointerEvents = open ? "auto" : "none";
  overlay.style.overscrollBehavior = "contain";
  overlay.setAttribute("aria-hidden", open ? "false" : "true");
};
var bindModalScrollBarrier = (overlay) => {
  overlay.onwheel = (event) => {
    event.stopPropagation();
    if (event.target === overlay) {
      event.preventDefault();
    }
  };
  overlay.ontouchmove = (event) => {
    event.stopPropagation();
    if (event.target === overlay) {
      event.preventDefault();
    }
  };
};
var fileToDataUrl = (file) => new Promise((resolve, reject) => {
  const reader = new FileReader;
  reader.onload = () => resolve(reader.result || "");
  reader.onerror = () => reject(reader.error);
  reader.readAsDataURL(file);
});
var setImageDropActive = (input, active) => {
  if (active) {
    input.style.outline = "2px dashed #2f7dd1";
    input.style.outlineOffset = "2px";
    input.style.backgroundColor = "rgba(47, 125, 209, 0.08)";
    return;
  }
  input.style.outline = "";
  input.style.outlineOffset = "";
  input.style.backgroundColor = "";
};
var hasFileDragPayload = (event) => {
  const dt = event.dataTransfer;
  if (!dt) {
    return false;
  }
  if (dt.files && dt.files.length > 0) {
    return true;
  }
  if (dt.items && dt.items.length > 0) {
    for (const item of dt.items) {
      if (item.kind === "file") {
        return true;
      }
    }
  }
  if (dt.types && dt.types.length > 0) {
    for (const t of dt.types) {
      if (t === "Files") {
        return true;
      }
    }
  }
  return false;
};
var buttonHoldStates = new WeakMap;
var buttonEventId = (item, events, name) => {
  if (events && typeof events[name] === "number") {
    return events[name];
  }
  if (!events && name === "click" && item.id) {
    return item.id;
  }
  return;
};
var sendButtonEvent = (type, id, item, ctx) => {
  if (!id) {
    return;
  }
  ctx.sender.send({
    type,
    id,
    inx: item.inx ?? undefined
  });
  ctx.sender.sendNow();
};
var stopButtonHold = (state, sendRelease) => {
  if (!state.active) {
    return;
  }
  state.active = false;
  state.activePointer = null;
  if (state.repeatTimer !== null) {
    window.clearInterval(state.repeatTimer);
    state.repeatTimer = null;
  }
  if (sendRelease) {
    const { item, events, ctx } = state.config;
    sendButtonEvent("onRelease", buttonEventId(item, events, "release"), item, ctx);
  }
};
var configureButtonEvents = (button, item, events, ctx) => {
  let state = buttonHoldStates.get(button);
  if (!state) {
    state = {
      active: false,
      activePointer: null,
      repeatTimer: null,
      config: { item, events, ctx }
    };
    buttonHoldStates.set(button, state);
    button.onclick = () => {
      const { item: item2, events: events2, ctx: ctx2 } = state.config;
      sendButtonEvent("onClick", buttonEventId(item2, events2, "click"), item2, ctx2);
    };
    button.onpointerdown = (event) => {
      const { item: item2, events: events2, ctx: ctx2 } = state.config;
      const pressId = buttonEventId(item2, events2, "press");
      const repeatId = buttonEventId(item2, events2, "repeat");
      if (!pressId && !repeatId) {
        return;
      }
      if (event.button !== undefined && event.button !== 0) {
        return;
      }
      event.preventDefault();
      if (state.active) {
        return;
      }
      state.active = true;
      state.activePointer = event.pointerId;
      button.setPointerCapture?.(event.pointerId);
      sendButtonEvent("onPress", pressId, item2, ctx2);
      if (repeatId) {
        const interval = Math.max(1, events2?.repeatInterval ?? 250);
        state.repeatTimer = window.setInterval(() => {
          const { item: item3, events: events3, ctx: ctx3 } = state.config;
          sendButtonEvent("onRepeat", buttonEventId(item3, events3, "repeat"), item3, ctx3);
        }, interval);
      }
    };
    button.onpointerup = (event) => {
      if (state.activePointer !== null && event.pointerId !== state.activePointer) {
        return;
      }
      event.preventDefault();
      stopButtonHold(state, true);
    };
    button.onpointercancel = () => stopButtonHold(state, true);
    button.onlostpointercapture = () => stopButtonHold(state, true);
    button.onblur = () => stopButtonHold(state, true);
  }
  state.config = { item, events, ctx };
};
var sendImageFileAsTextChanged = async (ctx, id, inx, file) => {
  const value = await fileToDataUrl(file).catch(() => "");
  if (!value) {
    return;
  }
  ctx.sender.send({
    type: "onTextChanged",
    id,
    inx,
    value
  });
  ctx.sender.sendNow();
};
var bindAutoClick = (element, item, ctx) => {
  const autoKey = "1";
  if (item.id) {
    if (!element.onclick) {
      element.dataset.wguiAutoClick = autoKey;
      element.onclick = () => {
        ctx.sender.send({
          type: "onClick",
          id: item.id,
          inx: item.inx
        });
        ctx.sender.sendNow();
      };
    }
    return;
  }
  if (element.dataset.wguiAutoClick === autoKey) {
    element.onclick = null;
    delete element.dataset.wguiAutoClick;
  }
};
var layoutScrollStates = new WeakMap;
var scrollNearBottomThreshold = 240;
var scrollNearBottomThrottleMs = 250;
var configureLayoutEvents = (element, item, payload, ctx) => {
  const id = payload.events?.scrollNearBottom;
  if (!id) {
    element.onscroll = null;
    layoutScrollStates.delete(element);
    return;
  }
  let state = layoutScrollStates.get(element);
  if (!state) {
    state = { nearBottom: false, lastSentAt: 0 };
    layoutScrollStates.set(element, state);
  } else {
    state.nearBottom = false;
  }
  element.onscroll = () => {
    const remaining = element.scrollHeight - element.scrollTop - element.clientHeight;
    const isNearBottom = remaining <= scrollNearBottomThreshold;
    if (!isNearBottom) {
      state.nearBottom = false;
      return;
    }
    if (state.nearBottom) {
      return;
    }
    const now = Date.now();
    if (now - state.lastSentAt < scrollNearBottomThrottleMs) {
      return;
    }
    state.nearBottom = true;
    state.lastSentAt = now;
    ctx.sender.send({
      type: "onScrollNearBottom",
      id,
      inx: item.inx ?? undefined
    });
    ctx.sender.sendNow();
  };
};
var bindSliderControlTracking = (slider) => {
  if (slider.dataset.wguiSliderTracking === "1") {
    return;
  }
  slider.dataset.wguiSliderTracking = "1";
  const begin = () => {
    slider.dataset.wguiSliderActive = "1";
  };
  const end = () => {
    delete slider.dataset.wguiSliderActive;
  };
  slider.addEventListener("pointerdown", begin);
  slider.addEventListener("pointerup", end);
  slider.addEventListener("pointercancel", end);
  slider.addEventListener("keydown", begin);
  slider.addEventListener("keyup", end);
  slider.addEventListener("blur", end);
};
var isSliderUserControlled = (slider) => slider.dataset.wguiSliderActive === "1" || document.activeElement === slider;
var textControlKey = (item) => `${item.id ?? ""}:${item.inx ?? ""}`;
var isTextControlUserControlled = (control) => document.activeElement === control;
var syncTextControlValue = (control, value, item) => {
  const key = textControlKey(item);
  const sameControl = control.dataset.wguiTextControlKey === key;
  control.dataset.wguiTextControlKey = key;
  if (sameControl && isTextControlUserControlled(control)) {
    return;
  }
  if (control.value !== value) {
    control.value = value;
  }
};
var pathQuery = (search) => {
  const params = new URLSearchParams(search);
  const query = {};
  params.forEach((value, key) => {
    query[key] = value;
  });
  return query;
};
var navigateLink = (event, anchor, ctx) => {
  if (event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey || anchor.target && anchor.target !== "_self" || anchor.hasAttribute("download")) {
    return;
  }
  const target = new URL(anchor.href, window.location.href);
  if (target.origin !== window.location.origin) {
    return;
  }
  event.preventDefault();
  const next = `${target.pathname}${target.search}${target.hash}`;
  if (next !== `${location.pathname}${location.search}${location.hash}`) {
    history.pushState({}, "", next);
  }
  ctx.sender.send({
    type: "pathChanged",
    path: location.pathname,
    query: pathQuery(location.search)
  });
  ctx.sender.sendNow();
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
      reconcileChildren(element, payload.body, ctx);
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
    if (payload.spacing) {
      element.style.gap = payload.spacing + "px";
    }
    if (payload.wrap) {
      element.style.flexWrap = "wrap";
      element.classList.add("flex-wrap");
    } else {
      element.style.flexWrap = "";
      element.classList.remove("flex-wrap");
    }
    if (payload.flex) {
      element.style.display = "flex";
      element.style.flexDirection = payload.flex;
      element.classList.add(payload.flex === "row" ? "flex-row" : "flex-col");
    }
    const horizontal = payload.horizontalResize || payload.horizontal_resize || payload.hresize;
    const vertical = payload.vresize;
    if (horizontal || vertical) {
      if (!element.style.overflow) {
        element.style.overflow = "auto";
      }
    }
    if (horizontal) {
      element.style.position = element.style.position || "relative";
      element.style.resize = "none";
      element.style.flexShrink = "0";
      let handle = Array.prototype.find.call(element.children, (child) => child instanceof HTMLDivElement && child.dataset.wguiResizeHandle === "true");
      if (!handle) {
        handle = document.createElement("div");
        handle.className = "wgui-resize-handle";
        handle.dataset.wguiResizeHandle = "true";
        element.appendChild(handle);
      }
      handle.style.position = "absolute";
      handle.style.top = "0";
      handle.style.right = "0";
      handle.style.bottom = "0";
      handle.style.width = "8px";
      handle.style.cursor = "col-resize";
      handle.style.zIndex = "2";
      handle.style.background = "transparent";
      handle.onmousedown = (e) => {
        e.preventDefault();
        const startX = e.clientX;
        const startWidth = element.getBoundingClientRect().width;
        const minWidth = item.minWidth || 0;
        const maxWidth = item.maxWidth || 0;
        const onMove = (moveEvent) => {
          const next = startWidth + (moveEvent.clientX - startX);
          let width = next;
          if (minWidth && width < minWidth)
            width = minWidth;
          if (maxWidth && width > maxWidth)
            width = maxWidth;
          element.dataset.wguiResizedWidth = `${width}`;
          element.style.width = `${width}px`;
          element.style.flexBasis = `${width}px`;
          element.style.setProperty("flex", `0 0 ${width}px`, "important");
        };
        const onUp = () => {
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
    configureLayoutEvents(element, item, payload, ctx);
    return element;
  }
  if (payload.type === "form") {
    let form;
    if (old instanceof HTMLFormElement) {
      form = old;
      reconcileChildren(form, payload.body, ctx);
    } else {
      form = document.createElement("form");
      if (old)
        old.replaceWith(form);
      renderChildren(form, payload.body, ctx);
    }
    form.action = payload.action || item.action || "";
    form.method = payload.method || item.method || "post";
    form.style.display = "flex";
    form.style.flexDirection = "column";
    if (payload.spacing) {
      form.style.gap = payload.spacing + "px";
    }
    return form;
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
      select.value = payload.value;
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
    configureButtonEvents(button, item, payload.events, ctx);
    return button;
  }
  if (payload.type === "link") {
    let anchor;
    if (old instanceof HTMLAnchorElement) {
      anchor = old;
    } else {
      anchor = document.createElement("a");
      if (old)
        old.replaceWith(anchor);
    }
    anchor.href = payload.href;
    anchor.textContent = payload.text;
    anchor.style.color = "inherit";
    anchor.style.textDecoration = "none";
    anchor.onclick = (event) => navigateLink(event, anchor, ctx);
    return anchor;
  }
  if (payload.type === "img") {
    let image;
    if (old instanceof HTMLImageElement) {
      image = old;
    } else {
      image = document.createElement("img");
      if (old)
        old.replaceWith(image);
    }
    image.src = payload.src;
    image.alt = payload.alt ?? "";
    image.style.maxWidth = "100%";
    image.style.maxHeight = "100%";
    image.style.objectFit = payload.objectFit ?? "contain";
    image.loading = "lazy";
    return image;
  }
  if (payload.type === "video") {
    let video;
    if (old instanceof HTMLVideoElement) {
      video = old;
    } else {
      video = document.createElement("video");
      if (old)
        old.replaceWith(video);
    }
    video.dataset.wguiRtc = "video";
    video.dataset.wguiRtcRoom = payload.room;
    video.dataset.wguiRtcLocal = payload.local ? "1" : "0";
    video.autoplay = payload.autoplay;
    video.muted = payload.muted;
    video.controls = payload.controls;
    video.playsInline = true;
    video.style.backgroundColor = "#000000";
    video.style.objectFit = "cover";
    return video;
  }
  if (payload.type === "audio") {
    let audio;
    if (old instanceof HTMLAudioElement) {
      audio = old;
    } else {
      audio = document.createElement("audio");
      if (old)
        old.replaceWith(audio);
    }
    audio.dataset.wguiRtc = "audio";
    audio.dataset.wguiRtcRoom = payload.room;
    audio.dataset.wguiRtcLocal = payload.local ? "1" : "0";
    audio.autoplay = payload.autoplay;
    audio.muted = payload.muted;
    audio.controls = payload.controls;
    return audio;
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
    slider.step = payload.step.toString();
    bindSliderControlTracking(slider);
    const sliderKey = `${item.id ?? ""}:${item.inx ?? ""}`;
    const sameSlider = slider.dataset.wguiSliderKey === sliderKey;
    slider.dataset.wguiSliderKey = sliderKey;
    if (!sameSlider || !isSliderUserControlled(slider)) {
      slider.value = payload.value.toString();
    }
    if (item.id) {
      let sliderFlushTimeout = 0;
      const flushSliderChange = () => {
        if (sliderFlushTimeout) {
          return;
        }
        sliderFlushTimeout = setTimeout(() => {
          sliderFlushTimeout = 0;
          ctx.sender.sendNow();
        }, 50);
      };
      const flushSliderChangeNow = () => {
        if (sliderFlushTimeout) {
          clearTimeout(sliderFlushTimeout);
          sliderFlushTimeout = 0;
        }
        ctx.sender.sendNow();
      };
      const sendSliderChange = (value) => {
        ctx.sender.send({
          type: "onSliderChange",
          id: item.id,
          inx: item.inx,
          value
        });
      };
      slider.oninput = (e) => {
        sendSliderChange(parseInt(e.target.value));
        flushSliderChange();
      };
      slider.onchange = (e) => {
        sendSliderChange(parseInt(e.target.value));
        flushSliderChangeNow();
      };
    }
    return slider;
  }
  if (payload.type === "datePicker") {
    let input;
    if (old instanceof HTMLInputElement) {
      input = old;
    } else {
      input = document.createElement("input");
      if (old)
        old.replaceWith(input);
    }
    input.type = "date";
    input.placeholder = payload.placeholder;
    syncTextControlValue(input, payload.value, item);
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
  if (payload.type === "textInput") {
    let input;
    if (old instanceof HTMLInputElement) {
      input = old;
    } else {
      input = document.createElement("input");
      if (old)
        old.replaceWith(input);
    }
    input.type = payload.inputType || payload.input_type || "text";
    input.placeholder = payload.placeholder;
    syncTextControlValue(input, payload.value, item);
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
    input.ondragover = (event) => {
      if (!hasFileDragPayload(event)) {
        setImageDropActive(input, false);
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      if (event.dataTransfer) {
        event.dataTransfer.dropEffect = "copy";
      }
      setImageDropActive(input, true);
    };
    input.ondragenter = (event) => {
      if (!hasFileDragPayload(event)) {
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      setImageDropActive(input, true);
    };
    input.ondragleave = () => {
      setImageDropActive(input, false);
    };
    input.ondrop = async (event) => {
      const dropped = event.dataTransfer?.files?.[0];
      if (!dropped || !dropped.type.startsWith("image/")) {
        setImageDropActive(input, false);
        return;
      }
      event.preventDefault();
      event.stopPropagation();
      setImageDropActive(input, false);
      const picker = document.querySelector('input[data-wgui-role="folder-picker"]');
      const pickerId = picker?.dataset.wguiId ? Number(picker.dataset.wguiId) : 0;
      if (!pickerId) {
        return;
      }
      await sendImageFileAsTextChanged(ctx, pickerId, undefined, dropped);
    };
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
    syncTextControlValue(textarea, payload.value, item);
    const rowCount = textarea.value.split(`
`).length;
    textarea.style.height = rowCount * 20 + "px";
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
      reconcileChildren(table, payload.items, ctx);
    } else {
      table = document.createElement("table");
      if (old)
        old.replaceWith(table);
      renderChildren(table, payload.items, ctx);
    }
    return table;
  }
  if (payload.type === "thead") {
    let thead;
    if (old instanceof HTMLTableSectionElement) {
      thead = old;
      reconcileChildren(thead, payload.items, ctx);
    } else {
      thead = document.createElement("thead");
      if (old)
        old.replaceWith(thead);
      renderChildren(thead, payload.items, ctx);
    }
    return thead;
  }
  if (payload.type === "tbody") {
    let tbody;
    if (old instanceof HTMLTableSectionElement) {
      tbody = old;
      reconcileChildren(tbody, payload.items, ctx);
    } else {
      tbody = document.createElement("tbody");
      if (old)
        old.replaceWith(tbody);
      renderChildren(tbody, payload.items, ctx);
    }
    return tbody;
  }
  if (payload.type === "tr") {
    let tr;
    if (old instanceof HTMLTableRowElement) {
      tr = old;
      reconcileChildren(tr, payload.items, ctx);
    } else {
      tr = document.createElement("tr");
      if (old)
        old.replaceWith(tr);
      renderChildren(tr, payload.items, ctx);
    }
    return tr;
  }
  if (payload.type === "th") {
    let th;
    if (old instanceof HTMLTableCellElement) {
      th = old;
      reconcileChildren(th, [payload.item], ctx);
    } else {
      th = document.createElement("th");
      if (old)
        old.replaceWith(th);
      renderChildren(th, [payload.item], ctx);
    }
    return th;
  }
  if (payload.type === "td") {
    let td;
    if (old instanceof HTMLTableCellElement) {
      td = old;
      reconcileChildren(td, [payload.item], ctx);
    } else {
      td = document.createElement("td");
      if (old)
        old.replaceWith(td);
      renderChildren(td, [payload.item], ctx);
    }
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
    element.webkitdirectory = false;
    element.multiple = false;
    element.accept = "image/*";
    element.dataset.wguiRole = "folder-picker";
    element.dataset.wguiId = item.id ? item.id.toString() : "";
    element.oninput = async (e) => {
      if (!item.id) {
        return;
      }
      const file = e?.target?.files?.[0];
      if (!file) {
        return;
      }
      await sendImageFileAsTextChanged(ctx, item.id, item.inx, file);
    };
    return element;
  }
  if (payload.type === "modal") {
    let overlay;
    if (old instanceof HTMLDivElement && old.dataset.modal === "overlay") {
      overlay = old;
    } else {
      overlay = document.createElement("div");
      overlay.dataset.modal = "overlay";
      overlay.setAttribute("role", "dialog");
      overlay.setAttribute("aria-modal", "true");
      if (old)
        old.replaceWith(overlay);
    }
    const fillsViewport = payload.body.some((child) => child.fill);
    applyModalOverlayStyles(overlay, payload.open, fillsViewport, item.padding || 32);
    if (old instanceof HTMLDivElement && old.dataset.modal === "overlay") {
      reconcileChildren(overlay, payload.body, ctx);
    } else {
      renderChildren(overlay, payload.body, ctx);
    }
    for (const [index, child] of Array.from(overlay.children).entries()) {
      if (child instanceof HTMLElement) {
        const fillsViewport2 = !!payload.body[index]?.fill;
        child.style.maxWidth = fillsViewport2 ? "none" : "calc(100vw - 64px)";
        child.style.maxHeight = fillsViewport2 ? "none" : "calc(100vh - 64px)";
        child.style.overscrollBehavior = "contain";
      }
    }
    bindModalScrollBarrier(overlay);
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
  if (payload.type === "connectionStatus") {
    let element;
    if (old instanceof HTMLDivElement && old.dataset.wguiConnectionStatus) {
      element = old;
      reconcileChildren(element, payload.body, ctx);
    } else {
      element = document.createElement("div");
      if (old)
        old.replaceWith(element);
      renderChildren(element, payload.body, ctx);
    }
    element.dataset.wguiConnectionStatus = payload.connected ? "connected" : "disconnected";
    element.dataset.wguiConnectionDisplay = "flex";
    element.style.flexDirection = payload.flex ?? "column";
    element.style.gap = payload.spacing ? `${payload.spacing}px` : "";
    element.style.flexWrap = payload.wrap ? "wrap" : "";
    applyConnectionStatus(element);
    return element;
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
  if (payload.type === "custom") {
    let element;
    if (old instanceof HTMLDivElement && old.dataset.wguiCustom === "true") {
      element = old;
    } else {
      disposeCustomComponentTree(old);
      element = document.createElement("div");
      if (old)
        old.replaceWith(element);
    }
    element.dataset.wguiCustom = "true";
    element.dataset.wguiCustomName = payload.name;
    mountCustomComponent(element, item, payload, ctx);
    return element;
  }
};
var renderItem = (item, ctx, old) => {
  if (old instanceof HTMLElement && old.dataset.wguiCustom === "true" && item.payload.type !== "custom") {
    disposeCustomComponentTree(old);
  }
  const element = renderPayload(item, ctx, old);
  if (!element) {
    return;
  }
  if (item.payload.type === "modal") {
    return element;
  }
  clearModalState(element, item);
  element.style.width = item.fill ? "100%" : item.width ? item.width + "px" : "";
  if (element instanceof HTMLElement && item.payload.type === "layout" && (item.payload.horizontalResize || item.payload.horizontal_resize || item.payload.hresize) && element.dataset.wguiResizedWidth) {
    element.style.width = `${element.dataset.wguiResizedWidth}px`;
    element.style.flexBasis = `${element.dataset.wguiResizedWidth}px`;
    element.style.setProperty("flex", `0 0 ${element.dataset.wguiResizedWidth}px`, "important");
  }
  element.style.boxSizing = item.fill ? "border-box" : "";
  element.style.height = item.height ? item.height + "px" : "";
  element.style.minWidth = item.minWidth !== undefined ? item.minWidth + "px" : "";
  element.style.maxWidth = item.maxWidth ? item.maxWidth + "px" : "";
  element.style.minHeight = item.minHeight ? item.minHeight + "px" : "";
  element.style.maxHeight = item.maxHeight ? item.maxHeight + "px" : "";
  element.style.flexGrow = item.grow ? item.grow.toString() : "";
  element.classList.toggle("grow", !!item.grow);
  element.style.backgroundColor = item.backgroundColor || "";
  element.style.color = item.color || "";
  if (item.breakWords) {
    element.style.overflowWrap = "anywhere";
    element.style.wordBreak = "break-word";
  } else {
    element.style.overflowWrap = "";
    element.style.wordBreak = "";
  }
  element.style.textAlign = item.textAlign || "";
  element.style.whiteSpace = item.whiteSpace || "";
  element.style.cursor = item.cursor || "";
  element.style.margin = "";
  element.style.marginLeft = "";
  element.style.marginRight = "";
  element.style.marginTop = "";
  element.style.marginBottom = "";
  if (item.margin)
    element.style.margin = item.margin + "px";
  if (item.marginLeft)
    element.style.marginLeft = item.marginLeft + "px";
  if (item.marginRight)
    element.style.marginRight = item.marginRight + "px";
  if (item.marginTop)
    element.style.marginTop = item.marginTop + "px";
  if (item.marginBottom)
    element.style.marginBottom = item.marginBottom + "px";
  element.style.padding = "";
  element.style.paddingLeft = "";
  element.style.paddingRight = "";
  element.style.paddingTop = "";
  element.style.paddingBottom = "";
  if (item.padding)
    element.style.padding = item.padding + "px";
  if (item.paddingLeft)
    element.style.paddingLeft = item.paddingLeft + "px";
  if (item.paddingRight)
    element.style.paddingRight = item.paddingRight + "px";
  if (item.paddingTop)
    element.style.paddingTop = item.paddingTop + "px";
  if (item.paddingBottom)
    element.style.paddingBottom = item.paddingBottom + "px";
  element.style.border = item.border || "";
  if (item.editable) {
    element.contentEditable = "true";
  } else {
    element.removeAttribute("contenteditable");
  }
  if (item.name) {
    element.setAttribute("name", item.name);
  } else {
    element.removeAttribute("name");
  }
  if (item.overflow) {
    element.style.overflow = item.overflow;
  } else {
    const isLayoutWithAutoOverflow = item.payload.type === "layout" && (item.payload.horizontalResize || item.payload.horizontal_resize || item.payload.hresize || item.payload.vresize);
    if (!isLayoutWithAutoOverflow) {
      element.style.overflow = "";
    }
  }
  if (!(element instanceof HTMLInputElement) && !(element instanceof HTMLSelectElement) && !(element instanceof HTMLTextAreaElement)) {
    bindAutoClick(element, item, ctx);
  }
  return element;
};

// ts/webrtc.ts
var ICE_SERVERS = [{ urls: ["stun:stun.l.google.com:19302"] }];

class WebRtcCoordinator {
  sender;
  rooms = new Map;
  constructor(sender) {
    this.sender = sender;
  }
  onSocketOpen() {
    for (const roomState of this.rooms.values()) {
      roomState.joined = false;
    }
  }
  syncElements(root) {
    const elementsByRoom = this.collectRoomElements(root);
    const desiredRooms = new Set(Object.keys(elementsByRoom));
    for (const [room, roomState] of this.rooms.entries()) {
      if (!desiredRooms.has(room)) {
        this.leaveRoom(room, roomState);
        this.rooms.delete(room);
      }
    }
    for (const room of desiredRooms) {
      const elements = elementsByRoom[room];
      let state = this.rooms.get(room);
      if (!state) {
        state = {
          joined: false,
          peers: [],
          participants: [],
          wantsLocalAudio: false,
          wantsLocalVideo: false,
          peerConnections: new Map,
          remoteStreams: new Map,
          pendingIceCandidates: new Map
        };
        this.rooms.set(room, state);
      }
      const wantsLocalVideo = elements.localVideo.length > 0;
      state.wantsLocalVideo = wantsLocalVideo;
      state.wantsLocalAudio = elements.localAudio.length > 0 || wantsLocalVideo;
      this.applyLocalPreview(state, elements);
      this.applyRemoteMedia(state, elements);
      if (!state.joined) {
        const displayName = this.detectDisplayName(root);
        this.sender.sendImmediate({
          type: "webRtcJoin",
          room,
          audio: state.wantsLocalAudio,
          video: state.wantsLocalVideo,
          displayName
        });
        state.joined = true;
      }
      if (state.joined) {
        this.ensureLocalMedia(room, state);
      }
    }
  }
  handleServerMessage(message) {
    if (message.type === "webRtcRoomState") {
      const roomState = this.rooms.get(message.room);
      if (!roomState) {
        return;
      }
      const raw = message;
      const selfClientId = typeof raw.selfClientId === "number" ? raw.selfClientId : typeof raw.self_client_id === "number" ? raw.self_client_id : roomState.selfClientId;
      const peers = Array.isArray(raw.peers) ? raw.peers.filter((peer) => typeof peer === "number") : [];
      const participants = Array.isArray(raw.participants) ? raw.participants.map((participant) => {
        const clientId = typeof participant?.clientId === "number" ? participant.clientId : typeof participant?.client_id === "number" ? participant.client_id : undefined;
        if (clientId == null) {
          return;
        }
        const displayNameRaw = participant?.displayName ?? participant?.display_name;
        const displayName = typeof displayNameRaw === "string" && displayNameRaw.trim().length > 0 ? displayNameRaw.trim() : `user ${clientId}`;
        return { clientId, displayName };
      }).filter((participant) => !!participant) : [];
      roomState.selfClientId = selfClientId;
      roomState.peers = peers;
      roomState.participants = participants.length > 0 ? participants : peers.map((peer) => ({
        clientId: peer,
        displayName: `user ${peer}`
      }));
      this.reconcilePeers(message.room, roomState);
      return;
    }
    if (message.type === "webRtcSignal") {
      const raw = message;
      const roomState = this.rooms.get(raw.room);
      if (!roomState) {
        return;
      }
      const fromClientId = typeof raw.fromClientId === "number" ? raw.fromClientId : typeof raw.from_client_id === "number" ? raw.from_client_id : undefined;
      if (typeof fromClientId !== "number") {
        return;
      }
      const payload = typeof raw.payload === "string" ? raw.payload : "";
      if (!payload) {
        return;
      }
      this.handleSignal(raw.room, roomState, fromClientId, payload);
    }
  }
  leaveRoom(room, roomState) {
    for (const pc of roomState.peerConnections.values()) {
      pc.close();
    }
    roomState.peerConnections.clear();
    roomState.remoteStreams.clear();
    roomState.pendingIceCandidates.clear();
    roomState.localStream?.getTracks().forEach((track) => track.stop());
    roomState.localStream = undefined;
    if (roomState.joined) {
      this.sender.sendImmediate({
        type: "webRtcLeave",
        room
      });
    }
  }
  collectRoomElements(root) {
    const out = {};
    const rtcEls = root.querySelectorAll("[data-wgui-rtc-room]");
    for (const el of rtcEls) {
      if (!(el instanceof HTMLMediaElement)) {
        continue;
      }
      const room = el.dataset.wguiRtcRoom || "";
      if (!room) {
        continue;
      }
      if (!out[room]) {
        out[room] = {
          localVideo: [],
          localAudio: [],
          remoteVideo: [],
          remoteAudio: []
        };
      }
      const isLocal = el.dataset.wguiRtcLocal === "1";
      const kind = el.dataset.wguiRtc;
      if (kind === "video") {
        if (isLocal && el instanceof HTMLVideoElement) {
          out[room].localVideo.push(el);
        } else if (el instanceof HTMLVideoElement) {
          out[room].remoteVideo.push(el);
        }
      }
      if (kind === "audio") {
        if (isLocal && el instanceof HTMLAudioElement) {
          out[room].localAudio.push(el);
        } else if (el instanceof HTMLAudioElement) {
          out[room].remoteAudio.push(el);
        }
      }
    }
    return out;
  }
  async ensureLocalMedia(room, roomState) {
    const wantsMedia = roomState.wantsLocalAudio || roomState.wantsLocalVideo;
    if (!wantsMedia || roomState.localStream) {
      return;
    }
    try {
      roomState.localStream = await navigator.mediaDevices.getUserMedia({
        audio: roomState.wantsLocalAudio,
        video: roomState.wantsLocalVideo
      });
      const peersNeedingRenegotiation = [];
      for (const [peerId, pc] of roomState.peerConnections.entries()) {
        this.addLocalTracks(pc, roomState.localStream);
        if (pc.signalingState === "stable" && pc.localDescription && pc.remoteDescription) {
          peersNeedingRenegotiation.push(peerId);
        }
      }
      for (const peerId of peersNeedingRenegotiation) {
        await this.createOffer(room, roomState, peerId);
      }
      this.syncElements(document.body);
    } catch (err) {
      console.error("failed to getUserMedia for room", room, err);
    }
  }
  reconcilePeers(room, roomState) {
    const activePeers = new Set(roomState.peers.filter((id) => id !== roomState.selfClientId));
    for (const [peerId, pc] of roomState.peerConnections.entries()) {
      if (activePeers.has(peerId)) {
        continue;
      }
      pc.close();
      roomState.peerConnections.delete(peerId);
      roomState.remoteStreams.delete(peerId);
      roomState.pendingIceCandidates.delete(peerId);
    }
    for (const peerId of activePeers) {
      const existing = roomState.peerConnections.get(peerId);
      if (existing) {
        continue;
      }
      const pc = this.createPeerConnection(room, roomState, peerId);
      roomState.peerConnections.set(peerId, pc);
      if ((roomState.selfClientId ?? 0) < peerId) {
        this.createOffer(room, roomState, peerId);
      }
    }
    this.syncElements(document.body);
  }
  createPeerConnection(room, roomState, peerId) {
    const pc = new RTCPeerConnection({ iceServers: ICE_SERVERS });
    if (roomState.localStream) {
      this.addLocalTracks(pc, roomState.localStream);
    }
    pc.onicecandidate = (event) => {
      if (!event.candidate) {
        return;
      }
      this.sendSignal(room, JSON.stringify({ kind: "ice", candidate: event.candidate }), peerId);
    };
    pc.ontrack = (event) => {
      const stream = event.streams[0] ?? this.ensurePeerRemoteStream(roomState, peerId);
      if (!event.streams[0]) {
        stream.addTrack(event.track);
      }
      roomState.remoteStreams.set(peerId, stream);
      this.syncElements(document.body);
    };
    pc.onconnectionstatechange = () => {
      if (pc.connectionState === "failed" || pc.connectionState === "closed") {
        roomState.remoteStreams.delete(peerId);
        roomState.pendingIceCandidates.delete(peerId);
        this.syncElements(document.body);
      }
    };
    return pc;
  }
  addLocalTracks(pc, stream) {
    for (const track of stream.getTracks()) {
      pc.addTrack(track, stream);
    }
  }
  async createOffer(room, roomState, peerId) {
    const pc = roomState.peerConnections.get(peerId);
    if (!pc) {
      return;
    }
    await this.ensureLocalMedia(room, roomState);
    if (!roomState.localStream) {
      this.ensureReceiveTransceivers(pc, roomState);
    }
    const offer = await pc.createOffer();
    await pc.setLocalDescription(offer);
    this.sendSignal(room, JSON.stringify({ kind: "offer", sdp: offer }), peerId);
  }
  async handleSignal(room, roomState, fromClientId, payload) {
    let signal;
    try {
      signal = JSON.parse(payload);
    } catch (_) {
      return;
    }
    let pc = roomState.peerConnections.get(fromClientId);
    if (!pc) {
      pc = this.createPeerConnection(room, roomState, fromClientId);
      roomState.peerConnections.set(fromClientId, pc);
    }
    if (signal.kind === "offer") {
      await this.ensureLocalMedia(room, roomState);
      if (!roomState.localStream) {
        this.ensureReceiveTransceivers(pc, roomState);
      }
      if (pc.signalingState !== "stable") {
        return;
      }
      await pc.setRemoteDescription(new RTCSessionDescription(signal.sdp));
      await this.flushPendingIce(roomState, fromClientId, pc);
      const answer = await pc.createAnswer();
      await pc.setLocalDescription(answer);
      this.sendSignal(room, JSON.stringify({ kind: "answer", sdp: answer }), fromClientId);
      return;
    }
    if (signal.kind === "answer") {
      if (pc.signalingState !== "have-local-offer" || !pc.localDescription) {
        return;
      }
      await pc.setRemoteDescription(new RTCSessionDescription(signal.sdp));
      await this.flushPendingIce(roomState, fromClientId, pc);
      return;
    }
    if (signal.kind === "ice" && signal.candidate) {
      if (!pc.remoteDescription) {
        const queued = roomState.pendingIceCandidates.get(fromClientId) ?? [];
        queued.push(signal.candidate);
        roomState.pendingIceCandidates.set(fromClientId, queued);
        return;
      }
      await pc.addIceCandidate(new RTCIceCandidate(signal.candidate));
    }
  }
  applyLocalPreview(roomState, elements) {
    for (const video of elements.localVideo) {
      video.srcObject = roomState.localStream ?? null;
      this.ensurePlayback(video);
    }
    for (const audio of elements.localAudio) {
      audio.srcObject = roomState.localStream ?? null;
      this.ensurePlayback(audio);
    }
  }
  applyRemoteMedia(roomState, elements) {
    const remotePeers = this.sortedRemotePeerIds(roomState);
    const videoSlots = this.ensureRemoteVideoSlots(elements, remotePeers.length);
    for (let i = 0;i < videoSlots.length; i += 1) {
      const peerId = remotePeers[i];
      const stream = peerId == null ? undefined : roomState.remoteStreams.get(peerId);
      const video = videoSlots[i];
      video.srcObject = stream ?? null;
      const label = peerId == null ? "Remote" : this.participantLabel(roomState, peerId);
      this.setVideoLabel(video, label);
      this.ensurePlayback(video);
    }
    const audioSlots = this.ensureRemoteAudioSlots(elements, remotePeers.length);
    for (let i = 0;i < audioSlots.length; i += 1) {
      const peerId = remotePeers[i];
      const stream = peerId == null ? undefined : roomState.remoteStreams.get(peerId);
      const audio = audioSlots[i];
      audio.srcObject = stream ?? null;
      this.ensurePlayback(audio);
    }
  }
  sortedRemotePeerIds(roomState) {
    const idsFromParticipants = roomState.participants.map((participant) => participant.clientId).filter((id) => id !== roomState.selfClientId);
    const ids = idsFromParticipants.length > 0 ? idsFromParticipants : roomState.peers.filter((id) => id !== roomState.selfClientId);
    return Array.from(new Set(ids)).sort((left, right) => left - right);
  }
  participantLabel(roomState, peerId) {
    return roomState.participants.find((participant) => participant.clientId === peerId)?.displayName || `user ${peerId}`;
  }
  detectDisplayName(root) {
    const rightAligned = Array.from(root.querySelectorAll("div,span,p")).find((element) => {
      const text = element.textContent?.trim() || "";
      return text.length > 0 && element.style.textAlign === "right";
    });
    if (rightAligned?.textContent) {
      return rightAligned.textContent.trim();
    }
    return "user";
  }
  sendSignal(room, payload, targetClientId) {
    const message = {
      type: "webRtcSignal",
      room,
      payload
    };
    if (targetClientId != null) {
      message.targetClientId = targetClientId;
    }
    this.sender.sendImmediate(message);
  }
  setVideoLabel(video, label) {
    const tile = video.parentElement;
    if (!tile) {
      return;
    }
    const labelNode = tile.firstElementChild;
    if (labelNode instanceof HTMLElement) {
      labelNode.textContent = label;
    }
  }
  ensureRemoteVideoSlots(elements, count) {
    const template = elements.remoteVideo[0];
    if (!template) {
      return [];
    }
    const tile = template.parentElement;
    const container = tile?.parentElement;
    if (!tile || !container) {
      return [template];
    }
    tile.dataset.wguiRtcTile = "1";
    tile.dataset.wguiRtcManaged = "template";
    const baseTile = tile;
    let tiles = Array.from(container.children).filter((child) => child instanceof HTMLElement && child.dataset.wguiRtcTile === "1");
    const needed = Math.max(count, 1);
    while (tiles.length < needed) {
      const clone = baseTile.cloneNode(true);
      clone.dataset.wguiRtcTile = "1";
      clone.dataset.wguiRtcManaged = "clone";
      const cloneVideo = clone.querySelector('video[data-wgui-rtc="video"]');
      if (cloneVideo) {
        cloneVideo.srcObject = null;
        cloneVideo.dataset.wguiRtcLocal = "0";
        cloneVideo.muted = true;
        cloneVideo.controls = false;
      }
      container.appendChild(clone);
      tiles.push(clone);
    }
    while (tiles.length > needed) {
      const tail = tiles.pop();
      if (!tail) {
        break;
      }
      if (tail.dataset.wguiRtcManaged === "template") {
        tiles.unshift(tail);
        break;
      }
      tail.remove();
    }
    tiles = Array.from(container.children).filter((child) => child instanceof HTMLElement && child.dataset.wguiRtcTile === "1");
    return tiles.slice(0, needed).map((slot) => slot.querySelector('video[data-wgui-rtc="video"]')).filter((video) => !!video);
  }
  ensureRemoteAudioSlots(elements, count) {
    const template = elements.remoteAudio[0];
    if (!template) {
      return [];
    }
    const parent = template.parentElement;
    if (!parent) {
      return [template];
    }
    template.dataset.wguiRtcManaged = "template";
    const needed = Math.max(count, 1);
    let slots = Array.from(parent.querySelectorAll('audio[data-wgui-rtc="audio"][data-wgui-rtc-local="0"]'));
    while (slots.length < needed) {
      const clone = template.cloneNode(true);
      clone.dataset.wguiRtcManaged = "clone";
      clone.controls = false;
      clone.style.display = "none";
      clone.srcObject = null;
      parent.appendChild(clone);
      slots.push(clone);
    }
    while (slots.length > needed) {
      const tail = slots.pop();
      if (!tail) {
        break;
      }
      if (tail.dataset.wguiRtcManaged === "template") {
        slots.unshift(tail);
        break;
      }
      tail.remove();
    }
    slots = Array.from(parent.querySelectorAll('audio[data-wgui-rtc="audio"][data-wgui-rtc-local="0"]'));
    return slots.slice(0, needed);
  }
  ensurePlayback(element) {
    if (!element.autoplay || !element.srcObject) {
      return;
    }
    const promise = element.play();
    if (promise && typeof promise.catch === "function") {
      promise.catch(() => {});
    }
  }
  ensureReceiveTransceivers(pc, roomState) {
    const hasKind = (kind) => pc.getTransceivers().some((transceiver) => transceiver.receiver.track?.kind === kind || transceiver.sender.track?.kind === kind);
    if (roomState.wantsLocalAudio && !hasKind("audio")) {
      pc.addTransceiver("audio", { direction: "recvonly" });
    }
    if (roomState.wantsLocalVideo && !hasKind("video")) {
      pc.addTransceiver("video", { direction: "recvonly" });
    }
  }
  ensurePeerRemoteStream(roomState, peerId) {
    const existing = roomState.remoteStreams.get(peerId);
    if (existing) {
      return existing;
    }
    const stream = new MediaStream;
    roomState.remoteStreams.set(peerId, stream);
    return stream;
  }
  async flushPendingIce(roomState, peerId, pc) {
    const queued = roomState.pendingIceCandidates.get(peerId);
    if (!queued || queued.length === 0 || !pc.remoteDescription) {
      return;
    }
    roomState.pendingIceCandidates.delete(peerId);
    for (const candidate of queued) {
      try {
        await pc.addIceCandidate(new RTCIceCandidate(candidate));
      } catch (err) {
        console.warn("failed queued ICE candidate", err);
      }
    }
  }
}

// ts/web_push.ts
var decodeBase64Url = (value) => {
  const padding = "=".repeat((4 - (value.length % 4 || 4)) % 4);
  const base64 = (value + padding).replace(/-/g, "+").replace(/_/g, "/");
  const bytes = atob(base64);
  const out = new Uint8Array(bytes.length);
  for (let i = 0;i < bytes.length; i += 1) {
    out[i] = bytes.charCodeAt(i);
  }
  return out;
};
var webPushSupported = () => ("Notification" in window) && ("serviceWorker" in navigator) && ("PushManager" in window);
var pushSubscriptionToServer = (sender, subscription) => {
  const payload = subscription ? subscription.toJSON() : null;
  sender.send({
    type: "webPushSubscriptionChanged",
    subscription: payload
  });
  sender.sendNow();
};
var normalizeVapidPublicKey = (value) => {
  if (!value) {
    return;
  }
  const trimmed = value.trim();
  if (trimmed.length === 0) {
    return;
  }
  return trimmed;
};
var enableWebPush = async (sender, message) => {
  if (!webPushSupported()) {
    pushSubscriptionToServer(sender, null);
    return;
  }
  const permission = await Notification.requestPermission();
  if (permission !== "granted") {
    pushSubscriptionToServer(sender, null);
    return;
  }
  const registration = await navigator.serviceWorker.register(message.serviceWorkerPath);
  let subscription = await registration.pushManager.getSubscription();
  if (!subscription) {
    const opts = {
      userVisibleOnly: true
    };
    const key = normalizeVapidPublicKey(message.vapidPublicKey);
    if (key) {
      opts.applicationServerKey = decodeBase64Url(key);
    }
    subscription = await registration.pushManager.subscribe(opts);
  }
  pushSubscriptionToServer(sender, subscription);
};
var disableWebPush = async (sender, message) => {
  if (!webPushSupported()) {
    pushSubscriptionToServer(sender, null);
    return;
  }
  const registration = await navigator.serviceWorker.getRegistration(message.serviceWorkerPath);
  if (!registration) {
    pushSubscriptionToServer(sender, null);
    return;
  }
  const subscription = await registration.pushManager.getSubscription();
  await subscription?.unsubscribe();
  pushSubscriptionToServer(sender, null);
};

// ts/message_sender.ts
var messageKey = (msg) => {
  const id = "id" in msg ? msg.id : "";
  const inx = "inx" in msg ? msg.inx ?? "" : "";
  return `${msg.type}:${id}:${inx}`;
};

class MessageSender {
  sender;
  queue = [];
  timeout = 0;
  constructor(send) {
    this.sender = send;
  }
  send(msg) {
    const key = messageKey(msg);
    this.queue = this.queue.filter((m) => {
      if (messageKey(m) === key) {
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
  sendImmediate(msg) {
    this.sender([msg]);
  }
}

// ts/ws.ts
var connectWebsocket = (args) => {
  let ws;
  const sessionStorageKey = "wgui.sid";
  let inMemorySid;
  const sender = new MessageSender((msgs) => {
    if (!ws || ws.readyState !== WebSocket.OPEN) {
      return;
    }
    ws.send(JSON.stringify(msgs));
  });
  const getSessionId = () => {
    try {
      const existing = window.localStorage.getItem(sessionStorageKey);
      if (existing) {
        return existing;
      }
      const legacy = window.sessionStorage.getItem(sessionStorageKey);
      if (legacy) {
        window.localStorage.setItem(sessionStorageKey, legacy);
        return legacy;
      }
    } catch (_) {}
    if (inMemorySid) {
      return inMemorySid;
    }
    const sid = (window.crypto?.randomUUID?.() ?? `sid-${Date.now()}-${Math.floor(Math.random() * 1e9)}`).replace(/[^a-zA-Z0-9_-]/g, "");
    try {
      window.localStorage.setItem(sessionStorageKey, sid);
    } catch (_) {
      inMemorySid = sid;
    }
    return sid;
  };
  const createConnection = () => {
    args.onConnectionChange?.(false);
    const href = window.location.href;
    const url = new URL(href);
    const wsProtocol = url.protocol === "https:" ? "wss" : "ws";
    const sid = encodeURIComponent(getSessionId());
    const wsUrl = `${wsProtocol}://${url.host}/ws?sid=${sid}`;
    ws = new WebSocket(wsUrl);
    ws.onmessage = (e) => {
      const data = e.data.toString();
      const messages = JSON.parse(data);
      args.onMessage(sender, messages);
    };
    ws.onopen = () => {
      args.onConnectionChange?.(true);
      args.onOpen(sender);
    };
    ws.onclose = () => {
      args.onConnectionChange?.(false);
      setTimeout(() => {
        createConnection();
      }, 1000);
    };
    ws.onerror = (e) => {
      args.onConnectionChange?.(false);
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
var getSetPropValue = (value) => {
  if (!value) {
    return;
  }
  if (value.String != null) {
    return value.String;
  }
  if (value.Number != null) {
    return value.Number.toString();
  }
  return;
};
var applySetProp = (element, set) => {
  const value = getSetPropValue(set.value);
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
    case "Color":
      element.style.color = value;
      break;
    case "Border":
      element.style.border = value;
      break;
    case "Spacing": {
      const parsed = Number(value);
      element.style.gap = isNaN(parsed) ? value : `${parsed}px`;
      break;
    }
    case "FlexDirection":
      element.style.display = "flex";
      element.style.flexDirection = value;
      break;
    case "Grow":
      element.style.flexGrow = value;
      break;
    case "Fill":
      element.style.width = value === "0" ? "" : "100%";
      element.style.boxSizing = value === "0" ? "" : "border-box";
      break;
    case "Width":
      element.style.width = value === "0" ? "" : `${value}px`;
      break;
    case "Height":
      element.style.height = value === "0" ? "" : `${value}px`;
      break;
    case "MinWidth":
      element.style.minWidth = value === "0" ? "" : `${value}px`;
      break;
    case "MaxWidth":
      element.style.maxWidth = value === "0" ? "" : `${value}px`;
      break;
    case "MinHeight":
      element.style.minHeight = value === "0" ? "" : `${value}px`;
      break;
    case "MaxHeight":
      element.style.maxHeight = value === "0" ? "" : `${value}px`;
      break;
    case "Padding":
      element.style.padding = value === "0" ? "" : `${value}px`;
      break;
    case "Overflow":
      element.style.overflow = value;
      break;
    case "WhiteSpace":
      element.style.whiteSpace = value;
      break;
    case "BreakWords":
      element.style.overflowWrap = value === "0" ? "" : "anywhere";
      element.style.wordBreak = value === "0" ? "" : "break-word";
      break;
    case "ID":
      element.id = value;
      break;
  }
};
var clearModalOverlayElement = (element) => {
  if (element.dataset.modal !== "overlay") {
    return;
  }
  delete element.dataset.modal;
  element.removeAttribute("role");
  element.removeAttribute("aria-modal");
  element.removeAttribute("aria-hidden");
  element.onclick = null;
  element.style.position = "";
  element.style.left = "";
  element.style.top = "";
  element.style.alignItems = "";
  element.style.justifyContent = "";
  element.style.backgroundColor = "";
  element.style.backdropFilter = "";
  element.style.zIndex = "";
  element.style.pointerEvents = "";
};
var clearModalOverlays = (root) => {
  clearModalOverlayElement(root);
  for (const overlay of root.querySelectorAll("[data-modal='overlay']")) {
    if (overlay instanceof HTMLElement) {
      clearModalOverlayElement(overlay);
    }
  }
};
var bodyAppRoot = (body) => body.firstElementChild ?? undefined;
var bodyPathItem = (body, path) => {
  const root = bodyAppRoot(body);
  if (!root) {
    return;
  }
  return getPathItem(path, root);
};
var renderBodyRoot = (body, item, ctx) => {
  const current = bodyAppRoot(body);
  if (current) {
    renderItem(item, ctx, current);
  } else {
    const rendered = renderItem(item, ctx);
    if (rendered) {
      body.appendChild(rendered);
    }
  }
};
var takeSsrRoot = () => {
  const element = document.getElementById("wgui-ssr-root");
  if (!element?.textContent) {
    return;
  }
  element.remove();
  try {
    return JSON.parse(element.textContent);
  } catch (err) {
    console.warn("failed to parse SSR root", err);
    return;
  }
};
var takeSsrHydrationId = () => {
  const element = document.querySelector('meta[name="wgui-ssr-id"]');
  if (!(element instanceof HTMLMetaElement)) {
    return;
  }
  const id = element.content;
  element.remove();
  return id || undefined;
};
var shouldIgnoreKeyboardEvent = (event) => {
  const target = event.target;
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tag = target.tagName.toLowerCase();
  return tag === "input" || tag === "textarea" || target.isContentEditable;
};
var shouldPreventDefaultKey = (event) => event.code === "ArrowUp" || event.code === "ArrowDown" || event.code === "ArrowLeft" || event.code === "ArrowRight";
window.onload = () => {
  const res = document.querySelector("body");
  if (!res) {
    return;
  }
  res.style.display = "flex";
  res.style.flexDirection = "row";
  res.style.height = "100vh";
  res.style.margin = "0";
  res.style.width = "100%";
  const debouncer = new Deboncer;
  let rtc;
  let initialRoot = takeSsrRoot();
  let ssrHydrationId = takeSsrHydrationId();
  const activeKeyboardKeys = new Set;
  const {
    sender
  } = connectWebsocket({
    onMessage: (sender2, msgs) => {
      if (!rtc) {
        rtc = new WebRtcCoordinator(sender2);
      }
      const ctx = {
        sender: sender2,
        debouncer
      };
      for (const message of msgs) {
        if (message.type === "pushState") {
          const next = new URL(message.url, window.location.href);
          const current = `${location.pathname}${location.search}${location.hash}`;
          const nextPath = `${next.pathname}${next.search}${next.hash}`;
          if (nextPath !== current) {
            history.pushState({}, "", message.url);
          }
          clearModalOverlays(res);
          sender2.send({
            type: "pathChanged",
            path: location.pathname,
            query: {}
          });
          sender2.sendNow();
          continue;
        }
        if (message.type === "navigate") {
          window.location.assign(message.url);
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
        if (message.type === "setTitle") {
          document.title = message.title;
          continue;
        }
        if (message.type === "webRtcRoomState" || message.type === "webRtcSignal") {
          rtc.handleServerMessage(message);
          continue;
        }
        if (message.type === "webPushEnable") {
          enableWebPush(sender2, message).catch((err) => {
            console.warn("web push enable failed", err);
          });
          continue;
        }
        if (message.type === "webPushDisable") {
          disableWebPush(sender2, message).catch((err) => {
            console.warn("web push disable failed", err);
          });
          continue;
        }
        if (message.type === "customData") {
          sendCustomData(res, message.id, message.inx, message.name, message.payload);
          continue;
        }
        if (message.type === "setProp") {
          const target = bodyPathItem(res, message.path);
          if (!target) {
            continue;
          }
          for (const set of message.sets) {
            applySetProp(target, set);
          }
          continue;
        }
        if (message.type === "replace" && message.path.length === 0) {
          renderBodyRoot(res, message.item, ctx);
          continue;
        }
        const element = bodyPathItem(res, message.path);
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
          const child = element.children.item(message.inx);
          disposeCustomComponentTree(child);
          child?.remove();
        }
      }
      rtc.syncElements(res);
    },
    onOpen: (sender2) => {
      if (!rtc) {
        rtc = new WebRtcCoordinator(sender2);
      }
      rtc.onSocketOpen();
      const params = new URLSearchParams(location.search);
      const query = {};
      params.forEach((value, key) => {
        query[key] = value;
      });
      sender2.send({
        type: "pathChanged",
        path: location.pathname,
        query,
        ssrHydrationId
      });
      if (initialRoot) {
        const ctx = {
          sender: sender2,
          debouncer
        };
        renderBodyRoot(res, initialRoot, ctx);
        initialRoot = undefined;
      }
      ssrHydrationId = undefined;
      sender2.sendNow();
      rtc.syncElements(res);
    },
    onConnectionChange: setConnectionStatus
  });
  window.addEventListener("keydown", (event) => {
    if (event.repeat || shouldIgnoreKeyboardEvent(event)) {
      return;
    }
    if (shouldPreventDefaultKey(event)) {
      event.preventDefault();
    }
    const keycode = event.code || event.key;
    activeKeyboardKeys.add(keycode);
    sender.send({
      type: "onKeyDown",
      keycode
    });
    sender.sendNow();
  });
  window.addEventListener("keyup", (event) => {
    const keycode = event.code || event.key;
    if (shouldIgnoreKeyboardEvent(event) && !activeKeyboardKeys.has(keycode)) {
      return;
    }
    if (shouldPreventDefaultKey(event)) {
      event.preventDefault();
    }
    activeKeyboardKeys.delete(keycode);
    sender.send({
      type: "onKeyUp",
      keycode
    });
    sender.sendNow();
  });
  window.addEventListener("blur", () => {
    if (activeKeyboardKeys.size === 0) {
      return;
    }
    for (const keycode of activeKeyboardKeys) {
      sender.send({
        type: "onKeyUp",
        keycode
      });
    }
    activeKeyboardKeys.clear();
    sender.sendNow();
  });
  window.addEventListener("popstate", (evet) => {
    clearModalOverlays(res);
    const params = new URLSearchParams(location.search);
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
