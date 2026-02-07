import { ThreeKind, ThreeNode, ThreeOp, ThreePropValue } from "./types.ts";

type ThreeLike = any

let threeLoadPromise: Promise<ThreeLike> | null = null

const getThree = (): ThreeLike | null => {
	const three = (window as any).THREE
	return three ?? null
}

const dynamicImport = (url: string) => {
	const importer = new Function("u", "return import(u)")
	return importer(url) as Promise<any>
}

const loadThree = () => {
	if (threeLoadPromise) {
		return threeLoadPromise
	}
	threeLoadPromise = new Promise((resolve, reject) => {
		const moduleUrl =
			(window as any).WGUI_THREE_MODULE_URL ??
			"https://cdnjs.cloudflare.com/ajax/libs/three.js/0.180.0/three.module.js"

		const sources = [
			(window as any).WGUI_THREE_URL,
			"/three.min.js",
			"https://unpkg.com/three@0.161.0/build/three.min.js",
		].filter(Boolean) as string[]

		const waitForThree = (timeoutMs: number) => {
			const start = Date.now()
			const check = () => {
				const three = getThree()
				if (three) {
					resolve(three)
					return
				}
				if (Date.now() - start > timeoutMs) {
					return
				}
				setTimeout(check, 50)
			}
			check()
		}

		const tryLoad = (index: number) => {
			if (index >= sources.length) {
				reject(new Error("Failed to load Three.js"))
				return
			}
			const src = sources[index]
			const existing = document.querySelector(
				`script[data-wgui-three-src=\"${src}\"]`
			) as HTMLScriptElement | null
			if (existing) {
				waitForThree(1500)
				setTimeout(() => {
					if (!getThree()) {
						tryLoad(index + 1)
					}
				}, 1600)
				return
			}

			const script = document.createElement("script")
			script.src = src
			script.async = true
			script.dataset.wguiThree = "true"
			script.dataset.wguiThreeSrc = src
			script.onload = () => {
				const three = getThree()
				if (three) {
					resolve(three)
				} else {
					tryLoad(index + 1)
				}
			}
			script.onerror = () => {
				tryLoad(index + 1)
			}
			document.head.appendChild(script)
		}

		if (getThree()) {
			resolve(getThree())
			return
		}

		dynamicImport(moduleUrl)
			.then(async (threeModule) => {
				;(window as any).THREE = threeModule
				resolve(threeModule)
			})
			.catch(() => {
				tryLoad(0)
			})
	})
	return threeLoadPromise
}

const hostMap = new WeakMap<HTMLCanvasElement, ThreeHost>()

export const applyThreeTree = (canvas: HTMLCanvasElement, root: ThreeNode) => {
	const host = ensureThreeHost(canvas)
	host.reset(root)
}

export const applyThreePatch = (element: Element, ops: ThreeOp[]) => {
	if (!(element instanceof HTMLCanvasElement)) {
		return
	}
	const host = ensureThreeHost(element)
	host.applyOps(ops)
}

export const disposeThreeHost = (element: Element) => {
	if (!(element instanceof HTMLCanvasElement)) {
		return
	}
	const host = hostMap.get(element)
	if (host) {
		host.dispose()
		hostMap.delete(element)
	}
}

const ensureThreeHost = (canvas: HTMLCanvasElement) => {
	let host = hostMap.get(canvas)
	if (!host) {
		host = new ThreeHost(canvas)
		hostMap.set(canvas, host)
	}
	return host
}

