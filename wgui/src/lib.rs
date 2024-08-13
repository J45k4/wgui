use gui::Item;
use server::Server;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use types::ClientEvent;
use types::Clients;
use types::Command;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

pub mod gui;
mod edit_distance;
pub mod types;
mod ui_client;
mod diff;
mod server;

pub struct Wgui {
    events_rx: mpsc::UnboundedReceiver<ClientEvent>,
    clients: Clients
}

impl Wgui {
    pub fn new(addr: SocketAddr) -> Self 
    {
        let (events_tx, events_rx) = mpsc::unbounded_channel();
        let clients: Clients = Arc::new(RwLock::new(HashMap::new()));

        {
            let clients = clients.clone();
            tokio::spawn(async move {
                Server::new(addr, events_tx, clients).await.run().await;
            });
        }

        Self {
            events_rx,
            clients
        }
    }

    pub async fn next(&mut self) -> Option<ClientEvent> {
        self.events_rx.recv().await
    }

    pub async fn render(&self, client_id: usize, item: Item) {
        log::debug!("render {:?}", item);
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