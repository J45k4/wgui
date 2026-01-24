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
- WUI template example with hot reload: `cargo run -p todo_wui`

## WUI templates

See `docs/wui.md` for the WUI template language, compiler/runtime usage, and hot reload notes.

## LSP

See `docs/lsp.md` for setting up the `wui-lsp` server in Zed or other editors.

## API overview

- Core runtime: `Wgui::new(addr)`, `wgui.next().await`, `wgui.render(client_id, item)`
- SSR snapshot: `Wgui::new_with_ssr(addr, || render())` or `axum::router_with_ssr(...)`
- Events: `ClientEvent::{Connected, Disconnected, OnClick, OnTextChanged, OnSliderChange, OnSelect, PathChanged}`

Component builders

- Layout: `vstack`, `hstack`
- Text: `text`
- Inputs: `text_input`, `textarea`, `select` + `option`, `checkbox`, `slider`
- Actions: `button`
- Table: `table`, `thead`, `tbody`, `tr`, `th`, `td`
- Media: `img`
- Overlays: `modal`
- Misc: `folder_picker`

Item modifiers

- Identity: `.id(u32)`, `.inx(u32)`
- Value helpers: `.svalue(&str)`, `.ivalue(i32)`, `.checked(bool)`, `.placeholder(&str)`, `.min(i32)`, `.max(i32)`, `.step(i32)`, `.open(bool)`
- Layout/style: `.spacing(u32)`, `.wrap(bool)`, `.grow(u32)`, `.width(u32)`, `.min_width(u32)`, `.max_width(u32)`, `.height(u32)`, `.min_height(u32)`, `.max_height(u32)`
- Box model: `.margin(u16)`, `.margin_left(u16)`, `.margin_right(u16)`, `.margin_top(u16)`, `.margin_bottom(u16)`, `.padding(u16)`, `.padding_left(u16)`, `.padding_right(u16)`, `.padding_top(u16)`, `.padding_bottom(u16)`
- Visuals: `.border(&str)`, `.background_color(&str)`, `.text_align(&str)`, `.cursor(&str)`, `.overflow(&str)`, `.editable(bool)`, `.hresize(bool)`, `.vresize(bool)`

## Development

```
# Build
bun build ./ts/app.ts --watch --outfile ./dist/index.js
# Check 
bunx tsc ./ts/* --noEmit --allowImportingTsExtensions
```
