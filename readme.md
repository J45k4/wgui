# wgui

Ever wondered that you would like to make web gui with rust and server-side virtual dom... probably not but here it is.

## Quick start

```rust
use log::Level;
use std::sync::Arc;
use wgui::wui::runtime::Ctx;
use wgui::{route, text, vstack, View, Wgui};

struct AppState;

#[route("/")]
fn home(_ctx: &Ctx<AppState>) -> View {
	View::page("Hello", vstack([text("Hello wgui")]))
}

#[tokio::main]
async fn main() {
	simple_logger::init_with_level(Level::Info).unwrap();
	let ctx = Arc::new(Ctx::new(AppState));
	let mut wgui = Wgui::new("0.0.0.0:12345".parse().unwrap());
	wgui.set_ctx(ctx);
	wgui.add_route(home_route);
	wgui.run().await;
}
```

`#[route]` handlers receive path parameters by name and return `View` for a
render or `Redirect` after a mutation. `POST` handlers may take one typed
`#[derive(serde::Deserialize)]` form argument. The older
`#[wgui_controller]` + `add_page` lifecycle API remains supported for
non-form websocket events and lifecycle hooks.

### Convention-based WUI route views

For a WUI-backed GET route, add the `view` option and return `view!`. The
route selects the template under `wui/pages`; the macro supplies the value
available as `state` in that template.

```rust
use wgui::{route, view, View};

#[route("/todos/:id", view)]
fn todo(_ctx: &Ctx<AppState>, id: u32) -> View {
	view!({
		title: format!("Todo {id}"),
		filters: { completed: false },
	})
}
```

The standard lookup is `/todos` → `wui/pages/todos/index.wui`,
`/todos/:id` → `wui/pages/todos/show.wui`, `/todos/:id/edit` →
`wui/pages/todos/edit.wui`, and `/*` → `wui/pages/not_found.wui`. Use
`template = "path/inside/wui"` to override a route's template. `view!` also
accepts an existing value implementing `WuiValueConvert`.

## Examples

- Todo list app (interactive text input + checkbox): `cargo run -p todo`
- Component showcase (text input, date picker, select, slider, modal, table): `cargo run -p allcomponents`
- WUI template examples: `cargo run -p todo_wui` and `cargo run -p puppychat`

## WUI templates

See `docs/wui.md` for the WUI template language, compiler/runtime usage, and hot reload notes.

`#[wgui_controller]` uses `mode = "auto"` by default: debug builds load `.wui` files from the app `wui/` directory for hot reload, while release builds embed the validated WUI source graph into the binary so the templates do not need to exist on disk. Use `mode = "runtime"` to always read files or `mode = "compiled"` to always embed templates.

Validate templates from the CLI before starting an app:

```
cargo run -p wgui -- check path/to/project
cargo run -p wgui -- check path/to/template.wui
```

## Database schemas and migrations

Projects can configure `schema`, `db`, `migrations_dir`, and `env_file` in `wgui.toml`. The
schema defaults to `schema.wdb`; SQLite migration commands use the configured database path (or
`DATABASE_URL` / `WGUI_DATABASE_URL` from the environment file for `migrate dev`).

Model-level indexes in `schema.wdb` support ordered single-column or composite keys:

```wdb
model Message {
  channel_id: Int
  time: DateTime
  endpoint: String

  @@index([channel_id, time])
  @@unique([endpoint])
}
```

WGUI generates deterministic SQLite index names and only adds missing indexes; removing an
attribute does not drop an existing index. Generate or inspect migrations with:

```
cargo run -p wgui --features sqlite -- migrations diff
cargo run -p wgui --features sqlite -- migrations create add_message_indexes
cargo run -p wgui --features sqlite -- migrations compare --from old.wdb --to schema.wdb
cargo run -p wgui --features sqlite -- migrate dev --name add_message_indexes
```

For local prototyping, apply `schema.wdb` directly without creating or recording a migration:

```
cargo run -p wgui --features sqlite -- db push
```

`db push` adds missing tables, columns, and indexes. To reconcile removed tables, columns,
indexes, or changed column types, review the reported operations and rerun with
`--accept-data-loss`. It only removes tables managed by a previous `db push` run; unrelated
tables and migration history are preserved. Do not mix `db push` with pending SQL migrations for
the same database.

## LSP

See `docs/lsp.md` for setting up the `wui-lsp` server in Zed or other editors.

## API overview

- Core runtime: `Wgui::new(addr)`, `wgui.next().await`, `wgui.render(client_id, item)`
- Routes: `#[route("/path")]`, `#[route("/path", view)]` + `view!({ ... })`, `wgui.set_ctx(Arc<Ctx<AppState>>)`, and `wgui.add_route(handler_route)`
- Partials: `#[partial("/path")]`, `wgui.add_partial(handler_partial)`, `partial_region(address, item)`, and `ctx.render(address)`
- SSR snapshot: `Wgui::new_with_ssr(addr, || render())`
- HTTP hooks: `wgui.set_http_handler(...)` for app-specific same-origin endpoints before WGUI falls back to assets/SSR.
- Static assets: `wgui.mount_static_file(...)` returns a `StaticAsset`; pass `asset.url()` to consumers that need a content-versioned URL. Fingerprints update when the server restarts.
- Controller POST routes: add `#[wgui_post("/auth/login")]` to a `#[wgui_controller]` method and accept extractors such as `FormData`, `Json<T>`, `HttpRequest`, plus optional `HttpCtx`.
- Navigation: `ctx.push_state(url)` updates the SPA route, while `ctx.navigate(url)` performs a full browser navigation.
- Events: `ClientEvent::{Connected, Disconnected, OnClick, OnTextChanged, OnSliderChange, OnSelect, PathChanged}`

Component builders

- Layout: `vstack`, `hstack`
- Text: `text`
- Inputs: `text_input`, `date_picker`, `textarea`, `select` + `option`, `checkbox`, `slider`
- Actions: `button`
- Table: `table`, `thead`, `tbody`, `tr`, `th`, `td`
- Media: `img`
- Overlays: `modal`
- Misc: `folder_picker`

Item modifiers

- Identity: `.id(u32)`, `.inx(u32)`
- Value helpers: `.svalue(&str)`, `.ivalue(i32)`, `.checked(bool)`, `.placeholder(&str)`, `.min(i32)`, `.max(i32)`, `.step(i32)`, `.open(bool)`
- Layout/style: `.spacing(u32)`, `.wrap(bool)`, `.grow(u32)`, `.fill(bool)`, `.width(u32)`, `.min_width(u32)`, `.max_width(u32)`, `.height(u32)`, `.min_height(u32)`, `.max_height(u32)`, `.break_words(bool)`
- Box model: `.margin(u16)`, `.margin_left(u16)`, `.margin_right(u16)`, `.margin_top(u16)`, `.margin_bottom(u16)`, `.padding(u16)`, `.padding_left(u16)`, `.padding_right(u16)`, `.padding_top(u16)`, `.padding_bottom(u16)`
- Visuals: `.border(&str)`, `.background_color(&str)`, `.text_align(&str)`, `.cursor(&str)`, `.overflow(&str)`, `.editable(bool)`, `.hresize(bool)`, `.vresize(bool)`

## Development

```
# Build
bun build ./ts/app.ts --watch --outfile ./dist/index.js
# Check 
bunx tsc --noEmit
```
