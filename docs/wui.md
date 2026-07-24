# WUI templates

WUI is a small JSX-like template language that compiles (or renders) to the existing `wgui` item tree.
It is designed to be deterministic and non-Turing complete: no user-defined functions, no loops outside `<For>`, and no arbitrary host calls.

## File layout

Recommended layout per app:

```
examples/your_app/
  wui/
    home.wui
  src/
    main.rs
```

## Template syntax

Elements and attributes:

- Self closing: `<Button text="Add" />`
- Nested: `<VStack> ... </VStack>`
- Attribute values:
  - String: `text="Add"`
  - Expression: `value={state.title}`
  - Bare literal: `spacing=4`, `checked=true`, `x=null`
  - Boolean shorthand: `disabled` is `disabled=true`

Expressions are limited to literals, paths, built-in calls, unary/binary ops, ternary, and `??`.

## Lexical rules

- Identifiers: `[A-Za-z_][A-Za-z0-9_:\-]*`
- Strings: `"..."`
- Numbers: `123` or `12.5`
- Booleans: `true`, `false`
- Null: `null`

Whitespace is ignored outside of strings.

## Expressions

Supported expression forms:

- Literals: `"text"`, `123`, `true`, `false`, `null`
- Paths: `state.title`, `item.completed`
- Unary: `!expr`, `-expr`
- Binary: `+ - * / %`, `== != < <= > >=`, `&& ||`
- Ternary: `cond ? a : b`
- Coalesce: `a ?? b`
- Built-ins: `path_matches(pattern)`

`path_matches(pattern)` matches the current route path against the same route patterns used by WGUI routes, including exact paths, `:param`/`{param}`, and `*`.

## Structural tags

- `<For each={state.items} itemAs="item" indexAs="i" key={item.id}> ... </For>`
- `<If test={state.items.len == 0}> ... </If>` followed by `<Else> ... </Else>`
- `<Scope name="list"> ... </Scope>` prefixes action names
- `<Page route="/todo" title="Todo" state="TodoState" />`
- `<Import src="partials/header.wui" />` inlines another template file
- `<Disconnected> ... </Disconnected>` renders its children only while the browser websocket is down
- `<Connected> ... </Connected>` renders its children only while the browser websocket is up

Rules:

- `For` requires `each`; `itemAs` defaults to `item`; `indexAs` is optional.
- `If` requires `test`.
- `Else` must immediately follow an `If` at the same nesting level.
- `Scope` requires `name`.
- `Page` is metadata only (no children).
- `Import` requires `src` and does not take children.

## Events and actions

Events are declared as string names on props:

```
<Button text="Add" onClick="AddTodo" />
<TextInput value={state.new_todo_name} onTextChanged="EditNewTodo" />
<Checkbox checked={item.completed} onClick="ToggleTodo" arg={item.id} />
```

The compiler/runtime turns those into action IDs and can decode `ClientEvent` into a typed action name.

## Forms and route actions

For CRUD-style mutations, prefer a `<Form>` and a `POST #[route]` handler over
keystroke actions. Form controls keep their values in the browser until submit.

```wui
<Form action="create">
  <TextInput name="name" placeholder="New todo" />
  <Button text="Add" />
</Form>

<Form action="toggle" arg={item.id}>
  <Button text="Toggle" />
</Form>
```

An action beginning with `/` is absolute. A relative action is resolved from
the current page route; `arg` inserts one path segment before the action. On
`/todos`, the forms above submit to `/todos/create` and
`/todos/:id/toggle`. A button inside a form submits unless it has a WUI event
prop. `TextInput name="field"` and `Checkbox name="field"` are standard HTML
form controls.

On the server, a POST route can take one typed form argument:

```rust
#[derive(serde::Deserialize)]
struct CreateTodoForm { name: String }

#[route("/todos/create", method = "POST")]
fn create(ctx: &Ctx<AppState>, form: CreateTodoForm) -> Redirect { /* … */ }
```

## Partial regions

Partials re-render a visible sub-tree only for clients that currently include
its concrete address. Mark the page region and register the partial handler:

