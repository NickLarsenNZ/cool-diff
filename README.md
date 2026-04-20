<p align="center">
  <img src="logo.svg" width="128" height="128" alt="cool-diff logo">
</p>

<h1 align="center">cool-diff</h1>

<p align="center">
  Compact, context-preserving diffs of structured data.
</p>

## What is cool-diff?

`cool-diff` compares two `serde_json::Value` trees and produces a minimal, human-readable diff. It is format-agnostic at the core (operates on parsed JSON values) and ships with a YAML-style renderer out of the box.

Designed for comparing Kubernetes resources, API responses, config files, or any structured data where you want to see exactly what changed without wading through noise.

## Features

- [x] Format-agnostic diff on `serde_json::Value` trees
- [x] Array matching by index, distinguished key, or content (contains)
- [x] Configurable per-path array matching and ambiguity strategies
- [x] Custom renderer support via `DiffRenderer` trait
- [x] YAML-style renderer with unified diff output (`-`/`+` indicators)
  - [x] Truncation for large subtrees (`# N more lines`)
  - [x] Omitted field/item markers (`# N fields omitted`)
  - [ ] ANSI colour output for terminal rendering
- [ ] JSON renderer
- [ ] Pre-configured `MatchConfig` for common Kubernetes resource types
- [ ] Inline comparison directives in the expected value

### Example output

```diff
 spec:
   # 2 fields omitted
   containers:
     - name: app
       # 2 items omitted
       env:
         - name: LOG_LEVEL
-          value: debug
+          value: info
-      image: "myapp:2.0"
+      image: "myapp:1.0"
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
cool-diff = "0.1"
```

## Quick start

```rust
use cool_diff::{diff, DiffConfig, DiffRenderer as _, YamlRenderer};

let actual = serde_json::json!({
    "server": {
        "host": "0.0.0.0",
        "port": 8080,
        "tls": true,
    }
});
let expected = serde_json::json!({
    "server": {
        "port": 3000,
    }
});

let tree = diff(&actual, &expected, &DiffConfig::default());

if !tree.is_empty() {
    let output = YamlRenderer::new().render(&tree);
    print!("{output}");
}
```

Output:

```diff
 server:
   # 2 fields omitted
-  port: 3000
+  port: 8080
```

## Array matching modes

By default, arrays are compared by position (index). You can configure per-path matching via `MatchConfig`:

| Mode | Description |
|---|---|
| **Index** (default) | Match by position. Element 0 compares to element 0, etc. |
| **Key** | Match by a configured distinguished field (e.g. `name`). Scans the actual array for an element with the same key value. |
| **Contains** | Find a matching element anywhere. Uses exact comparison for scalars, subset matching for objects. |

```rust
use cool_diff::{ArrayMatchConfig, ArrayMatchMode, DiffConfig, MatchConfig};

let config = DiffConfig::new().with_match_config(
    MatchConfig::new()
        .with_config_at(
            "spec.containers",
            ArrayMatchConfig::new(ArrayMatchMode::Key("name".to_owned())),
        )
        .with_config_at(
            "tags",
            ArrayMatchConfig::new(ArrayMatchMode::Contains),
        ),
);
```

## Renderer

The built-in `YamlRenderer` produces diff output using unified diff conventions:

Every line starts with an indicator in column 0:

- ` ` (space) for context lines
- `-` for expected values (what you wanted)
- `+` for actual values (what you got)

The renderer is configurable:

```rust
use cool_diff::YamlRenderer;

let renderer = YamlRenderer::new()
    .with_max_lines_per_side(Some(10))  // truncate large subtrees
    .with_indent_width(4);              // custom indentation
```

You can also implement the `DiffRenderer` trait for custom output formats. See `examples/custom_renderer.rs`.

## Examples

Runnable examples are in the `examples/` directory:

- [**barebones**](examples/barebones.rs) - simplest usage with all defaults
- [**match_configs**](examples/match_configs.rs) - mix of index, key, and contains matching
- [**contains_check**](examples/contains_check.rs) - checking that elements exist regardless of order
- [**kubernetes**](examples/kubernetes.rs) - diffing a Kubernetes Pod with key-based array matching
- [**custom_renderer**](examples/custom_renderer.rs) - implementing a custom `DiffRenderer`

Run any example with:

```sh
cargo run --example barebones
```

> [!TIP]
> Add `--features color` for coloured output:
>
> ```sh
> cargo run --example barebones --features color
> ```

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
