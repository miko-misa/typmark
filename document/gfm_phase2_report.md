# GFM拡張 実装レポート (Phase 2)

## 実装範囲
- Tables / Task lists / Strikethrough / Autolink literals
- AST追加: `BlockKind::Table`, `ListItem.task`, `InlineKind::Strikethrough`
- Parser/Resolver/Emitter/Sanitizer をGFM対応に更新

## 設計メモ
- Tableはヘッダ/区切り/ボディを行ベースで解析し、セル前後の空要素を正規化。
- Task listはリスト項目の最初の段落の先頭にある`[ ]`/`[x]`を検出してフラグ化。
- Strikethroughは`~~`をEmphasis処理の拡張として実装。
- Autolink literalはInline Text上で後段処理し、境界/末尾句読点/`)`を調整。

## テスト
- 追加フィクスチャ: `gfm_table_cases`, `gfm_task_list_cases`, `gfm_strikethrough_cases`, `gfm_autolink_cases`
- GFM由来のエッジケースを統合追加:
  - Table: 余剰セル切り捨て、空セル、inline強調、escaped `|`、code span内の`|`
  - Task list: 非先頭`[ ]`、リンク付き、ネスト子タスク
  - Strikethrough: 連結境界 (`~~foo~~bar`)
  - Autolink: 境界条件、wwwパス、angle autolink、非リンクケース、末尾括弧
- 実行結果:
  - `cargo test -p typmark-core --test golden -- --nocapture`: OK
  - `cargo test -p typmark-core commonmark_spec -- --nocapture`: OK (GFM優先のAutolinkはスキップ)

## 既知の差分/今後の確認
- CommonMarkのAutolink例とGFM autolink literalが衝突するため、CommonMark specのAutolinksをGFM優先でスキップ。
