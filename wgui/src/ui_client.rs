use std::{sync::Arc, pin::Pin, task::{Context, Poll}, collections::HashMap};

use futures_util::Stream;
use hyper::upgrade::Upgraded;
use hyper_tungstenite::{HyperWebsocket, tungstenite::Message, WebSocketStream};
use hyper_util::rt::TokioIo;
use tokio::sync::{mpsc, Mutex};
use futures_util::StreamExt;
use futures_util::SinkExt;

use crate::diff::diff;

use super::{types::ClientEvent, gui::Item};

#[derive(Debug)]
enum Command {
    Render(Item),
    Navigate(String),
    SetQuery(HashMap<String, String>),
}

pub struct ClientRenderer {
    cmd_sender: mpsc::UnboundedSender<Command>
}

impl ClientRenderer {
    pub async fn render(&self, root: Item) {
        log::info!("render root");

        self.cmd_sender.send(
            Command::Render(root)
        ).unwrap();
    }
}

pub struct ClientWriter {
    id: usize,
    cmd_sender: mpsc::UnboundedSender<Command>,
}

impl Clone for ClientWriter {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            cmd_sender: self.cmd_sender.clone()
        }
    }
}

impl Drop for ClientWriter {
    fn drop(&mut self) {
        log::info!("dropping ClientWriter");
    }
}

impl ClientWriter {
    pub async fn render(&self, root: Item) {
        log::info!("render root");

        self.cmd_sender.send(
            Command::Render(root)
        ).unwrap();
    }

    pub async fn navigate(&self, url: String) {
        log::info!("navigate to {}", url);

        self.cmd_sender.send(
            Command::Navigate(url)
        ).unwrap();
    }

    pub fn set_query(&self, map: HashMap<String, String>) {
        self.cmd_sender.send(
            Command::SetQuery(map)
        ).unwrap();
    }

    pub fn id(&self) -> usize {
        self.id
    }
}

pub struct ClientReceiver {
    id: usize,
    event_receiver: mpsc::UnboundedReceiver<ClientEvent>
}

impl Stream for ClientReceiver {
    type Item = (usize, ClientEvent);

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        let r = self.event_receiver.poll_recv(cx);

        match r {
            Poll::Ready(Some(event)) => Poll::Ready(Some((self.id, event))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending
        }
    }
}

impl ClientReceiver {
    pub async fn recv(&mut self) -> Option<ClientEvent> {
        self.event_receiver.recv().await
    }
}

#[derive(Debug)]
pub struct Client {
    id: usize,
    cmd_sender: mpsc::UnboundedSender<Command>,
    event_receiver: mpsc::UnboundedReceiver<ClientEvent>
}

impl Client {
    pub fn id(&self) -> usize {
        self.id
    }

    pub async fn get_renderer(&self) -> ClientRenderer {
        ClientRenderer {
            cmd_sender: self.cmd_sender.clone()
        }
    }

    pub async fn render(&self, root: Item) {
        log::info!("render root");

        self.cmd_sender.send(Command::Render(root)).unwrap();
    }

    pub async fn next(&mut self) -> Option<ClientEvent> {
        self.event_receiver.recv().await
    }

    pub fn split(self) -> (ClientWriter, ClientReceiver) {
        (
            ClientWriter { 
                id: self.id,
                cmd_sender: self.cmd_sender 
            }, 
            ClientReceiver {
                id: self.id,
                event_receiver: self.event_receiver
            }
        )
    }
}

struct Worker {
    ws: WebSocketStream<TokioIo<Upgraded>>,
    cmd_recv: mpsc::UnboundedReceiver<Command>,
    event_sender: mpsc::UnboundedSender<ClientEvent>,
    last_root: Option<Item>
}

impl Worker {
    pub async fn handle_websocket(&mut self, msg: Message) -> anyhow::Result<()> {
        match msg {
            Message::Text(msg) => {
                log::info!("recieved message: {}", msg);

                let msgs: Vec<ClientEvent> = serde_json::from_str(&msg)?;

                log::info!("received messages: {:?}", msgs);

                for msg in msgs {
                    self.event_sender.send(msg).unwrap();
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
        match cmd {
            Command::Render(root) => {
                log::info!("rendering root: {:?}", root);

                let changes = match &self.last_root {
                    Some(last_root) => {
                        let changes = diff(&last_root, &root);
                        changes
                    },
                    // None => vec![ClientAction::Replace(Replace { path: vec![], item: root.clone() })]
                    None => vec![]
                };

                if changes.len() == 0 {
                    return Ok(());
                }

                self.last_root = Some(root);
            
                log::info!("sending changes: {:?}", changes);
            
                let str = serde_json::to_string(&changes).unwrap();
            
                self.ws.send(Message::text(str)).await?;
            }
            Command::Navigate(url) => {
                // let changes = vec![
                //     ClientAction::PushState(
                //         crate::PushState { url: url.clone() }
                //     )
                // ];
                // let changes = vec![];
                // let msg = serde_json::to_string(&changes)?;
                // self.ws.send(Message::text(msg)).await?;
            }
            Command::SetQuery(query) => {
                // let changes = vec![
                //     ClientAction::SetQuery(
                //         SetQuery {
                //             query: query.clone()
                //         }
                //     )
                // ];
                // let changes = vec![];
                // let msg = serde_json::to_string(&changes)?;
                // self.ws.send(Message::text(msg)).await?;
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
    }
}


pub fn create_ui_client(id: usize, websocket: HyperWebsocket) -> Client {
    let (event_sender, event_receiver) = mpsc::unbounded_channel();
    let (cmd_sender, cmd_receiver) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        let ws = websocket.await.unwrap();

        Worker { 
            ws: ws,
            cmd_recv: cmd_receiver,
            event_sender: event_sender,
            last_root: None
        }.run().await;
    });

    Client {
        id: id,
        cmd_sender: cmd_sender,
        event_receiver: event_receiver
    }
}