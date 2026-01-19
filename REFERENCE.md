# TypMark Reference

This reference explains how TypMark input becomes HTML output.

## Basic flow
- Input is TypMark text
- Output is HTML
- Labels become id attributes
- Extra attributes become data attributes

## Document settings
Place a settings line at the start of the document to control rendering.
This line is an attribute list with only key=value pairs.
It applies to the whole document and does not attach to the next block.

Input
```
{ font-size=16px line-height=1.7 font="Noto Sans, Segoe UI, sans-serif"
  math-inline-size=13pt math-block-size=14.5pt
  code-font="JetBrains Mono, Consolas, monospace" code-size=1em
  paragraph-gap=1.2em image-max-width=100% page-width=72rem }

# Title
Text.
```

Output
The settings are applied to the document before rendering.

Settings
- font-size default 16px
- line-height default 1.7
- font default Noto Sans, Segoe UI, sans-serif
- math-inline-size default 13pt
- math-block-size default 14.5pt
- math-font default inherit
- code-font default JetBrains Mono, Consolas, monospace
- code-size default 1em
- paragraph-gap default 1.2em
- page-width default none
- image-max-width default 100%

## Headings and sections
Input
```
# Title
Text.
```

Output
```
<section>
  <h1>Title</h1>
  <p>Text.</p>
</section>
```

Headings group a section. The section ends at the next heading with the same or higher level.

## Paragraphs
Input
```
First line
second line
```

Output
```
<p>First line
second line</p>
```

Line breaks are preserved. Two trailing spaces turn the break into br.

## Emphasis and strike
Input
```
*em* **strong** ~~strike~~
```

Output
```
<p><em>em</em> <strong>strong</strong> <del>strike</del></p>
```

## Code spans
Input
```
`let x = 1;`
```

Output
```
<p><code>let x = 1;</code></p>
```

## Links and images
Input
```
[site](https://example.com) ![alt](img.png)
```

Output
```
<p><a href="https://example.com">site</a> <img src="img.png" alt="alt" /></p>
```

Bare URLs and emails become links.

## References
References use labels.

Input
```
{#intro}
# Introduction

See @intro.
```

Output
```
<section id="intro">
  <h1>Introduction</h1>
  <p>See <a class="TypMark-ref" href="#intro">Introduction</a>.</p>
</section>
```

Targets without titles require reference text.

Input
```
{#p}
Paragraph.

See @p[text].
```

Output
```
<p id="p">Paragraph.</p>
<p>See <a class="TypMark-ref" href="#p">text</a>.</p>
```

When reference text is built, links and references become span. Images become alt text.

Input
```
{#sec}
# Title with [link](https://example.com) and ![img alt](img.png)

See @sec.
```

Output
```
<section id="sec">
  <h1>Title with <a href="https://example.com">link</a> and <img src="img.png" alt="img alt" /></h1>
  <p>See <a class="TypMark-ref" href="#sec">Title with <span class="TypMark-delink">link</span> and img alt</a>.</p>
</section>
```

Missing references are rendered as span.

Output
```
<span class="TypMark-ref ref-unresolved" data-ref-label="missing">missing</span>
```

## Target lines
Target lines attach labels and attributes to the next block.

Input
```
{#note level=high}
Paragraph.
```

Output
```
<p id="note" data-level="high">Paragraph.</p>
```

Target lines only apply within the same container. They do not cross list or quote boundaries.

## Boxes
Input
```
{#box1 bg="#f8f8f8"}
::: box Note
Body.
:::
```

Output
```
<div class="TypMark-box" data-typmark="box" id="box1" data-bg="#f8f8f8">
  <div class="TypMark-box-title">Note</div>
  <div class="TypMark-box-body">
    <p>Body.</p>
  </div>
</div>
```

## Math
Input
```
Inline $a^2 + b^2$ and block:

$$
E = mc^2
$$
```

Inline math is wrapped in `<span class="TypMark-math-inline">` and contains a line-height guard `<span class="TypMark-math-inline-strut">` followed by Typst SVG. When rendering fails, the raw text is emitted with an error class.

## Code blocks
Fenced code blocks use figure. Each line has data-line. This applies even when the language token is omitted. Lines marked as diff deletions do not receive data-line and do not increment displayed line numbers.

Input
````
```rs {#code note=keep hl="2:printf" diff_del="3"}
let a = 1;
printf("hi");
let b = 2;
```
````

Output
```
<figure class="TypMark-codeblock" data-typmark="codeblock" id="code" data-note="keep" data-hl="2:printf" data-diff_del="3" data-lang="rs">
  <pre class="TypMark-pre"><code class="language-rs"><span class="line" data-line="1">let a = 1;</span><span class="line highlighted" data-line="2" data-highlighted-line id="printf" data-line-label="printf">printf(&quot;hi&quot;);</span><span class="line diff del" data-diff="del">let b = 2;</span></code></pre>
</figure>
```

Indented code blocks use a simple output.

## Tables
Input
```
| a | b |
| --- | :---: |
| 1 | 2 |
```

Output
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

## Lists and tasks
Input
```
- item
- [ ] task
- [x] done
```

Output
```
<ul class="task-list">
  <li>item</li>
  <li class="task-list-item"><input type="checkbox" disabled="" /> task</li>
  <li class="task-list-item"><input type="checkbox" disabled="" checked="" /> done</li>
</ul>
```

## Block quotes
Input
```
> Quote
```

Output
```
<blockquote>
  <p>Quote</p>
</blockquote>
```

## Raw HTML
Raw HTML is emitted as-is. If it has a label or attributes, it is wrapped.

Input
```
{#raw note=keep}
<div>Raw</div>
```

Output
```
<div class="TypMark-html" data-typmark="html" id="raw" data-note="keep">
  <div>Raw</div>
</div>
```
