# Clippy improvement backlog

This is a review of `cargo clippy --workspace --all-targets`.  It separates
mechanical cleanup from changes that need a deliberate API or performance
decision.  None of the findings is a compile error or a confirmed bug.

## Recommended first: safe, small cleanups

These changes preserve the existing design and are suitable for a focused
lint-cleanup commit.

### 1. Derive simple `Default` implementations

**Locations:** `wui-derive/src/lib.rs`, `wgui/src/gui.rs`

`RouteMethod`, `FlexDirection`, and `ItemPayload` have manual `Default`
implementations that only return one enum variant.  Mark that variant with
`#[default]` and derive `Default` instead.

```rust
#[derive(Default)]
enum RouteMethod {
    #[default]
    Get,
    Post,
}
```

This makes the intended default visible beside the enum variants and removes
boilerplate.  It should not change serialization or runtime behaviour.

### 2. Simplify duplicated route-template logic

**Location:** `wui-derive/src/lib.rs` around the route template directory
resolution.

Clippy found an `if` expression whose two branches produce the same value.
Remove the condition and retain the shared expression.

```rust
// Before
let directories = if condition { value } else { value };

// After
let directories = value;
```

Before merging, confirm whether the condition was meant to select a different
directory.  If so, this lint is pointing at an unfinished distinction rather
than merely redundant syntax.

### 3. Use idiomatic empty checks and defaults

**Locations:** `wgui/src/diff.rs`, `wgui/src/ui_client.rs`,
`wgui/src/wui/runtime.rs`, `wgui/src/lib.rs`

Replace verbose expressions with their standard-library equivalents:

```rust
// Before
if changes.len() == 0 { /* ... */ }
if sets.len() > 0 { /* ... */ }
map.entry(key).or_insert_with(Default::default)
option.unwrap_or_else(Default::default)

// After
if changes.is_empty() { /* ... */ }
if !sets.is_empty() { /* ... */ }
map.entry(key).or_default()
option.unwrap_or_default()
```

These are behaviour-preserving readability improvements.  `is_empty()` also
communicates intent more directly than comparing a length.

### 4. Use field-init shorthand and remove needless conversions

**Locations:** `wgui/src/diff.rs`, `wgui/src/ui_client.rs`, tests and examples

Where the field and local have the same name, prefer shorthand:

```rust
// Before
Change { item: item }

// After
Change { item }
```

Also remove conversions such as `value.into()` when `value` already has the
target type, and replace `&[value.clone()]`-style one-element slices where an
existing reference can be used.

This is fully mechanical and makes the real data transformations easier to
spot during review.

### 5. Remove unused code in tests and examples

**Locations:** `wgui/src/diff.rs`, `wgui/src/gui.rs`,
`examples/todo`, `examples/allcomponents`

Clippy reports unused imports, an unused test variable, and unused query
arguments in example handlers.  Remove them, or prefix intentionally-unused
parameters with an underscore:

```rust
// Before
fn todos(q: Query<Search>) -> Item { /* q is unused */ }

// After
fn todos(_q: Query<Search>) -> Item { /* ... */ }
```

For tests, prefer removing setup that is no longer relevant.  If it is meant
to document a scenario, make that purpose concrete with an assertion.

### 6. Replace single-pattern `match` blocks where they are only filtering

**Locations:** mostly `wgui/src/gui.rs`; also example code

Several builder methods use `match` with one meaningful arm and a no-op
fallback.  `if let` states that intent more clearly:

```rust
// Before
match payload {
    ItemPayload::Text(text) => text.value = value,
    _ => {}
}

// After
if let ItemPayload::Text(text) = payload {
    text.value = value;
}
```

Do this only where the fallback really is intentionally ignored.  Retain a
`match` when handling every future enum variant should be explicit.

### 7. Apply small boolean and control-flow simplifications

**Locations:** `wgui/src/schema_diff.rs`, `wgui/src/server.rs`,
`wgui/src/bin/wgui.rs`, example code

Examples reported by Clippy:

```rust
// Before
if index.unique == false { /* ... */ }
let Some(value) = option else { return None; };
if let Some(x) = value { if matches!(x, Kind::A) { /* ... */ } }

// After
if !index.unique { /* ... */ }
let value = option?;
if let Some(Kind::A) = value { /* ... */ }
```

The `?` form is appropriate only when the enclosing function already returns
`Option` or `Result` and the early-return value is exactly the corresponding
empty/error propagation.

