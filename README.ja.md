# TypMark

TypMark は拡張 Markdown のための Rust コアです。厳密な参照、Typst 数式、属性を扱います。
このリポジトリにはライブラリと CLI が含まれます。

## TypMark について
TypMark は Markdown を基盤にした形式です。
参照は明示的で検証されます。
数式は Typst 構文で書き、SVG として出力されます。
属性はブロックにラベルやメタ情報を付け、スタイルやリンクに使えます。

## クイックスタート
ファイルを HTML に変換します。
```
cargo run -p typmark-cli -- input.tmd > output.html
```

stdin から読み取り、HTML をファイルに保存します。
```
cat input.tmd | cargo run -p typmark-cli -- > output.html
```

レンダラーを通さずに生の HTML を出力します。
```
cargo run -p typmark-cli -- --raw input.tmd > output.html
```

診断を表示します。
```
cargo run -p typmark-cli -- --diagnostics pretty input.tmd
```

## インストール
TypMark CLI は GitHub Releases から入れる方法と、ソースからビルドする方法があります。

GitHub Releases から入れる
- GitHub Releases から自分の OS 向けの成果物を取得する
- アーカイブを展開する
- PATH にある場所へバイナリを置く
- その場所を PATH に追加する

Linux と macOS の例
```
tar -xzf typmark-cli-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
mv typmark-cli /usr/local/bin/typmark-cli
```

Windows PowerShell の例
```
Expand-Archive -Path typmark-cli-v0.1.0-x86_64-pc-windows-msvc.zip -DestinationPath .
Move-Item -Force typmark-cli.exe $env:USERPROFILE\\bin\\typmark-cli.exe
```

ソースからビルドする
このリポジトリからビルドします。

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
診断を有効にした場合でも HTML は stdout に出力されます。診断のみ必要な場合は stdout を無視してください。

CLI リファレンス
- CLI.ja.md
- CLI.md

言語リファレンス
- REFERENCE.ja.md
- REFERENCE.md

## リリース
TypMark CLI のバイナリはタグからリリースします。
`v0.1.0` のようなタグを push すると GitHub Actions が各環境向けにビルドして GitHub Releases に添付します。

リリース用 workflow
- `.github/workflows/release.yml`

成果物
- `typmark-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz`
- `typmark-cli-<tag>-x86_64-pc-windows-msvc.zip`
- `typmark-cli-<tag>-x86_64-apple-darwin.tar.gz`
- `typmark-cli-<tag>-aarch64-apple-darwin.tar.gz`

## Workspace
- crates/typmark-core: 解析、解決、HTML 出力の中核
- crates/typmark-cli: CLI
- crates/typmark-renderer: HTML ラップとアセット
