# TypMark CLI リファレンス

## 概要
TypMark CLI は TypMark のテキストを読み取り、HTML を stdout に出力します。
診断は stderr に出力されます。
エラー診断がある場合は終了コードが 1 になります。

## 使い方
```
typmark-cli [options] [input]
```

input を省略した場合は stdin から読み取ります。

## オプション
--diagnostics json
診断を JSON 形式で stderr に出力します。

--diagnostics pretty
診断を読みやすい形式で stderr に出力します。

--sanitized
HTML をサニタイズします。

--simple-code
コードブロックを簡易形式で出力します。

--source-map
HTML に data-tm-range を付与します。値は 0 から始まる行番号と UTF-8 のバイト列の列番号で、startLine:startCol-endLine:endCol の形式です。エディタのプレビュー同期向けです。

--no-section-wrap
セクションのラップを行いません。

--render
HTML を完全なドキュメントとして出力し、CSS と JS を埋め込みます。これがデフォルトの出力です。

--render-js
--render と同じ動作です（互換性のために残しています）。

--raw
レンダラーを通さずに生の HTML を出力します。

--pdf output.pdf
指定したパスに PDF を出力します。PDF 用の設定は文書設定行から読み取ります。

--theme auto|light|dark
レンダリング時のテーマを指定します。デフォルトは dark です。

--version
CLI のバージョンを表示します。

--help
使い方を表示します。

## PDF 設定
PDF 出力のときに文書設定行から参照されます。

- pdf-page
- pdf-margin 既定値 1.5rem
- pdf-scale
- pdf-base
- pdf-backend

PDF 出力ではレンダラーのスタイルとシンタックスハイライトが適用されます。
PDF には JavaScript を埋め込みません。
PDF 出力では白背景の専用ライトテーマを強制的に使います。
wkhtmltopdf または Chromium 系のブラウザを事前にインストールしてください。
PDF 出力では pdf-margin をページのマージンとして扱います。

## 出力
HTML は stdout に出力されます。
診断は stderr に出力されます。
診断を有効にした場合でも HTML は stdout に出力されます。診断のみ必要な場合は stdout を無視してください。
--pdf を使うと PDF は指定したファイルに書き出され、HTML は出力されません。

## 終了コード
0 はエラー診断がない場合です。
1 はエラー診断がある場合です。
2 は引数が不正な場合です。

## 例
```
typmark-cli input.tmd > output.html
```

```
cat input.tmd | typmark-cli --diagnostics pretty
```

```
typmark-cli --render input.tmd > output.html
```

```
typmark-cli --render --theme dark input.tmd > output.html
```

```
typmark-cli --raw input.tmd > output.html
```

```
typmark-cli --pdf output.pdf input.tmd
```
