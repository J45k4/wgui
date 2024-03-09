use std::vec;

use log::Level;
use wgui::gui::Item;
use wgui::gui::Text;
use wgui::gui::View;
use wgui::types::ClientEvent;
use wgui::Wgui;

fn render() -> Item {
    Item::View(View {
        body: vec![
            Item::Text(Text {
                text: "Hello, World".to_string()
            }),
        ],
        ..Default::default()
    })
}

#[tokio::main]
async fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();

    let mut wgui = Wgui::new();
    while let Some(event) = wgui.next().await {
        println!("{:?}", event);

        match event {
            ClientEvent::Disconnected { id } => {
                println!("disconnected: {}", id);
            },
            ClientEvent::Connected { id } => {
                println!("connected: {}", id);
                wgui.render(id, render());
            },
            ClientEvent::PathChanged(_) => {},
            ClientEvent::Input(q) => {}
        }
    }
}
