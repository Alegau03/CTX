# Install

## Prerequisites

- Rust toolchain (stable)
- macOS or Linux

## Build from source

```bash
cargo build --release
```

Binary path:

```bash
./target/release/ctx
```

Optional local install:

```bash
cargo install --path crates/ctx-cli
```

## Verify

```bash
ctx --help
cargo test --workspace
```
