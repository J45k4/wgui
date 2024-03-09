use std::vec;

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
    let mut wgui = Wgui::new();
    while let Some(event) = wgui.next().await {
        println!("{:?}", event);

        match event {
            ClientEvent::Disconnected => {},
            ClientEvent::Connected { id } => {
                wgui.render(id, render());
            },
            ClientEvent::PathChanged(_) => {},
            ClientEvent::Input(q) => {}
        }
    }
}
