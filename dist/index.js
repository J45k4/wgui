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

// ts/three_host.ts
var threeLoadPromise = null;
var getThree = () => {
  const three = window.THREE;
  return three ?? null;
};
var dynamicImport = (url) => {
  const importer = new Function("u", "return import(u)");
  return importer(url);
};
var loadThree = () => {
  if (threeLoadPromise) {
    return threeLoadPromise;
  }
  threeLoadPromise = new Promise((resolve, reject) => {
    const moduleUrl = window.WGUI_THREE_MODULE_URL ?? "https://cdnjs.cloudflare.com/ajax/libs/three.js/0.180.0/three.module.js";
    const sources = [
      window.WGUI_THREE_URL,
      "/three.min.js",
      "https://unpkg.com/three@0.161.0/build/three.min.js"
    ].filter(Boolean);
    const waitForThree = (timeoutMs) => {
      const start = Date.now();
      const check = () => {
        const three = getThree();
        if (three) {
          resolve(three);
          return;
        }
        if (Date.now() - start > timeoutMs) {
          return;
        }
        setTimeout(check, 50);
      };
      check();
    };
    const tryLoad = (index) => {
      if (index >= sources.length) {
        reject(new Error("Failed to load Three.js"));
        return;
      }
      const src = sources[index];
      const existing = document.querySelector(`script[data-wgui-three-src="${src}"]`);
      if (existing) {
        waitForThree(1500);
        setTimeout(() => {
          if (!getThree()) {
            tryLoad(index + 1);
          }
        }, 1600);
        return;
      }
      const script = document.createElement("script");
      script.src = src;
      script.async = true;
      script.dataset.wguiThree = "true";
      script.dataset.wguiThreeSrc = src;
      script.onload = () => {
        const three = getThree();
        if (three) {
          resolve(three);
        } else {
          tryLoad(index + 1);
        }
      };
      script.onerror = () => {
        tryLoad(index + 1);
      };
      document.head.appendChild(script);
    };
    if (getThree()) {
      resolve(getThree());
      return;
    }
    dynamicImport(moduleUrl).then(async (threeModule) => {
      window.THREE = threeModule;
      resolve(threeModule);
    }).catch(() => {
      tryLoad(0);
    });
  });
  return threeLoadPromise;
};
var hostMap = new WeakMap;
var applyThreeTree = (canvas, root) => {
  const host = ensureThreeHost(canvas);
  host.reset(root);
};
var applyThreePatch = (element, ops) => {
  if (!(element instanceof HTMLCanvasElement)) {
    return;
  }
  const host = ensureThreeHost(element);
  host.applyOps(ops);
};
var disposeThreeHost = (element) => {
  if (!(element instanceof HTMLCanvasElement)) {
    return;
  }
  const host = hostMap.get(element);
  if (host) {
    host.dispose();
    hostMap.delete(element);
  }
};
var ensureThreeHost = (canvas) => {
  let host = hostMap.get(canvas);
  if (!host) {
    host = new ThreeHost(canvas);
    hostMap.set(canvas, host);
  }
  return host;
};

