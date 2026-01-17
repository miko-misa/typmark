# TypMark

TypMark は拡張 Markdown のための CLI ツールです。厳密な参照、Typst 数式、属性を扱います。

## TypMark について
TypMark は Markdown を基盤にした形式です。
参照は明示的で検証されます。
数式は Typst 構文で書き、SVG として出力されます。
属性はブロックにラベルやメタ情報を付け、スタイルやリンクに使えます。

## クイックスタート
GitHub Releases から入れて実行します。
`<tag>` は取得したリリースのタグに置き換えてください。
Linux と macOS は OS と CPU に合う tar.gz を使ってください。
詳しくはインストールの項目を見てください。

Linux と macOS の例 (Linux x86_64 の例)
```
tar -xzf typmark-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz
mv typmark-cli /usr/local/bin/typmark-cli
typmark-cli input.tmd > output.html
```

Windows PowerShell
```
Expand-Archive -Path typmark-cli-<tag>-x86_64-pc-windows-msvc.zip -DestinationPath .
Move-Item -Force typmark-cli.exe $env:USERPROFILE\\bin\\typmark-cli.exe
typmark-cli input.tmd > output.html
```

必要に応じて stdin や診断を使ってください。

## インストール
TypMark CLI は GitHub Releases から入れます。

GitHub Releases から入れる
- GitHub Releases から自分の OS 向けの成果物を取得する
- アーカイブを展開する
- PATH にある場所へバイナリを置く
- その場所を PATH に追加する

Linux と macOS の例
```
tar -xzf typmark-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz
mv typmark-cli /usr/local/bin/typmark-cli
```

Windows PowerShell の例
```
Expand-Archive -Path typmark-cli-<tag>-x86_64-pc-windows-msvc.zip -DestinationPath .
Move-Item -Force typmark-cli.exe $env:USERPROFILE\\bin\\typmark-cli.exe
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
TypMark CLI のバイナリは GitHub Releases から入手できます。

成果物
- `typmark-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz`
- `typmark-cli-<tag>-x86_64-pc-windows-msvc.zip`
- `typmark-cli-<tag>-x86_64-apple-darwin.tar.gz`
- `typmark-cli-<tag>-aarch64-apple-darwin.tar.gz`

OS 別の成果物
| OS | CPU | 成果物 |
| --- | --- | --- |
| Linux | x86_64 | `typmark-cli-<tag>-x86_64-unknown-linux-gnu.tar.gz` |
| Windows | x86_64 | `typmark-cli-<tag>-x86_64-pc-windows-msvc.zip` |
| macOS | x86_64 | `typmark-cli-<tag>-x86_64-apple-darwin.tar.gz` |
| macOS | arm64 | `typmark-cli-<tag>-aarch64-apple-darwin.tar.gz` |
