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

## Structural tags

- `<For each={state.items} itemAs="item" indexAs="i" key={item.id}> ... </For>`
- `<If test={state.items.len == 0}> ... </If>` followed by `<Else> ... </Else>`
- `<Scope name="list"> ... </Scope>` prefixes action names
- `<Page route="/todo" title="Todo" state="TodoState" />`

## Events and actions

Events are declared as string names on props:

```
<Button text="Add" onClick="AddTodo" />
<TextInput value={state.new_todo_name} onTextChanged="EditNewTodo" />
<Checkbox checked={item.completed} onClick="ToggleTodo" arg={item.id} />
```

The compiler/runtime turns those into action IDs and can decode `ClientEvent` into a typed action name.

## Rendering options

There are two ways to use WUI today:

1) Compile-time codegen
- Use `wgui/src/bin/wui_gen.rs` or a build script that calls `wgui::wui::compiler::compile`.
- Emits `*_gen.rs` modules with `render()` and `decode()` helpers.

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

## Next steps

If you want deeper integration, consider:

- Binding protocol support (`bind:*` round trips)
- Router generation from `<Page route=...>`
- Stable key/identity plumbed into the diff engine
