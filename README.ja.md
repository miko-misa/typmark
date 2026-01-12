# TypMark

TypMark は拡張 Markdown のための Rust コアです。厳密な参照、Typst 数式、属性を扱います。
このリポジトリにはライブラリと CLI が含まれます。

## TypMark について
TypMark は Markdown を基盤にした形式です。
参照は明示的で検証されます。
数式は Typst 構文で書き、SVG として出力されます。
属性はブロックにラベルやメタ情報を付け、スタイルやリンクに使えます。

## クイックスタート
CLI をビルドしてファイルを変換します。
```
cargo run -p typmark-cli -- --diagnostics pretty input.tmd
```

stdin から読み取り、HTML をファイルに保存します。
```
cat input.tmd | cargo run -p typmark-cli -- > output.html
```

## インストール
TypMark CLI はこのリポジトリからビルドします。

必要なもの
- Rust toolchain

ビルド
```
cargo build -p typmark-cli
```

実行
```
./target/debug/typmark-cli --diagnostics pretty input.tmd
```

## 使い方
TypMark CLI は TypMark のテキストを読み取り、HTML を stdout に出力します。
診断は stderr に出力されます。
エラー診断がある場合は終了コードが 1 になります。

CLI リファレンス
- CLI.ja.md
- CLI.md

言語リファレンス
- REFERENCE.ja.md
- REFERENCE.md

## Workspace
- crates/typmark-core: 解析、解決、HTML 出力の中核
- crates/typmark-cli: CLI