class ThreeHost {
	private canvas: HTMLCanvasElement
	private three: ThreeLike | null
	private renderer: ThreeLike | null
	private scene: ThreeLike | null
	private activeCamera: ThreeLike | null
	private objects: Map<number, ThreeLike>
	private kinds: Map<number, ThreeKind>
	private parents: Map<number, number | null>
	private resizeObserver: ResizeObserver | null
	private running: boolean
	private pendingRoot: ThreeNode | null
	private pendingOps: ThreeOp[]
	private stlLoadTokens: Map<number, number>
	private isMiddlePanning: boolean
	private isRightRotating: boolean
	private panLastX: number
	private panLastY: number
	private rotateLastX: number
	private rotateLastY: number
	private onMouseDown: ((event: MouseEvent) => void) | null
	private onMouseMove: ((event: MouseEvent) => void) | null
	private onMouseUp: ((event: MouseEvent) => void) | null
	private onAuxClick: ((event: MouseEvent) => void) | null
	private onContextMenu: ((event: MouseEvent) => void) | null
	private onWheel: ((event: WheelEvent) => void) | null

	constructor(canvas: HTMLCanvasElement) {
		this.canvas = canvas
		this.three = getThree()
		this.renderer = null
		this.scene = null
		this.activeCamera = null
		this.objects = new Map()
		this.kinds = new Map()
		this.parents = new Map()
		this.resizeObserver = null
		this.running = false
		this.pendingRoot = null
		this.pendingOps = []
		this.stlLoadTokens = new Map()
		this.isMiddlePanning = false
		this.isRightRotating = false
		this.panLastX = 0
		this.panLastY = 0
		this.rotateLastX = 0
		this.rotateLastY = 0
		this.onMouseDown = null
		this.onMouseMove = null
		this.onMouseUp = null
		this.onAuxClick = null
		this.onContextMenu = null
		this.onWheel = null

		if (!this.three) {
			loadThree()
				.then((three) => {
					this.initWithThree(three)
				})
				.catch((err) => {
					console.warn("Failed to load Three.js", err)
				})
			return
		}

		this.initWithThree(this.three)
	}

	reset(root: ThreeNode) {
		if (!this.three || !this.scene) {
			this.pendingRoot = root
			return
		}
		this.clear()
		this.buildFromTree(root)
	}

	applyOps(ops: ThreeOp[]) {
		if (!this.three || !this.scene) {
			this.pendingOps.push(...ops)
			return
		}
		for (const op of ops) {
			this.applyOp(op)
		}
	}

	dispose() {
		this.stop()
		this.teardownPanControls()
		if (this.resizeObserver) {
			this.resizeObserver.disconnect()
			this.resizeObserver = null
		}
		this.clear()
		if (this.renderer) {
			this.renderer.dispose()
		}
	}

	private start() {
		if (this.running) {
			return
		}
		this.running = true
		const loop = () => {
			if (!this.running) {
				return
			}
			if (this.renderer && this.scene && this.activeCamera) {
				this.renderer.render(this.scene, this.activeCamera)
			}
			requestAnimationFrame(loop)
		}
		requestAnimationFrame(loop)
	}

	private initWithThree(three: ThreeLike) {
		if (this.three && this.scene) {
			return
		}
		if (!three.WebGLRenderer) {
			console.error("Loaded THREE module keys:", Object.keys(three))
			throw new Error("Three loaded, but WebGLRenderer missing (wrong build?)")
		}
		this.three = three
		const THREE = this.three
		this.renderer = new THREE.WebGLRenderer({ canvas: this.canvas, antialias: true })
		this.renderer.setPixelRatio(window.devicePixelRatio || 1)
		this.scene = new THREE.Scene()
		this.setupResizeObserver()
		this.setupPanControls()
		this.start()

		if (this.pendingRoot) {
			const root = this.pendingRoot
			this.pendingRoot = null
			this.reset(root)
		}
		if (this.pendingOps.length > 0) {
			const ops = [...this.pendingOps]
			this.pendingOps = []
			this.applyOps(ops)
		}
	}

	private stop() {
		this.running = false
	}

	private clear() {
		if (!this.scene) {
			return
		}
		for (const child of [...this.scene.children]) {
			this.scene.remove(child)
		}
		this.objects.clear()
		this.kinds.clear()
		this.parents.clear()
		this.stlLoadTokens.clear()
		this.activeCamera = null
	}

