# Repository Guidelines

## When making changes

1. Make changes
2. Run these commands to verify your changes:
   - `cargo build --workspace` to ensure all Rust code compiles.
   - `cargo test --workspace` to run unit tests in the core crate.
3. If any of these fails goto 1

## Project Structure & Module Organization
The workspace root (`Cargo.toml`) ties together the core `wgui` crate and the `examples/*` showcase projects. Core server-side virtual DOM logic lives in `wgui/src` (notably `gui.rs`, `server.rs`, `diff.rs`), with focused unit tests colocated under `#[cfg(test)]`. Browser-facing TypeScript resides in `ts/` and is bundled into `dist/index.js`; treat everything under `dist/` as generated output. Example applications such as `examples/todo` and `examples/allcomponents` exercise the library and ship any static assets they need.

## Build, Test, and Development Commands
- `cargo build` – compile all workspace crates to validate Rust changes.
- `cargo test -p wgui` – run the unit tests embedded in the core crate; add `-- --nocapture` when debugging logs.
- `cargo run -p todo` – launch the Todo example and verify end-to-end behaviour.
- `bun build ./ts/app.ts --outfile ./dist/index.js` – produce the browser bundle once; add `--watch` or call `bundle_watch.sh`/`.bat` for iterative work.
- `bunx tsc ./ts/* --noEmit --allowImportingTsExtensions` – type-check the front-end exactly as documented in `readme.md`.

## Coding Style & Naming Conventions
Editors should honour `.editorconfig` (tabs with a visual width of four). Run `cargo fmt` before committing Rust changes to normalize spacing, and prefer `snake_case` for functions/modules and `PascalCase` for types. TypeScript sticks to camelCase identifiers, PascalCase classes, and omits trailing semicolons; keep imports relative with explicit `.ts` extensions. Avoid manual edits in `dist/`; adjust `ts/` sources and re-run the bundler instead.

## Testing Guidelines
Add Rust tests alongside the modules they exercise, mirroring the existing `#[cfg(test)]` blocks in `diff.rs` and friends. Run `cargo test -p wgui` before every push, and consider `cargo test -p <example>` when modifying shared behaviour that examples rely on. The front-end currently lacks automated tests, so rely on `bunx tsc` plus live browser verification via the running example applications.

## Commit & Pull Request Guidelines
Recent commits use short, imperative messages in lowercase (e.g., `making folder picker`); follow that tone and scope. Squash trivial fixups locally, and include a brief summary of the affected module(s) in the body when needed. Pull requests should describe user-visible changes, list the commands you ran (`cargo test`, `bun build`), and link to any relevant issues or screenshots that help reviewers.
