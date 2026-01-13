# TypMark

TypMark is a Rust core for an extended Markdown format with strict references, Typst math, and attributes.
This repository ships a library and a CLI.

## About TypMark
TypMark is a Markdown based format with three core features.
References are explicit and validated.
Math is written in Typst syntax and rendered to SVG.
Attributes can label and annotate blocks for styling and linking.

## Quick start
Build and run the CLI on a file.
```
cargo run -p typmark-cli -- --diagnostics pretty input.tmd
```

Read from stdin and write HTML to a file.
```
cat input.tmd | cargo run -p typmark-cli -- > output.html
```

Render a full HTML document with embedded CSS. This is the default output, using the dark theme.
```
cargo run -p typmark-cli -- --render input.tmd > output.html
```

Output raw HTML without renderer wrapping.
```
cargo run -p typmark-cli -- --raw input.tmd > output.html
```

## Install
TypMark CLI is built from source in this repository.

Requirements
- Rust toolchain

Build
```
cargo build -p typmark-cli
```

Run
```
./target/debug/typmark-cli --diagnostics pretty input.tmd
```

## Usage
TypMark CLI reads TypMark text and writes HTML to stdout.
Diagnostics are written to stderr.
Exit code is 1 when any error diagnostic exists.

CLI reference
- CLI.md
- CLI.ja.md

Language reference
- REFERENCE.md
- REFERENCE.ja.md

## Workspace
- crates/typmark-core: parse, resolve, and HTML emission
- crates/typmark-cli: CLI
- crates/typmark-renderer: HTML wrapping and assets

## Japanese
README in Japanese is in README.ja.md.
