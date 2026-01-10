# TypMark Core 実装メモ（Phase 0）

本ドキュメントは、現在の実装内容と設計意図を整理したものです。OSS向けに、
実装の現状、制約、次の課題を明示します。

## 1. 目的と範囲
- 目的: 仕様（`document/spec.md`）の Phase 0 を最小構成で実装し、基盤となる型とパーサ骨組みを用意する。
- 範囲: AST/Span/Diagnostics の基盤 + ブロックパーサ + 基本 inline 解析（強調/リンク/画像/参照/数式/コード）。
- 対象拡張子: `.tmd`（仕様で明記済み）。

## 2. モジュール構成
- `crates/typmark-core/src/ast.rs`
  - AST ノード定義（Document/Block/Inline など）。
  - Code line meta は 1-based 行番号を前提。
- `crates/typmark-core/src/span.rs`
  - `Span { start, end }`（byte offset, half-open）。
- `crates/typmark-core/src/source_map.rs`
  - `Span` → `Range` 変換用の line-start テーブル。
  - Phase 0 では `Range.character` は UTF-8 バイトオフセット。
- `crates/typmark-core/src/diagnostic.rs`
  - LSP 互換の `Diagnostic` 型と安定した code 群。
- `crates/typmark-core/src/parser.rs`
  - ブロックパーサの最小実装（行ベース）。
  - target-line attrs 付与、code meta の基本検証、最小 list/quote/box 対応。
- `crates/typmark-core/src/section.rs`
  - Heading から Section を構築する後処理。
- `crates/typmark-core/src/resolver.rs`
  - ラベル解決と参照検証、自己参照チェック、再帰深さチェック。
- `crates/typmark-core/src/emit.rs`
  - Phase 0 HTML エミッタ（正規化コード行ラッパー対応、ref の anchor 化）。

## 3. パースの流れ（Phase 0）
1. `parse()` で `SourceMap` と line 分割（`split_lines`）。
2. `parse_document()` → `parse_blocks()` を呼び出し、ブロック列を構築。
3. target-line attrs は「次のブロック」にのみ適用。
4. 各ブロックは `Span` と `AttrList` を保持。

## 4. サポート済みブロック
以下のブロックを最小構成で解析します。

- 見出し（`#`〜`######` / Setext `---` `===`）
- パラグラフ（連続行を統合）
- 水平線（thematic break / `***` `---` `___`）
- リスト（`-`/`*`/`+`、ordered は `1.`/`1)`）
- ブロック引用（`>`）
- コードフェンス（```）
- インデントコードブロック（4スペース）
- ボックス（`:::` + `box`）
- 数式ブロック（`$$ ... $$`）
- HTML block（raw HTML / CommonMark の type 1-7）

### 注意点
- list は tight/loose を判定（item 間または item 内のブロック間に空行/target line があれば loose）。
- list item の継続行は marker 幅 + 後続空白（最大4）以上のインデントが必要（タブは4文字タブストップ換算）。
- list marker の直後が行末でも空の list item として扱う（タブ/空白のみの行も可）。
- 空の list item は段落を中断しない（ordered は start=1 でも同様）。
- list item の開始時は空行を1つまで許可（2つ目で item 終了）。
- ordered list が段落を中断できるのは開始番号 `1` のみ（CommonMark準拠）。
- blockquote は先頭3スペースまでのインデントを許容し、paragraph だけ lazy continuation を許可。
- box は `::: box` のみ対応。
- ATX 見出しは先頭3スペースまでのインデントと任意の closing `#` を許容。
- code fence は ``` のみ（`~~~` 未対応）。先頭3スペースまでのインデントと閉じフェンスを許容。
- box 内の code fence / 数式ブロックは box 閉じ判定から除外する。
- HTML block は CommonMark の type 1-7 を簡略実装（終了条件やタグ境界は最小判定）。

## 5. Inline 解析（Phase 0）
`parse_inline()` は基本的な inline トークン化を実装しています。
- 対応: code span（`` `...` ``）、inline math（`$...$`）、参照（`@Label` / `@Label[...]`）
- 対応: 強調（`*...*` / `_..._`）、強い強調（`**...**` / `__...__`）
  - CommonMark 由来の delimiter ルール（left/right flanking、`_` の制約）で処理
- 対応: リンク（`[text](url "title")`）、画像（`![alt](url "title")`）
  - destination は `<...>` または `(...)` 内のネスト括弧を許容
  - title は `"..."` / `'...'` / `(...)` を許容（空白区切りが必須）
- 対応: 参照リンク（`[text][label]` / `[text][]` / `[text]`）
  - 定義行 `[label]: url "title"` を解析して参照先に解決（同一ドキュメント内）
  - 定義は文書全体から集計し、先行参照も解決
  - destination/title の改行分割と title の複数行を許容（空行は不可）
- 対応: autolink（`<https://...>` / `<user@example.com>`）
- 対応: inline HTML（タグ/コメント/宣言/処理命令/CDATA）
  - タグ名は ASCII 英字で開始し、英数字/`-` のみを許可
  - コメント: `<!-- ... -->`（単一行）
  - 宣言: `<!DOCTYPE ...>`（単一行）
  - 処理命令: `<? ... ?>`（単一行）
  - CDATA: `<![CDATA[ ... ]]>`（単一行）