	private buildFromTree(root: ThreeNode) {
		const stack: Array<{ node: ThreeNode; parentId: number | null }> = [
			{ node: root, parentId: null },
		]
		while (stack.length) {
			const entry = stack.shift()
			if (!entry) {
				continue
			}
			this.createNode(entry.node)
			if (entry.parentId != null) {
				this.attach(entry.parentId, entry.node.id)
			}
			for (const child of entry.node.children) {
				stack.push({ node: child, parentId: entry.node.id })
			}
		}
	}

	private applyOp(op: ThreeOp) {
		switch (op.type) {
			case "create":
				this.createNode({
					id: op.id,
					kind: op.kind,
					props: op.props,
					children: [],
				})
				return
			case "attach":
				this.attach(op.parentId, op.childId)
				return
			case "detach":
				this.detach(op.parentId, op.childId)
				return
			case "setProp":
				this.setProp(op.id, op.key, op.value)
				return
			case "unsetProp":
				this.unsetProp(op.id, op.key)
				return
			case "delete":
				this.deleteNode(op.id)
				return
		}
	}

	private createNode(node: ThreeNode) {
		if (!this.three || !this.scene) {
			return
		}
		const THREE = this.three
		let obj: ThreeLike | null = null
		switch (node.kind) {
			case "scene":
				obj = this.scene
				break
			case "group":
				obj = new THREE.Group()
				break
			case "mesh":
				obj = new THREE.Mesh(
					new THREE.BoxGeometry(1, 1, 1),
					new THREE.MeshStandardMaterial({ color: 0xffffff })
				)
				break
			case "perspectiveCamera":
				obj = new THREE.PerspectiveCamera(50, 1, 0.1, 2000)
				break
			case "orthographicCamera":
				obj = new THREE.OrthographicCamera(-1, 1, 1, -1, 0.1, 2000)
				break
			case "boxGeometry":
				obj = new THREE.BoxGeometry(1, 1, 1)
				break
			case "sphereGeometry":
				obj = new THREE.SphereGeometry(1, 32, 16)
				break
			case "stlGeometry":
				obj = new THREE.BufferGeometry()
				break
			case "meshStandardMaterial":
				obj = new THREE.MeshStandardMaterial({ color: 0xffffff })
				break
			case "meshBasicMaterial":
				obj = new THREE.MeshBasicMaterial({ color: 0xffffff })
				break
			case "ambientLight":
				obj = new THREE.AmbientLight(0xffffff, 1)
				break
			case "directionalLight":
				obj = new THREE.DirectionalLight(0xffffff, 1)
				break
			case "pointLight":
				obj = new THREE.PointLight(0xffffff, 1)
				break
		}
		if (!obj) {
			return
		}
		this.objects.set(node.id, obj)
		this.kinds.set(node.id, node.kind)
		this.parents.set(node.id, null)

		for (const prop of node.props) {
			this.setProp(node.id, prop.key, prop.value)
		}
	}

	private attach(parentId: number, childId: number) {
		if (!this.scene || !this.three) {
			return
		}
		const parent = this.objects.get(parentId)
		const child = this.objects.get(childId)
		if (!parent || !child) {
			return
		}
		const parentKind = this.kinds.get(parentId)
		const childKind = this.kinds.get(childId)
		if (parentKind === "mesh" && childKind) {
			if (childKind.endsWith("Geometry")) {
				parent.geometry = child
				this.parents.set(childId, parentId)
				return
			}
			if (childKind.endsWith("Material")) {
				parent.material = child
				this.parents.set(childId, parentId)
				return
			}
		}
		if (parent.add) {
			parent.add(child)
			this.parents.set(childId, parentId)
		}
	}

	private detach(parentId: number, childId: number) {
		const parent = this.objects.get(parentId)
		const child = this.objects.get(childId)
		if (!parent || !child) {
			return
		}
		const parentKind = this.kinds.get(parentId)
		const childKind = this.kinds.get(childId)
		if (parentKind === "mesh" && childKind) {
			if (childKind.endsWith("Geometry") && parent.geometry === child) {
				parent.geometry = null
				this.parents.set(childId, null)
				return
			}
			if (childKind.endsWith("Material") && parent.material === child) {
				parent.material = null
				this.parents.set(childId, null)
				return
			}
		}
		if (parent.remove) {
			parent.remove(child)
			this.parents.set(childId, null)
		}
	}

