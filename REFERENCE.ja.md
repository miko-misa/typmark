# TypMark Reference

このリファレンスは TypMark を書いたときに HTML がどう出力されるかを具体的に示す。

## 基本の流れ
- 入力は TypMark のテキスト
- 出力は HTML
- ラベルは id 属性になる
- 追加の属性は data で始まる属性になる

## 文書設定
文書の先頭に設定行を置くと表示を調整できる。
設定行は key=value だけで書いた属性リストであり、次のブロックには付かない。
文書全体に適用される。

入力
```
{ font-size=16px line-height=1.7 font="Noto Sans, Segoe UI, sans-serif"
  math-inline-size=13pt math-block-size=14.5pt
  code-font="JetBrains Mono, Consolas, monospace" code-size=1em
  paragraph-gap=1.2em image-max-width=100% page-width=72rem }

# Title
Text.
```

出力
設定はレンダリングの前に文書へ適用される。

設定項目

表示の挙動
- ブロック数式は既定で中央揃えになる。
- ブロック数式が横幅を超える場合は縮小せず横スクロールになる。
- 表と引用も横幅超過時は横スクロールになる。

- font-size 既定値 16px
- line-height 既定値 1.7
- font 既定値 Noto Sans, Segoe UI, sans-serif
- math-inline-size 既定値 13pt
- math-block-size 既定値 14.5pt
- math-font 既定値 inherit
- code-font 既定値 JetBrains Mono, Consolas, monospace
- code-size 既定値 1em
- paragraph-gap 既定値 1.2em
- page-width 既定値 none
- image-max-width 既定値 100%

## 見出しとセクション
入力
```
# Title
Text.
```

出力
```
<section>
  <h1>Title</h1>
  <p>Text.</p>
</section>
```

見出しはセクションとしてまとめられる。次の同じか上のレベルの見出しまでが同じセクションになる。

## 段落
入力
```
First line
second line
```

出力
```
<p>First line
second line</p>
```

改行はそのまま出力される。行末に空白が二つある場合は改行が br になる。

## 強調と打ち消し
入力
```
*em* **strong** ~~strike~~
```

出力
```
<p><em>em</em> <strong>strong</strong> <del>strike</del></p>
```

## コードスパン
入力
```
`let x = 1;`
```

出力
```
<p><code>let x = 1;</code></p>
```

## リンクと画像
入力
```
[site](https://example.com) ![alt](img.png)
```

出力
```
<p><a href="https://example.com">site</a> <img src="img.png" alt="alt" /></p>
```

URL やメールは裸で書いてもリンクになる。

## 参照
参照はラベルを使って書く。

入力
```
{#intro}
# Introduction

See @intro.
```

出力
```
<section id="intro">
  <h1>Introduction</h1>
  <p>See <a class="TypMark-ref" href="#intro">Introduction</a>.</p>
</section>
```

タイトルを持たない要素は参照テキストが必須になる。

入力
```
{#p}
Paragraph.

See @p[text].
```

出力
```
<p id="p">Paragraph.</p>
<p>See <a class="TypMark-ref" href="#p">text</a>.</p>
```

参照テキストを作るときはリンクと参照が span に置き換わる。画像は alt になる。

入力
```
{#sec}
# Title with [link](https://example.com) and ![img alt](img.png)

See @sec.
```

出力
```
<section id="sec">
  <h1>Title with <a href="https://example.com">link</a> and <img src="img.png" alt="img alt" /></h1>
  <p>See <a class="TypMark-ref" href="#sec">Title with <span class="TypMark-delink">link</span> and img alt</a>.</p>
</section>
```

参照先が見つからない場合は span で出力される。

出力
```
<span class="TypMark-ref ref-unresolved" data-ref-label="missing">missing</span>
```

## ターゲット行
ターゲット行は次のブロックにラベルと属性を付ける。

入力
```
{#note level=high}
Paragraph.
```

出力
```
<p id="note" data-level="high">Paragraph.</p>
```

ターゲット行は同じコンテナの中だけで効く。リストや引用の外には広がらない。

## ボックス
入力
```
{#box1 bg="#f8f8f8"}
::: box Note
Body.
:::
```

出力
```
<div class="TypMark-box" data-typmark="box" id="box1" data-bg="#f8f8f8">
  <div class="TypMark-box-title">Note</div>
  <div class="TypMark-box-body">
    <p>Body.</p>
  </div>
</div>
```

## 数式
入力
```
Inline $a^2 + b^2$ and block:

$$
E = mc^2
$$
```

インライン数式は `<span class="TypMark-math-inline">` に包まれ、行の高さを確保する `<span class="TypMark-math-inline-strut">` と Typst の SVG が入る。失敗した場合は元の文字列を error 用の class で出力する。

## コードブロック
コードフェンスは figure で出力される。各行に data-line が付く。言語指定がない場合も同じ。diff の削除行は data-line を付けず、表示上の行番号も増えない。

入力
````
```rs {#code note=keep hl="2:printf" diff_del="3"}
let a = 1;
printf("hi");
let b = 2;
```
````

出力
```
<figure class="TypMark-codeblock" data-typmark="codeblock" id="code" data-note="keep" data-hl="2:printf" data-diff_del="3" data-lang="rs">
  <pre class="TypMark-pre"><code class="language-rs"><span class="line" data-line="1">let a = 1;</span><span class="line highlighted" data-line="2" data-highlighted-line id="printf" data-line-label="printf">printf(&quot;hi&quot;);</span><span class="line diff del" data-diff="del">let b = 2;</span></code></pre>
</figure>
```

インデントのコードブロックは簡易出力になる。

## 表
入力
```
| a | b |
| --- | :---: |
| 1 | 2 |
```

出力
```
<table>
  <thead>
    <tr>
      <th>a</th>
      <th align="center">b</th>
    </tr>
  </thead>
  <tbody>
    <tr>
      <td>1</td>
      <td align="center">2</td>
    </tr>
  </tbody>
</table>
```

## リストとタスク
入力
```
- item
- [ ] task
- [x] done
```

出力
```
<ul class="task-list">
  <li>item</li>
  <li class="task-list-item"><input type="checkbox" disabled="" /> task</li>
  <li class="task-list-item"><input type="checkbox" disabled="" checked="" /> done</li>
</ul>
```

## 引用
入力
```
> Quote
```

出力
```
<blockquote>
  <p>Quote</p>
</blockquote>
```

## 生の HTML
生の HTML はそのまま出力される。ラベルや属性がある場合は外側にラッパーが付く。

入力
```
{#raw note=keep}
<div>Raw</div>
```

出力
```
<div class="TypMark-html" data-typmark="html" id="raw" data-note="keep">
  <div>Raw</div>
</div>
```
