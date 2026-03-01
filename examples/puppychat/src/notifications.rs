use async_trait::async_trait;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NotificationChannel {
	Web,
	Android,
	Ios,
}

impl NotificationChannel {
	pub fn as_str(&self) -> &'static str {
		match self {
			Self::Web => "web",
			Self::Android => "android",
			Self::Ios => "ios",
		}
	}

	pub fn from_str(value: &str) -> Option<Self> {
		match value {
			"web" => Some(Self::Web),
			"android" => Some(Self::Android),
			"ios" => Some(Self::Ios),
			_ => None,
		}
	}
}

#[derive(Debug, Clone)]
pub struct PushTarget {
	pub channel: NotificationChannel,
	pub endpoint: String,
	pub p256dh_key: String,
	pub auth_key: String,
}

#[derive(Debug, Clone)]
pub struct PushPayload {
	pub title: String,
	pub body: String,
	pub thread: String,
}

#[async_trait]
pub trait PushProvider: Send + Sync {
	fn channel(&self) -> NotificationChannel;
	async fn send(&self, target: &PushTarget, payload: &PushPayload) -> Result<(), String>;
}

#[derive(Default)]
pub struct PushService {
	providers: HashMap<NotificationChannel, Arc<dyn PushProvider>>,
}

impl PushService {
	pub fn with_default_providers() -> Self {
		let mut out = Self::default();
		out.register_provider(Arc::new(WebPushProvider::default()));
		out
	}

	pub fn register_provider(&mut self, provider: Arc<dyn PushProvider>) {
		self.providers.insert(provider.channel(), provider);
	}

	pub async fn send(&self, target: &PushTarget, payload: &PushPayload) -> Result<(), String> {
		let Some(provider) = self.providers.get(&target.channel) else {
			return Err(format!(
				"no push provider registered for channel {}",
				target.channel.as_str()
			));
		};
		provider.send(target, payload).await
	}
}

#[derive(Default)]
pub struct WebPushProvider {}

#[async_trait]
impl PushProvider for WebPushProvider {
	fn channel(&self) -> NotificationChannel {
		NotificationChannel::Web
	}

	async fn send(&self, target: &PushTarget, payload: &PushPayload) -> Result<(), String> {
		// This is intentionally provider-shaped so Android/iOS native channels can plug in later.
		// A real Web Push implementation can replace this with VAPID+encrypted payload delivery.
		log::info!(
			"web-push queued endpoint={} p256dh={} auth={} thread={} title={} body={}",
			target.endpoint,
			!target.p256dh_key.is_empty(),
			!target.auth_key.is_empty(),
			payload.thread,
			payload.title,
			payload.body
		);
		Ok(())
	}
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct WebPushKeys {
	#[serde(default)]
	pub p256dh: String,
	#[serde(default)]
	pub auth: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct WebPushSubscription {
	#[serde(default)]
	pub endpoint: String,
	#[serde(default)]
	pub keys: WebPushKeys,
}