	private deleteNode(id: number) {
		const obj = this.objects.get(id)
		if (!obj) {
			return
		}
		const parentId = this.parents.get(id)
		if (parentId != null) {
			this.detach(parentId, id)
		}
		this.objects.delete(id)
		this.kinds.delete(id)
		this.parents.delete(id)
		this.stlLoadTokens.delete(id)
		if (obj.dispose) {
			obj.dispose()
		}
	}

	private setProp(id: number, key: string, value: ThreePropValue) {
		const obj = this.objects.get(id)
		if (!obj) {
			return
		}
		const THREE = this.three
		const decoded = decodeValue(value)
		switch (key) {
			case "position":
				if (decoded && obj.position) {
					obj.position.set(decoded.x, decoded.y, decoded.z)
				}
				return
			case "rotation":
				if (decoded && obj.rotation) {
					obj.rotation.set(decoded.x, decoded.y, decoded.z)
				}
				return
			case "scale":
				if (decoded && obj.scale) {
					obj.scale.set(decoded.x, decoded.y, decoded.z)
				}
				return
			case "lookAt":
				if (decoded && obj.lookAt) {
					obj.lookAt(decoded.x, decoded.y, decoded.z)
				}
				return
			case "visible":
				if (typeof decoded === "boolean") {
					obj.visible = decoded
				}
				return
			case "name":
				if (typeof decoded === "string") {
					obj.name = decoded
				}
				return
			case "castShadow":
				if (typeof decoded === "boolean") {
					obj.castShadow = decoded
				}
				return
			case "receiveShadow":
				if (typeof decoded === "boolean") {
					obj.receiveShadow = decoded
				}
				return
			case "color":
				if (decoded && obj.color) {
					obj.color = new THREE.Color(decoded.r / 255, decoded.g / 255, decoded.b / 255)
				}
				return
			case "intensity":
				if (typeof decoded === "number") {
					obj.intensity = decoded
				}
				return
			case "fov":
				if (typeof decoded === "number") {
					obj.fov = decoded
					if (obj.updateProjectionMatrix) obj.updateProjectionMatrix()
				}
				return
			case "near":
				if (typeof decoded === "number") {
					obj.near = decoded
					if (obj.updateProjectionMatrix) obj.updateProjectionMatrix()
				}
				return
			case "far":
				if (typeof decoded === "number") {
					obj.far = decoded
					if (obj.updateProjectionMatrix) obj.updateProjectionMatrix()
				}
				return
			case "active":
				if (typeof decoded === "boolean") {
					if (decoded) {
						this.activeCamera = obj
					} else if (this.activeCamera === obj) {
						this.activeCamera = null
					}
				}
				return
		}

		const kind = this.kinds.get(id)
		if (kind === "boxGeometry" && typeof decoded === "number" && obj.parameters) {
			if (key === "width" || key === "height" || key === "depth") {
				const width = key === "width" ? decoded : obj.parameters.width ?? 1
				const height = key === "height" ? decoded : obj.parameters.height ?? 1
				const depth = key === "depth" ? decoded : obj.parameters.depth ?? 1
				this.replaceGeometry(id, new THREE.BoxGeometry(width, height, depth))
			}
			return
		}
		if (kind === "sphereGeometry" && typeof decoded === "number" && obj.parameters) {
			if (key === "radius" || key === "widthSegments" || key === "heightSegments") {
				const radius = key === "radius" ? decoded : obj.parameters.radius ?? 1
				const widthSegments =
					key === "widthSegments" ? decoded : obj.parameters.widthSegments ?? 32
				const heightSegments =
					key === "heightSegments" ? decoded : obj.parameters.heightSegments ?? 16
				this.replaceGeometry(id, new THREE.SphereGeometry(radius, widthSegments, heightSegments))
			}
			return
		}
		if (kind === "stlGeometry") {
			if (key === "src" && typeof decoded === "string") {
				this.loadStlGeometry(id, decoded)
			}
			return
		}
		if (kind && kind.endsWith("Material")) {
			if (key === "metalness" && typeof decoded === "number") {
				obj.metalness = decoded
				return
			}
			if (key === "roughness" && typeof decoded === "number") {
				obj.roughness = decoded
				return
			}
			if (key === "wireframe" && typeof decoded === "boolean") {
				obj.wireframe = decoded
				return
			}
			if (key === "opacity" && typeof decoded === "number") {
				obj.opacity = decoded
				obj.transparent = decoded < 1
				return
			}
		}
	}

