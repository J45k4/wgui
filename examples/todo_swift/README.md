# Todo SwiftUI Example

This example shows the native-owned UI shape for `wgui`.

SwiftUI owns the screen, text fields, list behavior, navigation, and platform styling. A small bridge protocol represents the Rust/wgui store. The store receives todo actions and publishes snapshots back to SwiftUI.

```text
SwiftUI TodoView
  -> TodoStore
  -> WguiTodoBridge
      -> mock bridge for previews/docs
      -> future FFI bridge to embedded Rust
      -> optional WebSocket bridge to remote wgui
```

The important part is that the UI does not own business rules. In a real app, `WguiTodoBridge` would call into Rust over FFI:

```text
wgui_todo_open() -> handle
wgui_todo_dispatch(handle, action_json)
wgui_todo_subscribe(handle, callback)
wgui_todo_snapshot(handle) -> snapshot_json
```

For now the package includes `MockTodoBridge`, so the SwiftUI code can be read or dropped into an iOS app before the FFI layer exists.
