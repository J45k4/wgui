import { Context, Item } from "./types.ts";

type CustomPayload = {
	type: "custom"
	name: string
	entry: string
	props: unknown
}

type ControllerContext = {
	id: number
	inx?: number
	name: string
	emit: (name: string, payload?: unknown) => void
}

type Controller = {
	mount?: (props: unknown) => void | Promise<void>
	setProps?: (props: unknown) => void | Promise<void>
	dispose?: () => void
}

type ControllerCtor = new (element: HTMLElement, ctx: ControllerContext) => Controller

type ComponentModule = {
	default?: ControllerCtor
	Controller?: ControllerCtor
}

type CustomState = {
	key: string
	props: unknown
	controller?: Controller
	cancelled: boolean
}

const modules = new Map<string, Promise<ComponentModule>>()

const componentKey = (payload: CustomPayload): string => `${payload.name}\n${payload.entry}`

const loadModule = (entry: string): Promise<ComponentModule> => {
	if (!modules.has(entry)) {
		modules.set(entry, import(entry) as Promise<ComponentModule>)
	}
	return modules.get(entry)!
}

const getState = (element: HTMLElement): CustomState | undefined =>
	(element as any).__wguiCustomState as CustomState | undefined

const setState = (element: HTMLElement, state: CustomState | undefined) => {
	if (state) {
		;(element as any).__wguiCustomState = state
	} else {
		delete (element as any).__wguiCustomState
	}
}

const controllerContext = (item: Item, payload: CustomPayload, ctx: Context): ControllerContext => ({
	id: item.id,
	inx: item.inx,
	name: payload.name,
	emit: (name: string, eventPayload?: unknown) => {
		if (!item.id) {
			return
		}
		ctx.sender.send({
			type: "onCustom",
			id: item.id,
			inx: item.inx,
			name,
			payload: eventPayload ?? null,
		})
		ctx.sender.sendNow()
	},
})

const disposeState = (state: CustomState | undefined) => {
	if (!state) {
		return
	}
	state.cancelled = true
	try {
		state.controller?.dispose?.()
	} catch (err) {
		console.warn("wgui custom component dispose failed", err)
	}
}

export const disposeCustomComponent = (element: Element | null | undefined) => {
	if (!(element instanceof HTMLElement)) {
		return
	}
	disposeState(getState(element))
	setState(element, undefined)
}

export const disposeCustomComponentTree = (element: Element | null | undefined) => {
	if (!(element instanceof HTMLElement)) {
		return
	}
	disposeCustomComponent(element)
	for (const child of Array.from(element.children)) {
		disposeCustomComponentTree(child)
	}
}

export const mountCustomComponent = (element: HTMLElement, item: Item, payload: CustomPayload, ctx: Context) => {
	const key = componentKey(payload)
	const existing = getState(element)

	if (existing?.key === key) {
		existing.props = payload.props
		if (existing.controller?.setProps) {
			Promise.resolve(existing.controller.setProps(payload.props)).catch((err) => {
				console.warn("wgui custom component setProps failed", err)
			})
		}
		return
	}

	disposeState(existing)

	const state: CustomState = {
		key,
		props: payload.props,
		cancelled: false,
	}
	setState(element, state)

	loadModule(payload.entry)
		.then((module) => {
			if (state.cancelled) {
				return
			}
			const Controller = module.default ?? module.Controller
			if (!Controller) {
				throw new Error(`custom component ${payload.name} does not export a controller`)
			}
			const controller = new Controller(element, controllerContext(item, payload, ctx))
			state.controller = controller
			return Promise.resolve(controller.mount?.(state.props)).then(() => {
				if (!state.cancelled && !controller.mount && controller.setProps) {
					return controller.setProps(state.props)
				}
			})
		})
		.catch((err) => {
			console.error(`wgui custom component ${payload.name} failed`, err)
			if (!state.cancelled) {
				element.textContent = `Failed to load component ${payload.name}`
			}
		})
}
