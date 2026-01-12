# TypMark

Rust core for TypMark with strict references, Typst math, and attributes.

## Workspace
- crates/typmark-core: parse, resolve, and HTML emission
- crates/typmark-cli: CLI

## CLI
TypMark CLI reads a TypMark file and writes HTML to stdout.
Diagnostics are written to stderr.
Exit code is 1 when any error diagnostic exists.
CLI reference is in CLI.md.
Japanese CLI reference is in CLI.ja.md.

Example
```
cargo run -p typmark-cli -- --diagnostics pretty input.tmd
```

## Reference
TypMark language reference is in REFERENCE.md.
Japanese reference is in REFERENCE.ja.md.

## Japanese
README in Japanese is in README.ja.md.