- 対応: entity（HTML5 named + `&#...;` / `&#x...;`）
- 改行は `SoftBreak`、ただし行末スペース2個 or `\\` + 改行は `HardBreak`
- バックスラッシュは ASCII 記号をエスケープ
- bracket text はエスケープ（`\[`/`\]`/`\\`）を解釈し、inline として再帰解析
- code span は改行を空白に正規化し、先頭末尾の単一スペースを除去
- inline math に改行が含まれる場合は `E_MATH_INLINE_NL`
- reference bracket に改行が含まれる場合は `E_REF_BRACKET_NL`

## 6. Target-line attrs
`{#label key=value}` を検出すると `AttrList` として保持します。
- 直後のブロックに付与
- 付与対象がなければ `E_TARGET_ORPHAN`

## 7. Code meta（hl/diff）
`hl`, `diff_add`, `diff_del` を解析し、`CodeMeta` に格納します。
- オーバーラップは `E_CODE_CONFLICT`
- out-of-range は `W_CODE_RANGE_OOB`（Phase 0 では value 全体の range）
- code fence の attribute list に `#label` があればブロックラベルとして扱う

## 8. 診断（Diagnostics）
`Diagnostic { range, severity, code, message }` を採用。  
Phase 0 で使用する code は `document/core.md` に準拠します。

## 8.1 Resolver の概要
- Section 化後にラベル表を構築
- 参照解決: 未解決 → `W_REF_MISSING`、非タイトルへの `@label` → `E_REF_OMIT`
- タイトル内自己参照 → `E_REF_SELF_TITLE`
- タイトル参照の再帰深さ超過 → `E_REF_DEPTH`
- `@label`（括弧省略）時はタイトル由来の表示テキストを生成（ReferenceText コンテキスト）

## 9. 既知の制約（Phase 0）
- inline HTML は簡易判定（属性内 `>` などの厳密処理は未対応）
- raw HTML block は簡易判定（type 6/7 は空行で終了、タグ境界と閉じ判定は最小）
- named entity は WHATWG HTML5 の一覧に準拠（`entities.json`）
- list marker のインデントや内容インデントは簡略化（CommonMark の詳細ルールは未対応）
- ReferenceText 生成時はリンク/参照をフラット化（span ラップは最小）
- HTML エミッタは Phase 0 の最小形のみ

## 10. 次の実装優先度
1. GFM 拡張（tables/task list/strikethrough）と list ルール精密化
2. HTML/inline HTML の厳密化と sanitize 連携
3. Phase 1 以降のレンダリング（Typst/math SVG、sanitize）
