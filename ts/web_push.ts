import { MessageSender } from "./message_sender.ts"
import { WebPushDisable, WebPushEnable } from "./types.ts"

const decodeBase64Url = (value: string) => {
	const padding = "=".repeat((4 - (value.length % 4 || 4)) % 4)
	const base64 = (value + padding).replace(/-/g, "+").replace(/_/g, "/")
	const bytes = atob(base64)
	const out = new Uint8Array(bytes.length)
	for (let i = 0; i < bytes.length; i += 1) {
		out[i] = bytes.charCodeAt(i)
	}
	return out
}

const webPushSupported = () =>
	"Notification" in window && "serviceWorker" in navigator && "PushManager" in window

const pushSubscriptionToServer = (sender: MessageSender, subscription: PushSubscription | null) => {
	const payload = subscription ? (subscription.toJSON() as { [key: string]: unknown }) : null
	sender.send({
		type: "webPushSubscriptionChanged",
		subscription: payload,
	})
	sender.sendNow()
}

const normalizeVapidPublicKey = (value?: string | null) => {
	if (!value) {
		return undefined
	}
	const trimmed = value.trim()
	if (trimmed.length === 0) {
		return undefined
	}
	return trimmed
}

export const enableWebPush = async (sender: MessageSender, message: WebPushEnable) => {
	if (!webPushSupported()) {
		pushSubscriptionToServer(sender, null)
		return
	}
	const permission = await Notification.requestPermission()
	if (permission !== "granted") {
		pushSubscriptionToServer(sender, null)
		return
	}
	const registration = await navigator.serviceWorker.register(message.serviceWorkerPath)
	let subscription = await registration.pushManager.getSubscription()
	if (!subscription) {
		const opts: PushSubscriptionOptionsInit = {
			userVisibleOnly: true,
		}
		const key = normalizeVapidPublicKey(message.vapidPublicKey)
		if (key) {
			opts.applicationServerKey = decodeBase64Url(key)
		}
		subscription = await registration.pushManager.subscribe(opts)
	}
	pushSubscriptionToServer(sender, subscription)
}

export const disableWebPush = async (sender: MessageSender, message: WebPushDisable) => {
	if (!webPushSupported()) {
		pushSubscriptionToServer(sender, null)
		return
	}
	const registration = await navigator.serviceWorker.getRegistration(message.serviceWorkerPath)
	if (!registration) {
		pushSubscriptionToServer(sender, null)
		return
	}
	const subscription = await registration.pushManager.getSubscription()
	await subscription?.unsubscribe()
	pushSubscriptionToServer(sender, null)
}