	private unsetProp(id: number, key: string) {
		const obj = this.objects.get(id)
		if (!obj) {
			return
		}
		if (key === "active" && this.activeCamera === obj) {
			this.activeCamera = null
		}
		if (key === "src" && this.kinds.get(id) === "stlGeometry") {
			this.stlLoadTokens.set(id, (this.stlLoadTokens.get(id) ?? 0) + 1)
		}
	}

	private replaceGeometry(id: number, geometry: ThreeLike) {
		const obj = this.objects.get(id)
		if (!obj) {
			return
		}
		const parentId = this.parents.get(id)
		if (parentId != null) {
			const parent = this.objects.get(parentId)
			if (parent && parent.geometry) {
				parent.geometry = geometry
			}
		}
		this.objects.set(id, geometry)
	}

	private loadStlGeometry(id: number, src: string) {
		if (!this.three) {
			return
		}
		const token = (this.stlLoadTokens.get(id) ?? 0) + 1
		this.stlLoadTokens.set(id, token)

		fetch(src)
			.then((response) => {
				if (!response.ok) {
					throw new Error(`HTTP ${response.status}`)
				}
				return response.arrayBuffer()
			})
			.then((buffer) => {
				if (this.stlLoadTokens.get(id) !== token) {
					return
				}
				const geometry = parseStl(this.three, buffer)
				this.replaceGeometry(id, geometry)
			})
			.catch((err) => {
				if (this.stlLoadTokens.get(id) !== token) {
					return
				}
				console.warn(`Failed to load STL geometry from "${src}"`, err)
			})
	}

	private setupResizeObserver() {
		if (!this.renderer) {
			return
		}
		const resize = () => {
			if (!this.renderer || !this.canvas) {
				return
			}
			const width = this.canvas.clientWidth
			const height = this.canvas.clientHeight
			if (width === 0 || height === 0) {
				return
			}
			this.renderer.setSize(width, height, false)
			if (this.activeCamera) {
				if (this.activeCamera.isPerspectiveCamera) {
					this.activeCamera.aspect = width / height
					this.activeCamera.updateProjectionMatrix()
				}
			}
		}
		this.resizeObserver = new ResizeObserver(resize)
		this.resizeObserver.observe(this.canvas)
		resize()
	}

