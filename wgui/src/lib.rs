use gui::Item;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use types::ClientEvent;
use types::Clients;
use types::Command;
use std::collections::HashMap;
use std::sync::Arc;

pub mod gui;
mod edit_distance;
pub mod types;
mod ui_client;
mod diff;
mod server;

pub struct WguiBuilder {
    port: u16
}

impl WguiBuilder {
    pub fn new() -> Self {
        Self {
            port: 4477
        }
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn build(self) -> Wgui {
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

        {
            let clients = clients.clone();
            tokio::spawn(async move {
                server::server(self.port, events_tx, clients).await;
            });
        }

        Wgui {
            events_rx,
            clients
        }
    }
}

pub struct Wgui {
    events_rx: mpsc::UnboundedReceiver<ClientEvent>,
    clients: Clients
}

impl Wgui {
    pub async fn next(&mut self) -> Option<ClientEvent> {
        self.events_rx.recv().await
    }

    pub async fn render(&self, client_id: usize, item: Item) {
        log::info!("render {:?}", item);
        let clients = self.clients.read().await;
        let sender = match clients.get(&client_id) {
            Some(sender) => sender,
            None => {
                println!("client not found");
                return;
            }
        };
        sender.send(Command::Render(item)).unwrap();
    }
}