# TypMark

TypMark is a CLI tool for an extended Markdown format with strict references, Typst math, and attributes.

## About TypMark
TypMark is a Markdown based format with three core features.
References are explicit and validated.
Math is written in Typst syntax and rendered to SVG.
Attributes can label and annotate blocks for styling and linking.

## Quick start
### VS Code extension
Use the VS Code extension for preview and diagnostics.
The extension uses the TypMark CLI under the hood.

- Repository: https://github.com/miko-misa/vscode-typmark
- Releases: https://github.com/miko-misa/vscode-typmark/releases
- Install the VSIX, reload VS Code, then open a .tmd file.
- Run TypMark: Show Preview to render.
- The extension downloads typmark-cli automatically when typmark.cliPath is empty.

### CLI
Install from GitHub Releases, then run.
Replace <tag> with the release tag you downloaded.
For Linux and macOS, use the tar.gz archive that matches your OS and CPU.
See Install for details and PATH setup.

Linux and macOS (example uses Linux x86_64)

```
tar -xzf typmark-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz
mv typmark-cli /usr/local/bin/typmark-cli
typmark-cli input.tmd > output.html
```

Windows PowerShell

```
Expand-Archive -Path typmark-cli-<tag>-x86_64-pc-windows-msvc.zip -DestinationPath .
Move-Item -Force typmark-cli.exe $env:USERPROFILE\bin\typmark-cli.exe
typmark-cli input.tmd > output.html
```

Then use stdin or diagnostics as needed.

## Install
TypMark CLI can be installed from GitHub Releases.

Install from GitHub Releases
- Download the latest release asset for your OS from GitHub Releases
- Extract the archive
- Move the binary to a directory in your PATH
- Make sure the directory is in your PATH

Example for Linux and macOS
```
tar -xzf typmark-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz
mv typmark-cli /usr/local/bin/typmark-cli
```

Example for Windows PowerShell
```
Expand-Archive -Path typmark-cli-<tag>-x86_64-pc-windows-msvc.zip -DestinationPath .
Move-Item -Force typmark-cli.exe $env:USERPROFILE\\bin\\typmark-cli.exe
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

## WASM (experimental)
TypMark has a browser-focused WASM build for rendering.

Build (local)
```
wasm-pack build crates/typmark-wasm --target web --out-dir pkg
```

Notes
- PDF export is disabled in wasm builds.
- Provide fonts via the JS API when you need custom fonts.

## Release
TypMark CLI binaries are available on GitHub Releases.

Artifacts
- `typmark-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz`
- `typmark-cli-<tag>-x86_64-pc-windows-msvc.zip`
- `typmark-cli-<tag>-x86_64-apple-darwin.tar.gz`
- `typmark-cli-<tag>-aarch64-apple-darwin.tar.gz`

Artifacts by OS
| OS | CPU | Artifact |
| --- | --- | --- |
| Linux | x86_64 | `typmark-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz` |
| Windows | x86_64 | `typmark-cli-<tag>-x86_64-pc-windows-msvc.zip` |
| macOS | x86_64 | `typmark-cli-<tag>-x86_64-apple-darwin.tar.gz` |
| macOS | arm64 | `typmark-cli-<tag>-aarch64-apple-darwin.tar.gz` |

## Japanese
README in Japanese is in README.ja.md.
