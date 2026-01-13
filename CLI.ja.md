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

--no-section-wrap
セクションのラップを行いません。

--render
HTML を完全なドキュメントとして出力し、CSS を埋め込みます。

--render-js
HTML を完全なドキュメントとして出力し、CSS と JS を埋め込みます。

--raw
レンダラーを通さずに生の HTML を出力します。

--theme auto|light|dark
レンダリング時のテーマを指定します。デフォルトは dark です。

--help
使い方を表示します。

## 出力
HTML は stdout に出力されます。
診断は stderr に出力されます。

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
