# TypMark

TypMark is a Rust core for an extended Markdown format with strict references, Typst math, and attributes.
This repository ships a library and a CLI.

## About TypMark
TypMark is a Markdown based format with three core features.
References are explicit and validated.
Math is written in Typst syntax and rendered to SVG.
Attributes can label and annotate blocks for styling and linking.

## Quick start
Render a file to HTML.
```
cargo run -p typmark-cli -- input.tmd > output.html
```

Read from stdin and write HTML to a file.
```
cat input.tmd | cargo run -p typmark-cli -- > output.html
```

Output raw HTML without renderer wrapping.
```
cargo run -p typmark-cli -- --raw input.tmd > output.html
```

Show diagnostics.
```
cargo run -p typmark-cli -- --diagnostics pretty input.tmd
```

## Install
TypMark CLI can be installed from GitHub Releases or built from source.

Install from GitHub Releases
- Download the latest release asset for your OS from GitHub Releases
- Extract the archive
- Move the binary to a directory in your PATH
- Make sure the directory is in your PATH

Example for Linux and macOS
```
tar -xzf typmark-cli-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
mv typmark-cli /usr/local/bin/typmark-cli
```

Example for Windows PowerShell
```
Expand-Archive -Path typmark-cli-v0.1.0-x86_64-pc-windows-msvc.zip -DestinationPath .
Move-Item -Force typmark-cli.exe $env:USERPROFILE\\bin\\typmark-cli.exe
```

Build from source
Build from source in this repository.

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
When diagnostics are enabled, HTML is still written to stdout. Ignore stdout if you only need diagnostics.

CLI reference
- CLI.md
- CLI.ja.md

Language reference
- REFERENCE.md
- REFERENCE.ja.md

## Release
TypMark CLI binaries are released from Git tags.
Push a tag like `v0.1.0` and GitHub Actions builds platform binaries and uploads them to GitHub Releases.

Release workflow
- `.github/workflows/release.yml`

Artifacts
- `typmark-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz`
- `typmark-cli-<tag>-x86_64-pc-windows-msvc.zip`
- `typmark-cli-<tag>-x86_64-apple-darwin.tar.gz`
- `typmark-cli-<tag>-aarch64-apple-darwin.tar.gz`

## Workspace
- crates/typmark-core: parse, resolve, and HTML emission
- crates/typmark-cli: CLI
- crates/typmark-renderer: HTML wrapping and assets

## Japanese
README in Japanese is in README.ja.md.