	private setupPanControls() {
		if (this.onMouseDown || this.onMouseMove || this.onMouseUp || this.onWheel) {
			return
		}
		this.onMouseDown = (event: MouseEvent) => {
			if (event.button !== 1 && event.button !== 2) {
				return
			}
			event.preventDefault()
			if (event.button === 1) {
				this.isMiddlePanning = true
				this.panLastX = event.clientX
				this.panLastY = event.clientY
			}
			if (event.button === 2) {
				this.isRightRotating = true
				this.rotateLastX = event.clientX
				this.rotateLastY = event.clientY
			}
			document.body.style.userSelect = "none"
			document.body.style.cursor = "grabbing"
		}
		this.onMouseMove = (event: MouseEvent) => {
			if (this.isMiddlePanning) {
				event.preventDefault()
				const dx = event.clientX - this.panLastX
				const dy = event.clientY - this.panLastY
				this.panLastX = event.clientX
				this.panLastY = event.clientY
				this.panFromCamera(dx, dy)
			}
			if (this.isRightRotating) {
				event.preventDefault()
				const dx = event.clientX - this.rotateLastX
				const dy = event.clientY - this.rotateLastY
				this.rotateLastX = event.clientX
				this.rotateLastY = event.clientY
				this.rotateFromCamera(dx, dy)
			}
		}
		this.onMouseUp = (event: MouseEvent) => {
			if (event.button === 1) {
				this.isMiddlePanning = false
			}
			if (event.button === 2) {
				this.isRightRotating = false
			}
			this.stopControlsInteraction()
		}

		this.onAuxClick = (event: MouseEvent) => {
			if (event.button === 1) {
				event.preventDefault()
			}
		}
		this.onContextMenu = (event: MouseEvent) => {
			event.preventDefault()
		}
		this.onWheel = (event: WheelEvent) => {
			event.preventDefault()
			this.zoomFromWheel(event.deltaY)
		}
		this.canvas.addEventListener("mousedown", this.onMouseDown)
		window.addEventListener("mousemove", this.onMouseMove)
		window.addEventListener("mouseup", this.onMouseUp)
		this.canvas.addEventListener("auxclick", this.onAuxClick)
		this.canvas.addEventListener("contextmenu", this.onContextMenu)
		this.canvas.addEventListener("wheel", this.onWheel, { passive: false })
	}

	private teardownPanControls() {
		this.isMiddlePanning = false
		this.isRightRotating = false
		this.stopControlsInteraction()
		if (this.onMouseDown) {
			this.canvas.removeEventListener("mousedown", this.onMouseDown)
			this.onMouseDown = null
		}
		if (this.onMouseMove) {
			window.removeEventListener("mousemove", this.onMouseMove)
			this.onMouseMove = null
		}
		if (this.onMouseUp) {
			window.removeEventListener("mouseup", this.onMouseUp)
			this.onMouseUp = null
		}
		if (this.onAuxClick) {
			this.canvas.removeEventListener("auxclick", this.onAuxClick)
			this.onAuxClick = null
		}
		if (this.onContextMenu) {
			this.canvas.removeEventListener("contextmenu", this.onContextMenu)
			this.onContextMenu = null
		}
		if (this.onWheel) {
			this.canvas.removeEventListener("wheel", this.onWheel)
			this.onWheel = null
		}
	}

	private stopControlsInteraction() {
		if (this.isMiddlePanning || this.isRightRotating) {
			return
		}
		document.body.style.userSelect = ""
		document.body.style.cursor = ""
	}

	private panFromCamera(dx: number, dy: number) {
		if (!this.three || !this.activeCamera || !this.activeCamera.position) {
			return
		}
		const THREE = this.three
		const camera = this.activeCamera
		const cameraPos = camera.position
		const distance = Math.max(1, cameraPos.length ? cameraPos.length() : 1)
		const factor = distance / Math.max(this.canvas.clientHeight || 1, 1)
		const right = new THREE.Vector3(1, 0, 0).applyQuaternion(camera.quaternion)
		const up = new THREE.Vector3(0, 1, 0).applyQuaternion(camera.quaternion)
		const delta = right.multiplyScalar(-dx * factor).add(up.multiplyScalar(dy * factor))
		cameraPos.add(delta)
	}

	private zoomFromWheel(deltaY: number) {
		if (!this.three || !this.activeCamera) {
			return
		}
		const THREE = this.three
		const camera = this.activeCamera

		if (camera.isPerspectiveCamera && camera.position) {
			const amount = Math.max(-1, Math.min(1, -deltaY)) * 0.1
			const forward = new THREE.Vector3(0, 0, -1).applyQuaternion(camera.quaternion)
			camera.position.add(forward.multiplyScalar(amount))
			return
		}

		if (camera.isOrthographicCamera) {
			const factor = deltaY > 0 ? 0.9 : 1.1
			camera.zoom = Math.max(0.05, Math.min(100, camera.zoom / factor))
			camera.updateProjectionMatrix()
		}
	}

