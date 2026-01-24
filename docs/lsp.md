# WUI Language Server (wui-lsp)

This project includes an LSP server for `.wui` files that runs over `--stdio`.
It provides diagnostics, basic completions, hover, go-to-definition, and rename for action strings.

## Build

From the repo root:

```
cargo build -p wui-lsp
```

This builds `target/debug/wui-lsp`.
Use `cargo build -p wui-lsp --release` for a release binary.

## Zed configuration

Zed can use either settings.json or a local dev extension.

### Option A: settings.json

Add to your Zed settings (User or Workspace):

```json
{
  "file_types": {
    "WUI": ["wui"]
  },
  "languages": {
    "WUI": {
      "language_servers": ["wui-lsp"],
      "file_extensions": ["wui"]
    }
  },
  "language_servers": {
    "wui-lsp": {
      "command": "target/debug/wui-lsp",
      "args": ["--stdio"]
    }
  }
}
```

For a release binary, change `command` to `target/release/wui-lsp`.

### Option B: local Zed extension

This repo includes a dev extension under `zed-wui/`. In Zed:

- Extensions → Install Dev Extension
- Select the `zed-wui/` folder

The extension launches the repo-local `target/debug/wui-lsp --stdio`.
Update `zed-wui/extension.toml` with a real tree-sitter grammar commit if needed.

If you previously added `.zed/settings.json` overrides, remove or adjust them so `.wui` files are owned by the `WUI` language from the extension.

## VSCode configuration

For VSCode, configure a custom language server via your extension or a launch config.
At minimum, the server command is:

```
/path/to/wui-lsp --stdio
```

## Notes

- Logs go to stderr. Use `RUST_LOG=info` to enable logging.
- The server currently re-parses the full file on each change.
