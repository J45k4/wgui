use super::{gui::Item, types::ClientEvent};
use crate::{
	diff::diff,
	types::{ClientAction, Clients, Command, Replace},
	ws::{WsMessage, WsStream},
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;

pub struct UiWsWorker<S>
where
	S: WsStream,
{
	id: usize,
	ws: S,
	event_tx: mpsc::UnboundedSender<ClientEvent>,
	cmd_recv: mpsc::UnboundedReceiver<Command>,
	clients: Clients,
	last_root: Option<Item>,
}

impl<S> UiWsWorker<S>
where
	S: WsStream,
{
	pub async fn new(
		id: usize,
		ws: S,
		event_tx: mpsc::UnboundedSender<ClientEvent>,
		clients: Clients,
	) -> Self {
		log::info!("[{}] connection started", id);
		let (cmd_sender, cmd_recv) = mpsc::unbounded_channel();
		clients.write().await.insert(id, cmd_sender);
		event_tx.send(ClientEvent::Connected { id }).unwrap();
		Self {
			id,
			ws,
			cmd_recv: cmd_recv,
			event_tx,
			last_root: None,
			clients,
		}
	}

	pub async fn handle_websocket(&mut self, msg: WsMessage) -> anyhow::Result<()> {
		match msg {
			WsMessage::Text(msg) => {
				log::info!("recieved message: {}", msg);

				let msgs: Vec<ClientEvent> = serde_json::from_str(&msg)?;

				log::info!("received messages: {:?}", msgs);

				for msg in msgs {
					self.event_tx.send(msg).unwrap();
				}
			}
			WsMessage::Binary(msg) => {
				println!("Received binary message: {:02X?}", msg);
				self.ws
					.send(WsMessage::Binary(b"Thank you, come again.".to_vec()))
					.await?;
			}
			WsMessage::Ping(msg) => {
				log::info!("Received ping message: {:02X?}", msg);
			}
			WsMessage::Pong(msg) => {
				log::info!("Received pong message: {:02X?}", msg);
			}
			WsMessage::Close => {
				println!("Received close message");
			}
		};

		Ok(())
	}

	async fn handle_command(&mut self, cmd: Command) -> anyhow::Result<()> {
		log::debug!("handling command: {:?}", cmd);
		match cmd {
			Command::Render(root) => {
				let changes = match &self.last_root {
					Some(last_root) => {
						let changes = diff(&last_root, &root);
						changes
					}
					None => vec![ClientAction::Replace(Replace {
						path: vec![],
						item: root.clone(),
					})],
				};
				if changes.len() == 0 {
					return Ok(());
				}
				self.last_root = Some(root);
				log::debug!("sending changes: {:?}", changes);
				let str = serde_json::to_string(&changes).unwrap();
				self.ws.send(WsMessage::Text(str)).await?;
			}
			Command::SetTitle(title) => {
				let changes = vec![ClientAction::SetTitle { title }];
				let str = serde_json::to_string(&changes).unwrap();
				self.ws.send(WsMessage::Text(str)).await?;
			}
		};

		Ok(())
	}

	pub async fn run(mut self) {
		loop {
			tokio::select! {
				msg = self.ws.next() => {
					match msg {
						Some(msg) => match msg {
							Ok(msg) => {
								match self.handle_websocket(msg).await {
									Ok(_) => {},
									Err(err) => {
										log::error!("Error handling websocket message: {}", err);
									},
								}
							},
							Err(err) => {
								log::error!("Error receiving websocket message: {}", err);

								break;
							},
						},
						None => {
							log::error!("Websocket closed");

							break;
						},
					}
				}
				cmd = self.cmd_recv.recv() => {
					match cmd {
						Some(cmd) => {
							match self.handle_command(cmd).await {
								Ok(_) => {},
								Err(err) => {
									log::error!("Error handling command: {}", err);
								}
							}
						}
						None => {
							log::error!("Command channel closed");

							break;
						}
					}
				}
			};
		}

		log::info!("[{}] connection closed", self.id);
		self.clients.write().await.remove(&self.id);
		self.event_tx
			.send(ClientEvent::Disconnected { id: self.id })
			.unwrap();
	}
}
