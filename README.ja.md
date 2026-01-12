# TypMark

TypMark の Rust コアです。厳密な参照、Typst 数式、属性を扱います。

## Workspace
- crates/typmark-core: 解析、解決、HTML 出力の中核
- crates/typmark-cli: CLI

## CLI
TypMark CLI は TypMark ファイルを読み取り、HTML を stdout に出力します。
診断は stderr に出力されます。
エラー診断がある場合は終了コードが 1 になります。
CLI の詳細は CLI.ja.md を参照してください。
英語版は CLI.md です。

例
```
cargo run -p typmark-cli -- --diagnostics pretty input.tmd
```

## Reference
TypMark のリファレンスは REFERENCE.ja.md です。
英語版は REFERENCE.md です。