class ThreeHost {
  canvas;
  three;
  renderer;
  scene;
  activeCamera;
  objects;
  kinds;
  parents;
  resizeObserver;
  running;
  pendingRoot;
  pendingOps;
  stlLoadTokens;
  isMiddlePanning;
  isRightRotating;
  panLastX;
  panLastY;
  rotateLastX;
  rotateLastY;
  onMouseDown;
  onMouseMove;
  onMouseUp;
  onAuxClick;
  onContextMenu;
  onWheel;
  constructor(canvas) {
    this.canvas = canvas;
    this.three = getThree();
    this.renderer = null;
    this.scene = null;
    this.activeCamera = null;
    this.objects = new Map;
    this.kinds = new Map;
    this.parents = new Map;
    this.resizeObserver = null;
    this.running = false;
    this.pendingRoot = null;
    this.pendingOps = [];
    this.stlLoadTokens = new Map;
    this.isMiddlePanning = false;
    this.isRightRotating = false;
    this.panLastX = 0;
    this.panLastY = 0;
    this.rotateLastX = 0;
    this.rotateLastY = 0;
    this.onMouseDown = null;
    this.onMouseMove = null;
    this.onMouseUp = null;
    this.onAuxClick = null;
    this.onContextMenu = null;
    this.onWheel = null;
    if (!this.three) {
      loadThree().then((three) => {
        this.initWithThree(three);
      }).catch((err) => {
        console.warn("Failed to load Three.js", err);
      });
      return;
    }
    this.initWithThree(this.three);
  }
  reset(root) {
    if (!this.three || !this.scene) {
      this.pendingRoot = root;
      return;
    }
    this.clear();
    this.buildFromTree(root);
  }
  applyOps(ops) {
    if (!this.three || !this.scene) {
      this.pendingOps.push(...ops);
      return;
    }
    for (const op of ops) {
      this.applyOp(op);
    }
  }
  dispose() {
    this.stop();
    this.teardownPanControls();
    if (this.resizeObserver) {
      this.resizeObserver.disconnect();
      this.resizeObserver = null;
    }
    this.clear();
    if (this.renderer) {
      this.renderer.dispose();
    }
  }
  start() {
    if (this.running) {
      return;
    }
    this.running = true;
    const loop = () => {
      if (!this.running) {
        return;
      }
      if (this.renderer && this.scene && this.activeCamera) {
        this.renderer.render(this.scene, this.activeCamera);
      }
      requestAnimationFrame(loop);
    };
    requestAnimationFrame(loop);
  }
  initWithThree(three) {
    if (this.three && this.scene) {
      return;
    }
    if (!three.WebGLRenderer) {
      console.error("Loaded THREE module keys:", Object.keys(three));
      throw new Error("Three loaded, but WebGLRenderer missing (wrong build?)");
    }
    this.three = three;
    const THREE = this.three;
    this.renderer = new THREE.WebGLRenderer({ canvas: this.canvas, antialias: true });
    this.renderer.setPixelRatio(window.devicePixelRatio || 1);
    this.scene = new THREE.Scene;
    this.setupResizeObserver();
    this.setupPanControls();
    this.start();
    if (this.pendingRoot) {
      const root = this.pendingRoot;
      this.pendingRoot = null;
      this.reset(root);
    }
    if (this.pendingOps.length > 0) {
      const ops = [...this.pendingOps];
      this.pendingOps = [];
      this.applyOps(ops);
    }
  }
  stop() {
    this.running = false;
  }
  clear() {
    if (!this.scene) {
      return;
    }
    for (const child of [...this.scene.children]) {
      this.scene.remove(child);
    }
    this.objects.clear();
    this.kinds.clear();
    this.parents.clear();
    this.stlLoadTokens.clear();
    this.activeCamera = null;
  }
  buildFromTree(root) {
    const stack = [
      { node: root, parentId: null }
    ];
    while (stack.length) {
      const entry = stack.shift();
      if (!entry) {
        continue;
      }
      this.createNode(entry.node);
      if (entry.parentId != null) {
        this.attach(entry.parentId, entry.node.id);
      }
      for (const child of entry.node.children) {
        stack.push({ node: child, parentId: entry.node.id });
      }
    }
  }
  applyOp(op) {
    switch (op.type) {
      case "create":
        this.createNode({
          id: op.id,
          kind: op.kind,
          props: op.props,
          children: []
        });
        return;
      case "attach":
        this.attach(op.parentId, op.childId);
        return;
      case "detach":
        this.detach(op.parentId, op.childId);
        return;
      case "setProp":
        this.setProp(op.id, op.key, op.value);
        return;
      case "unsetProp":
        this.unsetProp(op.id, op.key);
        return;
      case "delete":
        this.deleteNode(op.id);
        return;
    }
  }
  createNode(node) {
    if (!this.three || !this.scene) {
      return;
    }
    const THREE = this.three;
    let obj = null;
    switch (node.kind) {
      case "scene":
        obj = this.scene;
        break;
      case "group":
        obj = new THREE.Group;
        break;
      case "mesh":
        obj = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshStandardMaterial({ color: 16777215 }));
        break;
      case "perspectiveCamera":
        obj = new THREE.PerspectiveCamera(50, 1, 0.1, 2000);
        break;
      case "orthographicCamera":
        obj = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 2000);
        break;
      case "boxGeometry":
        obj = new THREE.BoxGeometry(1, 1, 1);
        break;
      case "sphereGeometry":
        obj = new THREE.SphereGeometry(1, 32, 16);
        break;
      case "cylinderGeometry":
        obj = new THREE.CylinderGeometry(1, 1, 1, 24);
        break;
      case "stlGeometry":
        obj = new THREE.BufferGeometry;
        break;
      case "meshStandardMaterial":
        obj = new THREE.MeshStandardMaterial({ color: 16777215 });
        break;
      case "meshBasicMaterial":
        obj = new THREE.MeshBasicMaterial({ color: 16777215 });
        break;
      case "ambientLight":
        obj = new THREE.AmbientLight(16777215, 1);
        break;
      case "directionalLight":
        obj = new THREE.DirectionalLight(16777215, 1);
        break;
      case "pointLight":
        obj = new THREE.PointLight(16777215, 1);
        break;
    }
    if (!obj) {
      return;
    }
    this.objects.set(node.id, obj);
    this.kinds.set(node.id, node.kind);
    this.parents.set(node.id, null);
    for (const prop of node.props) {
      this.setProp(node.id, prop.key, prop.value);
    }
  }
  attach(parentId, childId) {
    if (!this.scene || !this.three) {
      return;
    }
    const parent = this.objects.get(parentId);
    const child = this.objects.get(childId);
    if (!parent || !child) {
      return;
    }
    const parentKind = this.kinds.get(parentId);
    const childKind = this.kinds.get(childId);
    if (parentKind === "mesh" && childKind) {
      if (childKind.endsWith("Geometry")) {
        parent.geometry = child;
        this.parents.set(childId, parentId);
        return;
      }
      if (childKind.endsWith("Material")) {
        parent.material = child;
        this.parents.set(childId, parentId);
        return;
      }
    }
    if (parent.add) {
      parent.add(child);
      this.parents.set(childId, parentId);
    }
  }
  detach(parentId, childId) {
    const parent = this.objects.get(parentId);
    const child = this.objects.get(childId);
    if (!parent || !child) {
      return;
    }
    const parentKind = this.kinds.get(parentId);
    const childKind = this.kinds.get(childId);
    if (parentKind === "mesh" && childKind) {
      if (childKind.endsWith("Geometry") && parent.geometry === child) {
        parent.geometry = null;
        this.parents.set(childId, null);
        return;
      }
      if (childKind.endsWith("Material") && parent.material === child) {
        parent.material = null;
        this.parents.set(childId, null);
        return;
      }
    }
    if (parent.remove) {
      parent.remove(child);
      this.parents.set(childId, null);
    }
  }
  deleteNode(id) {
    const obj = this.objects.get(id);
    if (!obj) {
      return;
    }
    const parentId = this.parents.get(id);
    if (parentId != null) {
      this.detach(parentId, id);
    }
    this.objects.delete(id);
    this.kinds.delete(id);
    this.parents.delete(id);
    this.stlLoadTokens.delete(id);
    if (obj.dispose) {
      obj.dispose();
    }
  }
  setProp(id, key, value) {
    const obj = this.objects.get(id);
    if (!obj) {
      return;
    }
    const THREE = this.three;
    const decoded = decodeValue(value);
    switch (key) {
      case "position":
        if (decoded && obj.position) {
          obj.position.set(decoded.x, decoded.y, decoded.z);
        }
        return;
      case "rotation":
        if (decoded && obj.rotation) {
          obj.rotation.set(decoded.x, decoded.y, decoded.z);
        }
        return;
      case "scale":
        if (decoded && obj.scale) {
          obj.scale.set(decoded.x, decoded.y, decoded.z);
        }
        return;
      case "lookAt":
        if (decoded && obj.lookAt) {
          obj.lookAt(decoded.x, decoded.y, decoded.z);
        }
        return;
      case "up":
        if (decoded && obj.up) {
          obj.up.set(decoded.x, decoded.y, decoded.z);
        }
        return;
      case "visible":
        if (typeof decoded === "boolean") {
          obj.visible = decoded;
        }
        return;
      case "name":
        if (typeof decoded === "string") {
          obj.name = decoded;
        }
        return;
      case "castShadow":
        if (typeof decoded === "boolean") {
          obj.castShadow = decoded;
        }
        return;
      case "receiveShadow":
        if (typeof decoded === "boolean") {
          obj.receiveShadow = decoded;
        }
        return;
      case "color":
        if (decoded && obj.color) {
          obj.color = new THREE.Color(decoded.r / 255, decoded.g / 255, decoded.b / 255);
        }
        return;
      case "intensity":
        if (typeof decoded === "number") {
          obj.intensity = decoded;
        }
        return;
      case "fov":
        if (typeof decoded === "number") {
          obj.fov = decoded;
          if (obj.updateProjectionMatrix)
            obj.updateProjectionMatrix();
        }
        return;
      case "near":
        if (typeof decoded === "number") {
          obj.near = decoded;
          if (obj.updateProjectionMatrix)
            obj.updateProjectionMatrix();
        }
        return;
      case "far":
        if (typeof decoded === "number") {
          obj.far = decoded;
          if (obj.updateProjectionMatrix)
            obj.updateProjectionMatrix();
        }
        return;
      case "active":
        if (typeof decoded === "boolean") {
          if (decoded) {
            this.activeCamera = obj;
          } else if (this.activeCamera === obj) {
            this.activeCamera = null;
          }
        }
        return;
    }
    const kind = this.kinds.get(id);
    if (kind === "boxGeometry" && typeof decoded === "number" && obj.parameters) {
      if (key === "width" || key === "height" || key === "depth") {
        const width = key === "width" ? decoded : obj.parameters.width ?? 1;
        const height = key === "height" ? decoded : obj.parameters.height ?? 1;
        const depth = key === "depth" ? decoded : obj.parameters.depth ?? 1;
        this.replaceGeometry(id, new THREE.BoxGeometry(width, height, depth));
      }
      return;
    }
    if (kind === "sphereGeometry" && typeof decoded === "number" && obj.parameters) {
      if (key === "radius" || key === "widthSegments" || key === "heightSegments") {
        const radius = key === "radius" ? decoded : obj.parameters.radius ?? 1;
        const widthSegments = key === "widthSegments" ? decoded : obj.parameters.widthSegments ?? 32;
        const heightSegments = key === "heightSegments" ? decoded : obj.parameters.heightSegments ?? 16;
        this.replaceGeometry(id, new THREE.SphereGeometry(radius, widthSegments, heightSegments));
      }
      return;
    }
    if (kind === "cylinderGeometry" && typeof decoded === "number" && obj.parameters) {
      if (key === "radiusTop" || key === "radiusBottom" || key === "height" || key === "radialSegments") {
        const radiusTop = key === "radiusTop" ? decoded : obj.parameters.radiusTop ?? 1;
        const radiusBottom = key === "radiusBottom" ? decoded : obj.parameters.radiusBottom ?? 1;
        const height = key === "height" ? decoded : obj.parameters.height ?? 1;
        const radialSegments = key === "radialSegments" ? decoded : obj.parameters.radialSegments ?? 24;
        this.replaceGeometry(id, new THREE.CylinderGeometry(radiusTop, radiusBottom, height, radialSegments));
      }
      return;
    }
    if (kind === "stlGeometry") {
      if (key === "src" && typeof decoded === "string") {
        this.loadStlGeometry(id, decoded);
      }
      return;
    }
    if (kind && kind.endsWith("Material")) {
      if (key === "metalness" && typeof decoded === "number") {
        obj.metalness = decoded;
        return;
      }
      if (key === "roughness" && typeof decoded === "number") {
        obj.roughness = decoded;
        return;
      }
      if (key === "wireframe" && typeof decoded === "boolean") {
        obj.wireframe = decoded;
        return;
      }
      if (key === "opacity" && typeof decoded === "number") {
        obj.opacity = decoded;
        obj.transparent = decoded < 1;
        return;
      }
    }
  }
  unsetProp(id, key) {
    const obj = this.objects.get(id);
    if (!obj) {
      return;
    }
    if (key === "active" && this.activeCamera === obj) {
      this.activeCamera = null;
    }
    if (key === "src" && this.kinds.get(id) === "stlGeometry") {
      this.stlLoadTokens.set(id, (this.stlLoadTokens.get(id) ?? 0) + 1);
    }
  }
  replaceGeometry(id, geometry) {
    const obj = this.objects.get(id);
    if (!obj) {
      return;
    }
    const parentId = this.parents.get(id);
    if (parentId != null) {
      const parent = this.objects.get(parentId);
      if (parent && parent.geometry) {
        parent.geometry = geometry;
      }
    }
    this.objects.set(id, geometry);
  }
  loadStlGeometry(id, src) {
    if (!this.three) {
      return;
    }
    const token = (this.stlLoadTokens.get(id) ?? 0) + 1;
    this.stlLoadTokens.set(id, token);
    fetch(src).then((response) => {
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      return response.arrayBuffer();
    }).then((buffer) => {
      if (this.stlLoadTokens.get(id) !== token) {
        return;
      }
      const geometry = parseStl(this.three, buffer);
      this.replaceGeometry(id, geometry);
    }).catch((err) => {
      if (this.stlLoadTokens.get(id) !== token) {
        return;
      }
      console.warn(`Failed to load STL geometry from "${src}"`, err);
    });
  }
  setupResizeObserver() {
    if (!this.renderer) {
      return;
    }
    const resize = () => {
      if (!this.renderer || !this.canvas) {
        return;
      }
      const width = this.canvas.clientWidth;
      const height = this.canvas.clientHeight;
      if (width === 0 || height === 0) {
        return;
      }
      this.renderer.setSize(width, height, false);
      if (this.activeCamera) {
        if (this.activeCamera.isPerspectiveCamera) {
          this.activeCamera.aspect = width / height;
          this.activeCamera.updateProjectionMatrix();
        }
      }
    };
    this.resizeObserver = new ResizeObserver(resize);
    this.resizeObserver.observe(this.canvas);
    resize();
  }
  setupPanControls() {
    if (this.onMouseDown || this.onMouseMove || this.onMouseUp || this.onWheel) {
      return;
    }
    this.onMouseDown = (event) => {
      if (event.button !== 1 && event.button !== 2) {
        return;
      }
      event.preventDefault();
      if (event.button === 1) {
        this.isMiddlePanning = true;
        this.panLastX = event.clientX;
        this.panLastY = event.clientY;
      }
      if (event.button === 2) {
        this.isRightRotating = true;
        this.rotateLastX = event.clientX;
        this.rotateLastY = event.clientY;
      }
      document.body.style.userSelect = "none";
      document.body.style.cursor = "grabbing";
    };
    this.onMouseMove = (event) => {
      if (this.isMiddlePanning) {
        event.preventDefault();
        const dx = event.clientX - this.panLastX;
        const dy = event.clientY - this.panLastY;
        this.panLastX = event.clientX;
        this.panLastY = event.clientY;
        this.panFromCamera(dx, dy);
      }
      if (this.isRightRotating) {
        event.preventDefault();
        const dx = event.clientX - this.rotateLastX;
        const dy = event.clientY - this.rotateLastY;
        this.rotateLastX = event.clientX;
        this.rotateLastY = event.clientY;
        this.rotateFromCamera(dx, dy);
      }
    };
    this.onMouseUp = (event) => {
      if (event.button === 1) {
        this.isMiddlePanning = false;
      }
      if (event.button === 2) {
        this.isRightRotating = false;
      }
      this.stopControlsInteraction();
    };
    this.onAuxClick = (event) => {
      if (event.button === 1) {
        event.preventDefault();
      }
    };
    this.onContextMenu = (event) => {
      event.preventDefault();
    };
    this.onWheel = (event) => {
      event.preventDefault();
      this.zoomFromWheel(event.deltaY);
    };
    this.canvas.addEventListener("mousedown", this.onMouseDown);
    window.addEventListener("mousemove", this.onMouseMove);
    window.addEventListener("mouseup", this.onMouseUp);
    this.canvas.addEventListener("auxclick", this.onAuxClick);
    this.canvas.addEventListener("contextmenu", this.onContextMenu);
    this.canvas.addEventListener("wheel", this.onWheel, { passive: false });
  }
  teardownPanControls() {
    this.isMiddlePanning = false;
    this.isRightRotating = false;
    this.stopControlsInteraction();
    if (this.onMouseDown) {
      this.canvas.removeEventListener("mousedown", this.onMouseDown);
      this.onMouseDown = null;
    }
    if (this.onMouseMove) {
      window.removeEventListener("mousemove", this.onMouseMove);
      this.onMouseMove = null;
    }
    if (this.onMouseUp) {
      window.removeEventListener("mouseup", this.onMouseUp);
      this.onMouseUp = null;
    }
    if (this.onAuxClick) {
      this.canvas.removeEventListener("auxclick", this.onAuxClick);
      this.onAuxClick = null;
    }
    if (this.onContextMenu) {
      this.canvas.removeEventListener("contextmenu", this.onContextMenu);
      this.onContextMenu = null;
    }
    if (this.onWheel) {
      this.canvas.removeEventListener("wheel", this.onWheel);
      this.onWheel = null;
    }
  }
  stopControlsInteraction() {
    if (this.isMiddlePanning || this.isRightRotating) {
      return;
    }
    document.body.style.userSelect = "";
    document.body.style.cursor = "";
  }
  panFromCamera(dx, dy) {
    if (!this.three || !this.activeCamera || !this.activeCamera.position) {
      return;
    }
    const THREE = this.three;
    const camera = this.activeCamera;
    const cameraPos = camera.position;
    const distance = Math.max(1, cameraPos.length ? cameraPos.length() : 1);
    const factor = distance / Math.max(this.canvas.clientHeight || 1, 1);
    const right = new THREE.Vector3(1, 0, 0).applyQuaternion(camera.quaternion);
    const up = new THREE.Vector3(0, 1, 0).applyQuaternion(camera.quaternion);
    const delta = right.multiplyScalar(-dx * factor).add(up.multiplyScalar(dy * factor));
    cameraPos.add(delta);
  }
  zoomFromWheel(deltaY) {
    if (!this.three || !this.activeCamera) {
      return;
    }
    const THREE = this.three;
    const camera = this.activeCamera;
    if (camera.isPerspectiveCamera && camera.position) {
      const amount = Math.max(-1, Math.min(1, -deltaY)) * 0.1;
      const forward = new THREE.Vector3(0, 0, -1).applyQuaternion(camera.quaternion);
      camera.position.add(forward.multiplyScalar(amount));
      return;
    }
    if (camera.isOrthographicCamera) {
      const factor = deltaY > 0 ? 0.9 : 1.1;
      camera.zoom = Math.max(0.05, Math.min(100, camera.zoom / factor));
      camera.updateProjectionMatrix();
    }
  }
  rotateFromCamera(dx, dy) {
    if (!this.three || !this.activeCamera) {
      return;
    }
    const THREE = this.three;
    const camera = this.activeCamera;
    if (!camera.quaternion) {
      return;
    }
    const yaw = -dx * 0.005;
    const pitch = -dy * 0.005;
    const up = new THREE.Vector3(0, 1, 0);
    const right = new THREE.Vector3(1, 0, 0).applyQuaternion(camera.quaternion);
    camera.quaternion.premultiply(new THREE.Quaternion().setFromAxisAngle(up, yaw));
    camera.quaternion.multiply(new THREE.Quaternion().setFromAxisAngle(right, pitch));
  }
}
var decodeValue = (value) => {
  switch (value.type) {
    case "number":
      return value.value;
    case "bool":
      return value.value;
    case "string":
      return value.value;
    case "vec3":
      return { x: value.x, y: value.y, z: value.z };
    case "color":
      return { r: value.r, g: value.g, b: value.b, a: value.a };
  }
};
var parseStl = (THREE, arrayBuffer) => {
  if (isBinaryStl(arrayBuffer)) {
    return parseBinaryStl(THREE, arrayBuffer);
  }
  return parseAsciiStl(THREE, arrayBuffer);
};
var isBinaryStl = (arrayBuffer) => {
  if (arrayBuffer.byteLength < 84) {
    return false;
  }
  const dataView = new DataView(arrayBuffer);
  const triangleCount = dataView.getUint32(80, true);
  const expected = 84 + triangleCount * 50;
  return expected === arrayBuffer.byteLength;
};
var parseBinaryStl = (THREE, arrayBuffer) => {
  const dataView = new DataView(arrayBuffer);
  const triangleCount = dataView.getUint32(80, true);
  const positions = [];
  const normals = [];
  let offset = 84;
  for (let i = 0;i < triangleCount; i++) {
    const nx = dataView.getFloat32(offset, true);
    const ny = dataView.getFloat32(offset + 4, true);
    const nz = dataView.getFloat32(offset + 8, true);
    offset += 12;
    for (let v = 0;v < 3; v++) {
      const x = dataView.getFloat32(offset, true);
      const y = dataView.getFloat32(offset + 4, true);
      const z = dataView.getFloat32(offset + 8, true);
      positions.push(x, y, z);
      normals.push(nx, ny, nz);
      offset += 12;
    }
    offset += 2;
  }
  return buildGeometry(THREE, positions, normals);
};
var parseAsciiStl = (THREE, arrayBuffer) => {
  const text = new TextDecoder().decode(arrayBuffer);
  const positions = [];
  const normals = [];
  const facetRegex = /facet\s+normal\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+outer\s+loop\s+vertex\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+vertex\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+vertex\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+endloop\s+endfacet/gi;
  let match;
  while ((match = facetRegex.exec(text)) !== null) {
    const numbers = match.slice(1).map(Number);
    const [nx, ny, nz, x1, y1, z1, x2, y2, z2, x3, y3, z3] = numbers;
    positions.push(x1, y1, z1, x2, y2, z2, x3, y3, z3);
    normals.push(nx, ny, nz, nx, ny, nz, nx, ny, nz);
  }
  return buildGeometry(THREE, positions, normals);
};
var buildGeometry = (THREE, positions, normals) => {
  const geometry = new THREE.BufferGeometry;
  geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3));
  const hasUsableNormals = normals.length === positions.length && normals.some((component) => Math.abs(component) > 0.000001);
  if (hasUsableNormals) {
    geometry.setAttribute("normal", new THREE.Float32BufferAttribute(normals, 3));
  } else {
    geometry.computeVertexNormals();
  }
  if (geometry.computeBoundingSphere) {
    geometry.computeBoundingSphere();
  }
  return geometry;
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
      let handle = element.querySelector(".wgui-resize-handle");
      if (!handle) {
        handle = document.createElement("div");
        handle.className = "wgui-resize-handle";
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
          element.style.width = `${width}px`;
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
    textarea.value = payload.value;
    const rowCount = payload.value.split(`
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
    } else {
      table = document.createElement("table");
      if (old)
        old.replaceWith(table);
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
    for (const child of overlay.children) {
      if (child instanceof HTMLElement) {
        child.style.maxWidth = "calc(100vw - 64px)";
        child.style.maxHeight = "calc(100vh - 64px)";
      }
    }
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
  if (payload.type === "threeView") {
    let canvas;
    if (old instanceof HTMLCanvasElement) {
      canvas = old;
    } else {
      canvas = document.createElement("canvas");
      if (old)
        old.replaceWith(canvas);
    }
    canvas.dataset.wguiThree = "true";
    canvas.style.display = "block";
    canvas.style.width = "100%";
    canvas.style.height = "100%";
    applyThreeTree(canvas, payload.root);
    return canvas;
  }
};
var renderItem = (item, ctx, old) => {
  if (old instanceof HTMLCanvasElement && item.payload.type !== "threeView") {
    disposeThreeHost(old);
  }
  const element = renderPayload(item, ctx, old);
  if (!element) {
    return;
  }
  element.style.width = item.width ? item.width + "px" : "";
  element.style.height = item.height ? item.height + "px" : "";
  element.style.minWidth = item.minWidth ? item.minWidth + "px" : "";
  element.style.maxWidth = item.maxWidth ? item.maxWidth + "px" : "";
  element.style.minHeight = item.minHeight ? item.minHeight + "px" : "";
  element.style.maxHeight = item.maxHeight ? item.maxHeight + "px" : "";
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
  if (item.payload.type !== "modal" && !(element instanceof HTMLInputElement) && !(element instanceof HTMLSelectElement) && !(element instanceof HTMLTextAreaElement)) {
    bindAutoClick(element, item, ctx);
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
  const sessionStorageKey = "wgui.sid";
  const sender = new MessageSender((msgs) => {
    if (!ws) {
      return;
    }
    ws.send(JSON.stringify(msgs));
  });
  const getSessionId = () => {
    const existing = window.sessionStorage.getItem(sessionStorageKey);
    if (existing) {
      return existing;
    }
    const sid = (window.crypto?.randomUUID?.() ?? `sid-${Date.now()}-${Math.floor(Math.random() * 1e9)}`).replace(/[^a-zA-Z0-9_-]/g, "");
    window.sessionStorage.setItem(sessionStorageKey, sid);
    return sid;
  };
  const createConnection = () => {
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
    case "ID":
      element.id = value;
      break;
  }
};
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
  let root = res.querySelector("#wgui-root");
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
        if (message.type === "setTitle") {
          document.title = message.title;
          continue;
        }
        if (message.type === "threePatch") {
          const target = getPathItem(message.path, root);
          if (target) {
            applyThreePatch(target, message.ops);
          }
          continue;
        }
        if (message.type === "setProp") {
          const target = getPathItem(message.path, root);
          if (!target) {
            continue;
          }
          for (const set of message.sets) {
            applySetProp(target, set);
          }
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
      const params = new URLSearchParams(location.search);
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
