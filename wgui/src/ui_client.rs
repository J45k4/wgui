use std::{sync::Arc, pin::Pin, task::{Context, Poll}, collections::HashMap};

use futures_util::Stream;
use hyper::upgrade::Upgraded;
use hyper_tungstenite::{HyperWebsocket, tungstenite::Message, WebSocketStream};
use hyper_util::rt::TokioIo;
use tokio::sync::{mpsc, Mutex, RwLock};
use futures_util::StreamExt;
use futures_util::SinkExt;
use crate::{diff::diff, types::{ClientAction, Clients, Command, Replace}};
use super::{types::ClientEvent, gui::Item};

pub struct UiWsWorker {
    id: usize,
    ws: WebSocketStream<TokioIo<Upgraded>>,
    event_tx: mpsc::UnboundedSender<ClientEvent>,
    cmd_recv: mpsc::UnboundedReceiver<Command>,
    clients: Clients,
    last_root: Option<Item>
}

impl UiWsWorker {
    pub async fn new(id: usize, 
        ws: WebSocketStream<TokioIo<Upgraded>>, 
        event_tx: mpsc::UnboundedSender<ClientEvent>,
        clients: Clients
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
            clients
        }
    }

    pub async fn handle_websocket(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::Text(msg) => {
                log::info!("recieved message: {}", msg);

                let msgs: Vec<ClientEvent> = serde_json::from_str(&msg)?;

                log::info!("received messages: {:?}", msgs);

                for msg in msgs {
                    self.event_tx.send(msg).unwrap();
                }
            },
            Message::Binary(msg) => {
                println!("Received binary message: {:02X?}", msg);
                self.ws.send(Message::binary(b"Thank you, come again.".to_vec())).await?;
            },
            Message::Ping(msg) => {
                // No need to send a reply: tungstenite takes care of this for you.
                log::info!("Received ping message: {:02X?}", msg);
            },
            Message::Pong(msg) => {
                log::info!("Received pong message: {:02X?}", msg);
            }
            Message::Close(msg) => {
                // No need to send a reply: tungstenite takes care of this for you.
                if let Some(msg) = &msg {
                    println!("Received close message with code {} and message: {}", msg.code, msg.reason);
                } else {
                    println!("Received close message");
                }
            },
            Message::Frame(msg) => {
               unreachable!();
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
                    },
                    None => vec![ClientAction::Replace(Replace { path: vec![], item: root.clone() })]
                };

                if changes.len() == 0 {
                    return Ok(());
                }

                self.last_root = Some(root);
            
                log::debug!("sending changes: {:?}", changes);
            
                let str = serde_json::to_string(&changes).unwrap();
            
                self.ws.send(Message::text(str)).await?;
            }
            // Command::Navigate(url) => {
            //     // let changes = vec![
            //     //     ClientAction::PushState(
            //     //         crate::PushState { url: url.clone() }
            //     //     )
            //     // ];
            //     // let changes = vec![];
            //     // let msg = serde_json::to_string(&changes)?;
            //     // self.ws.send(Message::text(msg)).await?;
            // }
            // Command::SetQuery(query) => {
            //     // let changes = vec![
            //     //     ClientAction::SetQuery(
            //     //         SetQuery {
            //     //             query: query.clone()
            //     //         }
            //     //     )
            //     // ];
            //     // let changes = vec![];
            //     // let msg = serde_json::to_string(&changes)?;
            //     // self.ws.send(Message::text(msg)).await?;
            // }
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
        self.event_tx.send(ClientEvent::Disconnected { id: self.id }).unwrap();
    }
}