	private rotateFromCamera(dx: number, dy: number) {
		if (!this.three || !this.activeCamera) {
			return
		}
		const THREE = this.three
		const camera = this.activeCamera
		if (!camera.quaternion) {
			return
		}
		const yaw = -dx * 0.005
		const pitch = -dy * 0.005
		const up = new THREE.Vector3(0, 1, 0)
		const right = new THREE.Vector3(1, 0, 0).applyQuaternion(camera.quaternion)
		camera.quaternion.premultiply(new THREE.Quaternion().setFromAxisAngle(up, yaw))
		camera.quaternion.multiply(new THREE.Quaternion().setFromAxisAngle(right, pitch))
	}
}

const decodeValue = (value: ThreePropValue) => {
	switch (value.type) {
		case "number":
			return value.value
		case "bool":
			return value.value
		case "string":
			return value.value
		case "vec3":
			return { x: value.x, y: value.y, z: value.z }
		case "color":
			return { r: value.r, g: value.g, b: value.b, a: value.a }
	}
}

const parseStl = (THREE: ThreeLike, arrayBuffer: ArrayBuffer) => {
	if (isBinaryStl(arrayBuffer)) {
		return parseBinaryStl(THREE, arrayBuffer)
	}
	return parseAsciiStl(THREE, arrayBuffer)
}

const isBinaryStl = (arrayBuffer: ArrayBuffer) => {
	if (arrayBuffer.byteLength < 84) {
		return false
	}
	const dataView = new DataView(arrayBuffer)
	const triangleCount = dataView.getUint32(80, true)
	const expected = 84 + triangleCount * 50
	return expected === arrayBuffer.byteLength
}

const parseBinaryStl = (THREE: ThreeLike, arrayBuffer: ArrayBuffer) => {
	const dataView = new DataView(arrayBuffer)
	const triangleCount = dataView.getUint32(80, true)
	const positions: number[] = []
	const normals: number[] = []
	let offset = 84

	for (let i = 0; i < triangleCount; i++) {
		const nx = dataView.getFloat32(offset, true)
		const ny = dataView.getFloat32(offset + 4, true)
		const nz = dataView.getFloat32(offset + 8, true)
		offset += 12

		for (let v = 0; v < 3; v++) {
			const x = dataView.getFloat32(offset, true)
			const y = dataView.getFloat32(offset + 4, true)
			const z = dataView.getFloat32(offset + 8, true)
			positions.push(x, y, z)
			normals.push(nx, ny, nz)
			offset += 12
		}

		offset += 2
	}

	return buildGeometry(THREE, positions, normals)
}

const parseAsciiStl = (THREE: ThreeLike, arrayBuffer: ArrayBuffer) => {
	const text = new TextDecoder().decode(arrayBuffer)
	const positions: number[] = []
	const normals: number[] = []
	const facetRegex =
		/facet\s+normal\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+outer\s+loop\s+vertex\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+vertex\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+vertex\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+([+-]?\d*\.?\d+(?:[eE][+-]?\d+)?)\s+endloop\s+endfacet/gi

	let match: RegExpExecArray | null
	while ((match = facetRegex.exec(text)) !== null) {
		const numbers = match.slice(1).map(Number)
		const [nx, ny, nz, x1, y1, z1, x2, y2, z2, x3, y3, z3] = numbers
		positions.push(x1, y1, z1, x2, y2, z2, x3, y3, z3)
		normals.push(nx, ny, nz, nx, ny, nz, nx, ny, nz)
	}

	return buildGeometry(THREE, positions, normals)
}

const buildGeometry = (THREE: ThreeLike, positions: number[], normals: number[]) => {
	const geometry = new THREE.BufferGeometry()
	geometry.setAttribute("position", new THREE.Float32BufferAttribute(positions, 3))
	if (normals.length === positions.length) {
		geometry.setAttribute("normal", new THREE.Float32BufferAttribute(normals, 3))
	} else {
		geometry.computeVertexNormals()
	}
	if (geometry.computeBoundingSphere) {
		geometry.computeBoundingSphere()
	}
	return geometry
}
