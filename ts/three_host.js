"use strict";
var __spreadArray = (this && this.__spreadArray) || function (to, from, pack) {
    if (pack || arguments.length === 2) for (var i = 0, l = from.length, ar; i < l; i++) {
        if (ar || !(i in from)) {
            if (!ar) ar = Array.prototype.slice.call(from, 0, i);
            ar[i] = from[i];
        }
    }
    return to.concat(ar || Array.prototype.slice.call(from));
};
Object.defineProperty(exports, "__esModule", { value: true });
exports.disposeThreeHost = exports.applyThreePatch = exports.applyThreeTree = void 0;
var threeLoadPromise = null;
var getThree = function () {
    var three = window.THREE;
    return three !== null && three !== void 0 ? three : null;
};
var loadThree = function () {
    if (threeLoadPromise) {
        return threeLoadPromise;
    }
    threeLoadPromise = new Promise(function (resolve, reject) {
        var existing = document.querySelector("script[data-wgui-three]");
        if (existing) {
            var check_1 = function () {
                var three = getThree();
                if (three) {
                    resolve(three);
                }
                else {
                    setTimeout(check_1, 50);
                }
            };
            check_1();
            return;
        }
        var script = document.createElement("script");
        script.src = "https://unpkg.com/three@0.161.0/build/three.min.js";
        script.async = true;
        script.dataset.wguiThree = "true";
        script.onload = function () {
            var three = getThree();
            if (three) {
                resolve(three);
            }
            else {
                reject(new Error("Three.js loaded but window.THREE is missing"));
            }
        };
        script.onerror = function () { return reject(new Error("Failed to load Three.js")); };
        document.head.appendChild(script);
    });
    return threeLoadPromise;
};
var hostMap = new WeakMap();
var applyThreeTree = function (canvas, root) {
    var host = ensureThreeHost(canvas);
    host.reset(root);
};
exports.applyThreeTree = applyThreeTree;
var applyThreePatch = function (element, ops) {
    if (!(element instanceof HTMLCanvasElement)) {
        return;
    }
    var host = ensureThreeHost(element);
    host.applyOps(ops);
};
exports.applyThreePatch = applyThreePatch;
var disposeThreeHost = function (element) {
    if (!(element instanceof HTMLCanvasElement)) {
        return;
    }
    var host = hostMap.get(element);
    if (host) {
        host.dispose();
        hostMap.delete(element);
    }
};
exports.disposeThreeHost = disposeThreeHost;
var ensureThreeHost = function (canvas) {
    var host = hostMap.get(canvas);
    if (!host) {
        host = new ThreeHost(canvas);
        hostMap.set(canvas, host);
    }
    return host;
};
var ThreeHost = /** @class */ (function () {
    function ThreeHost(canvas) {
        var _this = this;
        this.canvas = canvas;
        this.three = getThree();
        this.renderer = null;
        this.scene = null;
        this.activeCamera = null;
        this.objects = new Map();
        this.kinds = new Map();
        this.parents = new Map();
        this.resizeObserver = null;
        this.running = false;
        this.pendingRoot = null;
        this.pendingOps = [];
        if (!this.three) {
            loadThree()
                .then(function (three) {
                _this.initWithThree(three);
            })
                .catch(function (err) {
                console.warn("Failed to load Three.js", err);
            });
            return;
        }
        this.initWithThree(this.three);
    }
    ThreeHost.prototype.reset = function (root) {
        if (!this.three || !this.scene) {
            this.pendingRoot = root;
            return;
        }
        this.clear();
        this.buildFromTree(root);
    };
    ThreeHost.prototype.applyOps = function (ops) {
        var _a;
        if (!this.three || !this.scene) {
            (_a = this.pendingOps).push.apply(_a, ops);
            return;
        }
        for (var _i = 0, ops_1 = ops; _i < ops_1.length; _i++) {
            var op = ops_1[_i];
            this.applyOp(op);
        }
    };
    ThreeHost.prototype.dispose = function () {
        this.stop();
        if (this.resizeObserver) {
            this.resizeObserver.disconnect();
            this.resizeObserver = null;
        }
        this.clear();
        if (this.renderer) {
            this.renderer.dispose();
        }
    };
    ThreeHost.prototype.start = function () {
        var _this = this;
        if (this.running) {
            return;
        }
        this.running = true;
        var loop = function () {
            if (!_this.running) {
                return;
            }
            if (_this.renderer && _this.scene && _this.activeCamera) {
                _this.renderer.render(_this.scene, _this.activeCamera);
            }
            requestAnimationFrame(loop);
        };
        requestAnimationFrame(loop);
    };
    ThreeHost.prototype.initWithThree = function (three) {
        if (this.three && this.scene) {
            return;
        }
        this.three = three;
        var THREE = this.three;
        this.renderer = new THREE.WebGLRenderer({ canvas: this.canvas, antialias: true });
        this.renderer.setPixelRatio(window.devicePixelRatio || 1);
        this.scene = new THREE.Scene();
        this.setupResizeObserver();
        this.start();
        if (this.pendingRoot) {
            var root = this.pendingRoot;
            this.pendingRoot = null;
            this.reset(root);
        }
        if (this.pendingOps.length > 0) {
            var ops = __spreadArray([], this.pendingOps, true);
            this.pendingOps = [];
            this.applyOps(ops);
        }
    };
    ThreeHost.prototype.stop = function () {
        this.running = false;
    };
    ThreeHost.prototype.clear = function () {
        if (!this.scene) {
            return;
        }
        for (var _i = 0, _a = __spreadArray([], this.scene.children, true); _i < _a.length; _i++) {
            var child = _a[_i];
            this.scene.remove(child);
        }
        this.objects.clear();
        this.kinds.clear();
        this.parents.clear();
        this.activeCamera = null;
    };
    ThreeHost.prototype.buildFromTree = function (root) {
        var stack = [
            { node: root, parentId: null },
        ];
        while (stack.length) {
            var entry = stack.shift();
            if (!entry) {
                continue;
            }
            this.createNode(entry.node);
            if (entry.parentId != null) {
                this.attach(entry.parentId, entry.node.id);
            }
            for (var _i = 0, _a = entry.node.children; _i < _a.length; _i++) {
                var child = _a[_i];
                stack.push({ node: child, parentId: entry.node.id });
            }
        }
    };
    ThreeHost.prototype.applyOp = function (op) {
        switch (op.type) {
            case "create":
                this.createNode({
                    id: op.id,
                    kind: op.kind,
                    props: op.props,
                    children: [],
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
    };
    ThreeHost.prototype.createNode = function (node) {
        if (!this.three || !this.scene) {
            return;
        }
        var THREE = this.three;
        var obj = null;
        switch (node.kind) {
            case "scene":
                obj = this.scene;
                break;
            case "group":
                obj = new THREE.Group();
                break;
            case "mesh":
                obj = new THREE.Mesh(new THREE.BoxGeometry(1, 1, 1), new THREE.MeshStandardMaterial({ color: 0xffffff }));
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
            case "meshStandardMaterial":
                obj = new THREE.MeshStandardMaterial({ color: 0xffffff });
                break;
            case "meshBasicMaterial":
                obj = new THREE.MeshBasicMaterial({ color: 0xffffff });
                break;
            case "ambientLight":
                obj = new THREE.AmbientLight(0xffffff, 1);
                break;
            case "directionalLight":
                obj = new THREE.DirectionalLight(0xffffff, 1);
                break;
            case "pointLight":
                obj = new THREE.PointLight(0xffffff, 1);
                break;
        }
        if (!obj) {
            return;
        }
        this.objects.set(node.id, obj);
        this.kinds.set(node.id, node.kind);
        this.parents.set(node.id, null);
        for (var _i = 0, _a = node.props; _i < _a.length; _i++) {
            var prop = _a[_i];
            this.setProp(node.id, prop.key, prop.value);
        }
    };
    ThreeHost.prototype.attach = function (parentId, childId) {
        if (!this.scene || !this.three) {
            return;
        }
        var parent = this.objects.get(parentId);
        var child = this.objects.get(childId);
        if (!parent || !child) {
            return;
        }
        var parentKind = this.kinds.get(parentId);
        var childKind = this.kinds.get(childId);
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
    };
    ThreeHost.prototype.detach = function (parentId, childId) {
        var parent = this.objects.get(parentId);
        var child = this.objects.get(childId);
        if (!parent || !child) {
            return;
        }
        var parentKind = this.kinds.get(parentId);
        var childKind = this.kinds.get(childId);
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
    };
    ThreeHost.prototype.deleteNode = function (id) {
        var obj = this.objects.get(id);
        if (!obj) {
            return;
        }
        var parentId = this.parents.get(id);
        if (parentId != null) {
            this.detach(parentId, id);
        }
        this.objects.delete(id);
        this.kinds.delete(id);
        this.parents.delete(id);
        if (obj.dispose) {
            obj.dispose();
        }
    };
    ThreeHost.prototype.setProp = function (id, key, value) {
        var _a, _b, _c, _d, _e, _f;
        var obj = this.objects.get(id);
        if (!obj) {
            return;
        }
        var THREE = this.three;
        var decoded = decodeValue(value);
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
                    }
                    else if (this.activeCamera === obj) {
                        this.activeCamera = null;
                    }
                }
                return;
        }
        var kind = this.kinds.get(id);
        if (kind === "boxGeometry" && typeof decoded === "number" && obj.parameters) {
            if (key === "width" || key === "height" || key === "depth") {
                var width = key === "width" ? decoded : (_a = obj.parameters.width) !== null && _a !== void 0 ? _a : 1;
                var height = key === "height" ? decoded : (_b = obj.parameters.height) !== null && _b !== void 0 ? _b : 1;
                var depth = key === "depth" ? decoded : (_c = obj.parameters.depth) !== null && _c !== void 0 ? _c : 1;
                this.replaceGeometry(id, new THREE.BoxGeometry(width, height, depth));
            }
            return;
        }
        if (kind === "sphereGeometry" && typeof decoded === "number" && obj.parameters) {
            if (key === "radius" || key === "widthSegments" || key === "heightSegments") {
                var radius = key === "radius" ? decoded : (_d = obj.parameters.radius) !== null && _d !== void 0 ? _d : 1;
                var widthSegments = key === "widthSegments" ? decoded : (_e = obj.parameters.widthSegments) !== null && _e !== void 0 ? _e : 32;
                var heightSegments = key === "heightSegments" ? decoded : (_f = obj.parameters.heightSegments) !== null && _f !== void 0 ? _f : 16;
                this.replaceGeometry(id, new THREE.SphereGeometry(radius, widthSegments, heightSegments));
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
    };
    ThreeHost.prototype.unsetProp = function (id, key) {
        var obj = this.objects.get(id);
        if (!obj) {
            return;
        }
        if (key === "active" && this.activeCamera === obj) {
            this.activeCamera = null;
        }
    };
    ThreeHost.prototype.replaceGeometry = function (id, geometry) {
        var obj = this.objects.get(id);
        if (!obj) {
            return;
        }
        var parentId = this.parents.get(id);
        if (parentId != null) {
            var parent_1 = this.objects.get(parentId);
            if (parent_1 && parent_1.geometry) {
                parent_1.geometry = geometry;
            }
        }
        this.objects.set(id, geometry);
    };
    ThreeHost.prototype.setupResizeObserver = function () {
        var _this = this;
        if (!this.renderer) {
            return;
        }
        var resize = function () {
            if (!_this.renderer || !_this.canvas) {
                return;
            }
            var width = _this.canvas.clientWidth;
            var height = _this.canvas.clientHeight;
            if (width === 0 || height === 0) {
                return;
            }
            _this.renderer.setSize(width, height, false);
            if (_this.activeCamera) {
                if (_this.activeCamera.isPerspectiveCamera) {
                    _this.activeCamera.aspect = width / height;
                    _this.activeCamera.updateProjectionMatrix();
                }
            }
        };
        this.resizeObserver = new ResizeObserver(resize);
        this.resizeObserver.observe(this.canvas);
        resize();
    };
    return ThreeHost;
}());
var decodeValue = function (value) {
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
