# wgui

Ever wondered that you would like to make web gui with rust and server-side virtual dom... probably not but here it is.

## Quick start

```rust
use log::Level;
use std::collections::HashSet;
use wgui::*;

fn render() -> Item {
	vstack([text("Hello wgui")]).into()
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();
	let mut wgui = Wgui::new("0.0.0.0:12345".parse().unwrap());
	let mut client_ids = HashSet::new();

	while let Some(event) = wgui.next().await {
		match event {
			ClientEvent::Disconnected { id } => {
				client_ids.remove(&id);
			}
			ClientEvent::Connected { id } => {
				wgui.render(id, render()).await;
				client_ids.insert(id);
			}
			_ => {}
		}

		for id in &client_ids {
			wgui.render(*id, render()).await;
		}
	}
}
```

## Examples

- Todo list app (interactive text input + checkbox): `cargo run -p todo`
- Component showcase (text input, select, slider, modal, table): `cargo run -p allcomponents`

## Development

```
# Build
bun build ./ts/app.ts --watch --outfile ./dist/index.js
# Check 
bunx tsc ./ts/* --noEmit --allowImportingTsExtensions
```