```rust
let address = format!("/devices/{peer_id}/status");
let region = partial_region(address.clone(), status_item);

#[partial("/devices/:peer_id/status")]
fn status(ctx: &Ctx<AppState>, peer_id: String) -> View {
    View::partial(render_status(ctx, &peer_id))
}
```

Register it with `wgui.add_partial(status_partial)`. A mutation can then call
`ctx.render(address)`. WGUI reruns the handler per subscribed client and sends
the regular VDOM diff; it does not use stream operations.

## Widgets and props

Core tags and common props:

- `VStack`, `HStack`: `spacing`, `padding*`, `margin*`, `backgroundColor`, `border`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `grow`, `textAlign`, `cursor`, `wrap`, `overflow`
- `Connected`, `Disconnected`: same layout and style props as `VStack`
- `Text`: `value`, `textAlign`, `color`
- `Button`: `text`, `onClick`, `arg`
- `Form`: `action`, `arg`, `method`, plus layout props
- `TextInput`: `name`, `value`, `bind:value`, `placeholder`, `onTextChanged`
- `Checkbox`: `name`, `checked`, `bind:checked`, `onClick`, `arg`
- `Slider`: `min`, `max`, `value`, `step`, `onSliderChange`
- `Image`: `src`, `alt`, `objectFit`

Notes:

- Event props (`onClick`, `onTextChanged`, etc.) must be string literals.
- Only one event handler per element in the current implementation.
- `arg={...}` is supported for click events and is encoded as `inx` on the wire.

## Binding (bind:*)

Bindings are parsed and rendered but not yet wired to a server-side mutation protocol.
For now, use explicit actions for edits:

```
<TextInput value={state.name} onTextChanged="EditName" />
```

## Rendering options

There are three ways to use WUI today:

1) Compile-time codegen
- Use `wgui/src/bin/wui_gen.rs` or a build script that calls `wgui::wui::compiler::compile`.
- Emits `*_gen.rs` modules with `render()` and `decode()` helpers.
- Optionally generates a controller stub under `src/controllers/*_controller.rs` once (not overwritten).

2) Controller macro
- Use `#[wgui_controller]` on a controller impl that renders a WUI template.
- `mode = "auto"` is the default: debug builds load `.wui` files from the app `wui/` directory, while release builds embed the validated WUI source graph into the binary and parse it from memory.
- `mode = "runtime"` always reads templates from disk.
- `mode = "compiled"` always embeds templates at macro expansion time.
- `template = "path/inside/wui"` selects a specific template module without the `.wui` extension.

3) Runtime templates (hot reload)
- Use `wgui::wui::runtime::Template` to parse and render at runtime.

### Route views

`#[route("/path", view)]` resolves a conventional template below
`wui/pages` and lets the handler return a rendered WUI page with `view!`:

```rust
#[route("/todos", view)]
fn todos(ctx: &Ctx<AppState>) -> View {
	let todos = load_todos(ctx);
	view!({
		items: todos,
		filters: { completed: false },
	})
}
```

The anonymous object is available as `state` in the template, so the example
uses `state.items` and `state.filters.completed`. Template paths are derived
from the route: `/todos` uses `pages/todos/index`, `/todos/:id` uses
`pages/todos/show`, and `/todos/:id/edit` uses `pages/todos/edit`. The
wildcard route `/*` uses `pages/not_found`. Override the convention with
`template = "pages/admin/dashboard"` in the route attribute.

## Current limits

- No user-defined functions or arbitrary host calls in templates.
- Only one event handler per element.
- `bind:*` is parsed and rendered but does not yet generate mutation actions.
- Routing metadata is collected from `<Page>` but is not wired into a router yet.

## SSR snapshot

If you need a server-rendered first paint, use `Wgui::new_with_ssr`
to render an `Item` tree as HTML on initial load. The JS bundle will take over and apply updates.

## LSP support

`wui-lsp` provides diagnostics, completions, hover, go-to-definition, and rename for actions.
See `docs/lsp.md` for setup in Zed or other editors.

## Next steps

If you want deeper integration, consider:

- Binding protocol support (`bind:*` round trips)
- Router generation from `<Page route=...>`
- Stable key/identity plumbed into the diff engine
