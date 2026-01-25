# WUI templates

WUI is a small JSX-like template language that compiles (or renders) to the existing `wgui` item tree.
It is designed to be deterministic and non-Turing complete: no functions, no loops outside `<For>`, and no arbitrary host calls.

## File layout

Recommended layout per app:

```
examples/your_app/
  wui/pages/
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

Expressions are limited to literals, paths, unary/binary ops, ternary, and `??`.

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

Builtins are reserved for future use and are not evaluated yet.

## Structural tags

- `<For each={state.items} itemAs="item" indexAs="i" key={item.id}> ... </For>`
- `<If test={state.items.len == 0}> ... </If>` followed by `<Else> ... </Else>`
- `<Scope name="list"> ... </Scope>` prefixes action names
- `<Page route="/todo" title="Todo" state="TodoState" />`

Rules:

- `For` requires `each`; `itemAs` defaults to `item`; `indexAs` is optional.
- `If` requires `test`.
- `Else` must immediately follow an `If` at the same nesting level.
- `Scope` requires `name`.
- `Page` is metadata only (no children).

## Events and actions

Events are declared as string names on props:

```
<Button text="Add" onClick="AddTodo" />
<TextInput value={state.new_todo_name} onTextChanged="EditNewTodo" />
<Checkbox checked={item.completed} onClick="ToggleTodo" arg={item.id} />
```

The compiler/runtime turns those into action IDs and can decode `ClientEvent` into a typed action name.

## Widgets and props

Core tags and common props:

- `VStack`, `HStack`: `spacing`, `padding*`, `margin*`, `backgroundColor`, `border`, `width`, `height`, `minWidth`, `maxWidth`, `minHeight`, `maxHeight`, `grow`, `textAlign`, `cursor`, `wrap`, `overflow`
- `Text`: `value`, `textAlign`, `color`
- `Button`: `text`, `onClick`, `arg`
- `TextInput`: `value`, `bind:value`, `placeholder`, `onTextChanged`
- `Checkbox`: `checked`, `bind:checked`, `onClick`, `arg`
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
## Rendering options

There are two ways to use WUI today:

1) Compile-time codegen
- Use `wgui/src/bin/wui_gen.rs` or a build script that calls `wgui::wui::compiler::compile`.
- Emits `*_gen.rs` modules with `render()` and `decode()` helpers.
- Optionally generates a controller stub under `src/controllers/*_controller.rs` once (not overwritten).

2) Runtime templates (hot reload)
- Use `wgui::wui::runtime::Template` to parse and render at runtime.
- See `examples/todo_wui` for file watching and re-render on change.

## Hot reload (runtime)

The `todo_wui` example demonstrates hot reload:

- It watches `wui/pages/todo.wui` for changes.
- On update, it re-parses the template and re-renders all connected clients.

Run it with:

```
cargo run -p todo_wui
```

## Current limits

- No user-defined functions or arbitrary host calls in templates.
- Only one event handler per element.
- `bind:*` is parsed and rendered but does not yet generate mutation actions.
- Routing metadata is collected from `<Page>` but is not wired into a router yet.
- Expression builtins are not executed yet.

## SSR snapshot

If you need a server-rendered first paint, use `Wgui::new_with_ssr` or `axum::router_with_ssr`
to render an `Item` tree as HTML on initial load. The JS bundle will take over and apply updates.

## LSP support

`wui-lsp` provides diagnostics, completions, hover, go-to-definition, and rename for actions.
See `docs/lsp.md` for setup in Zed or other editors.

## Next steps

If you want deeper integration, consider:

- Binding protocol support (`bind:*` round trips)
- Router generation from `<Page route=...>`
- Stable key/identity plumbed into the diff engine