## Design improvements worth planning

These findings are valid, but the right change affects public APIs or runtime
cost.  They should be reviewed independently of mechanical lint cleanup.

### 8. Replace the large `Server::new` parameter list with a configuration type

**Location:** `wgui/src/server.rs` (`Server::new` has ten parameters)

Large constructors are hard to call correctly, especially when several
arguments have similar types.  Group stable server configuration into a
named struct, with a builder for optional settings.

```rust
pub struct ServerConfig {
    pub address: SocketAddr,
    pub ssr_renderer: Option<SsrRenderer>,
    pub /* other configuration fields */
}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        // ...
    }
}
```

Benefits:

- Call sites label each non-obvious value.
- Adding an option does not require changing every call site.
- Related SSR settings can be validated together.

Migration consideration: retain the existing constructor temporarily or add a
`ServerConfig::new(...)` baseline constructor to avoid an unnecessarily abrupt
public API break.

### 9. Give the SSR callback type a name

**Location:** `wgui/src/server.rs`

The SSR renderer function type occurs repeatedly and triggers
`clippy::type_complexity`.  A type alias makes fields and method signatures
readable without changing behaviour:

```rust
pub type SsrRenderer = Arc<dyn Fn(&Request) -> BoxFuture<'static, Response>
    + Send
    + Sync>;
```

Use the actual request, future, and response types from the server module.
If the callback evolves, a trait such as `SsrRenderer` may eventually be more
extensible, but a type alias is the smallest useful first step.

### 10. Investigate large event enum variants before boxing them

**Locations:** `wgui/src/types.rs`, `wgui/src/wui/route_handler.rs`,
`wgui/src/lib.rs`

Clippy reports large variants in `ClientEvent`, `ServerEvent`, `RouteResult`,
and `PageMount` (roughly 488--568 bytes).  Rust sizes an enum to fit its
largest variant, so passing or queueing a small variant may still copy a large
value.

Possible change:

```rust
enum ServerEvent {
    Small(SmallEvent),
    Page(Box<PageEvent>),
}
```

Potential benefit: smaller enum values and less copying in queues/channels.
Potential cost: one heap allocation and pointer indirection for the boxed
case.  These types appear central to request and UI event flow, so benchmark
real event-heavy workloads before changing them.  Do not apply this merely to
silence Clippy.

### 11. Decide whether `Default` is meaningful for `Db` and `PubSub`

**Locations:** `wgui/src/db_table.rs`, `wgui/src/pubsub.rs`; an analogous
example type in `examples/puppychat`

Clippy sees `new()` constructors with no arguments and suggests `Default`.
Implement it only if an empty, ready-to-use value is a sound default:

```rust
impl<S> Default for Db<S> {
    fn default() -> Self {
        Self::new()
    }
}
```

This is useful for embedding in structs that derive `Default` and for generic
code.  It is not required if `new()` is preferred as the explicit construction
vocabulary or if initialization semantics might later become fallible.

### 12. Keep indexed edit-distance loops unless measurement supports a rewrite

**Location:** `wgui/src/edit_distance.rs`

Clippy suggests iterator-based loops because indices are used only to index
the dynamic-programming table.  An iterator rewrite may be more idiomatic:

```rust
for row in dp.iter_mut().skip(1) {
    // initialize or update the row
}
```

However, edit-distance code naturally needs neighbouring cells and explicit
row/column coordinates.  Indexed loops can be clearer and sometimes easier
for LLVM to optimize.  Treat this as a readability review, not a required
cleanup; preserve it if an iterator version is less understandable or slower.

### 13. Explicitly choose the Cargo feature resolver

**Location:** root `Cargo.toml`

Cargo warns that the workspace uses resolver `1` while containing edition-2024
crates, for which resolver `3` is the normal choice.  Consider adding:

```toml
[workspace]
resolver = "3"
```

This can alter how features are unified across the workspace.  Run the full
build and tests afterwards and review dependency feature changes, especially
if examples and core crates depend on the same package with different feature
sets.

## Suggested execution order

1. Make one mechanical lint-cleanup commit covering sections 1--7.
2. Add the SSR callback alias (section 9), optionally together with a
   backwards-compatible `ServerConfig` introduction (section 8).
3. Benchmark event delivery and SSR workloads before deciding on enum boxing
   (section 10).
4. Decide the public `Default` policy and Cargo resolver change in separate,
   reviewable commits.

After each implementation change, run:

```bash
cargo fmt --check
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets
```
