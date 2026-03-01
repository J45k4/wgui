const defaultNotificationTitle = () => {
	try {
		const scope = new URL(self.registration.scope)
		if (scope.hostname && scope.hostname.trim().length > 0) {
			return scope.hostname
		}
	} catch (_) {}
	return "Notification"
}

self.addEventListener("install", (event) => {
	event.waitUntil(self.skipWaiting())
})

self.addEventListener("activate", (event) => {
	event.waitUntil(self.clients.claim())
})

self.addEventListener("push", (event) => {
	if (!event.data) {
		return
	}
	const fallbackTitle = defaultNotificationTitle()
	let payload = {}
	try {
		payload = event.data.json()
	} catch (_) {
		payload = {
			title: fallbackTitle,
			body: event.data.text(),
		}
	}
	const title = payload.title || fallbackTitle
	const body = payload.body || ""
	const url = payload.url || "/"
	event.waitUntil(
		self.registration.showNotification(title, {
			body,
			data: { url },
		}),
	)
})

self.addEventListener("notificationclick", (event) => {
	event.notification.close()
	const url = event.notification?.data?.url || "/"
	event.waitUntil(
		self.clients.matchAll({ type: "window", includeUncontrolled: true }).then((clients) => {
			for (const client of clients) {
				if ("focus" in client) {
					client.navigate(url)
					return client.focus()
				}
			}
			return self.clients.openWindow(url)
		}),
	)
})
