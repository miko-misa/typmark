# TypMark Phase 1 — Design Document

## 1. Overview

**Phase 1 Goal:** Achieve CommonMark baseline completeness.

Phase 0 implemented the minimal TypMark-specific features and a subset of CommonMark blocks/inlines. Phase 1 fills the gaps to reach full CommonMark compatibility while maintaining all Phase 0 TypMark features (target-line attributes, references, math, boxes).

**Status:** Planning
**Dependencies:** Phase 0 complete
**Target Completion:** TBD

---

## 2. Phase 0 vs Phase 1 Feature Matrix

### 2.1 Blocks

| Block Type | Phase 0 | Phase 1 | Notes |
|------------|---------|---------|-------|
| ATX Headings | ✅ | ✅ | Already complete |
| Setext Headings | ✅ | ✅ | Already complete |
| Paragraphs | ✅ | ✅ | Already complete |
| Thematic Breaks | ✅ | ✅ | Already complete |
| Code Blocks (Fenced) | ✅ | ✅ | With TypMark metadata extension |
| Code Blocks (Indented) | ❌ | ✅ | **NEW in Phase 1** |
| Blockquotes | ✅ | ✅ | Already complete |
| Lists (Unordered) | ✅ | ✅ | Already complete |
| Lists (Ordered) | ✅ | ✅ | Already complete |
| Lists (Tight/Loose) | ✅ | ✅ | Already complete |
| HTML Blocks | ✅ | ✅ | Already complete |
| Math Blocks ($$) | ✅ | ✅ | TypMark extension |
| Box Blocks (:::) | ✅ | ✅ | TypMark extension |
| Link Reference Definitions | ❌ | ✅ | **NEW in Phase 1** |

### 2.2 Inlines

| Inline Type | Phase 0 | Phase 1 | Notes |
|-------------|---------|---------|-------|
| Text | ✅ | ✅ | Already complete |
| Emphasis (*/_) | ✅ | ✅ | Already complete |
| Strong (**/__) | ✅ | ✅ | Already complete |
| Code Spans | ✅ | ✅ | Already complete |
| Links (Inline) | ✅ | ✅ | Already complete |
| Links (Reference) | ❌ | ✅ | **NEW in Phase 1** |
| Images (Inline) | ✅ | ✅ | Already complete |
| Images (Reference) | ❌ | ✅ | **NEW in Phase 1** |
| Autolinks | ✅ | ✅ | Basic form complete |
| Autolinks (Extended) | ❌ | ⏭️ | **Deferred to Phase 2 (GFM)** |
| HTML Spans | ✅ | ✅ | Already complete |
| Line Breaks (Soft) | ✅ | ✅ | Already complete |
| Line Breaks (Hard) | ✅ | ✅ | Already complete |
| Inline Math ($) | ✅ | ✅ | TypMark extension |
| References (@label) | ✅ | ✅ | TypMark extension |

### 2.3 Edge Cases and Compliance

| Feature | Phase 0 | Phase 1 | Notes |
|---------|---------|---------|-------|
| Nested emphasis rules | ⚠️ | ✅ | Improve left-flanking / right-flanking detection |
| Link title parsing | ⚠️ | ✅ | Support all CommonMark quote styles |
| Escaping backslash | ⚠️ | ✅ | Full escape sequence support |
| Entity references | ⚠️ | ✅ | Numeric + named entities (full spec) |
| Precedence rules | ⚠️ | ✅ | Code span > link > emphasis |
| Delimiter run rules | ⚠️ | ✅ | Exact CommonMark algorithm |

---

## 3. New Features for Phase 1

### 3.1 Indented Code Blocks

**Syntax:**
```
    code line 1
    code line 2
```

**Rules:**
- Lines indented by 4+ spaces (or 1 tab) form a code block.
- No language or metadata support (unlike fenced code blocks).
- Ends when a line is not indented or the container ends.
- Cannot interrupt a paragraph (blank line required).

**Implementation:**
- Add `IndentedCodeBlock` variant or reuse `CodeBlock` with `lang = None` and no metadata.
- Parse during block-level pass, after checking for fenced blocks.

**Diagnostics:**
- None specific to indented code blocks.

**HTML Output:**
```html
<pre><code>code line 1
code line 2
</code></pre>
```

**Tests:**
- `indented_code_basic.tmd`
- `indented_code_after_paragraph.tmd`
- `indented_code_in_list.tmd`

---

### 3.2 Link Reference Definitions

**Syntax:**
```
[label]: https://example.com "Title"
[label2]: /path/to/page
```

**Rules:**
- Must start at column 0 or indented up to 3 spaces.
- Label is case-insensitive for matching (normalize to lowercase).
- URL may be wrapped in `<...>`.
- Title is optional; may use `"..."`, `'...'`, or `(...)`.
- May span multiple lines (title on next line).
- Does NOT produce any output in HTML (definitions are invisible).

**Implementation:**
- Parse during block-level pass.
- Store in a `HashMap<String, (url: String, title: Option<String>)>`.
- Normalize label to lowercase for lookup.
- Pass the definition map to the inline parser.

**Diagnostics:**
- `W_LINK_DEF_UNUSED` (warning): A link reference definition was never used.
- `W_LINK_DEF_DUP` (warning): Duplicate link reference definition (first wins).

**Tests:**
- `link_ref_def_basic.tmd`
- `link_ref_def_multiline.tmd`
- `link_ref_def_case_insensitive.tmd`
- `link_ref_def_unused.tmd` (diagnostics)

---

### 3.3 Reference Links and Images

**Syntax (Full):**
```
[link text][ref]
![alt text][ref]
```

**Syntax (Collapsed):**
```
[link text][]
![alt text][]
```

**Syntax (Shortcut):**
```
[ref]
![ref]
```

**Rules:**
- `[ref]` looks up the link reference definition with label `ref` (case-insensitive).
- If definition not found, render as plain text.
- Collapsed form: reference label is the same as the link text.
- Shortcut form: no `[]` after, reference label is the link text.

**Implementation:**
- During inline parsing, when encountering `[`, check for reference syntax.
- Look up in the link definition map (case-insensitive).
- If found, create `Link` or `Image` node with resolved URL and title.
- If not found, treat as plain text.

**Diagnostics:**
- `W_LINK_REF_MISSING` (warning): Reference link `[text][ref]` has no definition for `ref`.

**HTML Output:**
Same as inline links/images:
```html
<a href="url" title="title">link text</a>
<img src="url" alt="alt text" title="title" />
```

**Tests:**
- `link_ref_full.tmd`
- `link_ref_collapsed.tmd`
- `link_ref_shortcut.tmd`
- `link_ref_missing.tmd` (diagnostics)
- `image_ref_full.tmd`
- `image_ref_collapsed.tmd`
- `image_ref_shortcut.tmd`

---

### 3.4 Emphasis and Strong Delimiter Rules (Strict CommonMark)

**Current State (Phase 0):**
- Basic left/right-flanking detection implemented.
- May not handle all edge cases (e.g., punctuation, Unicode whitespace).

**Phase 1 Requirements:**
- Implement full CommonMark delimiter run rules:
  - Left-flanking: not followed by Unicode whitespace, and either not followed by punctuation or preceded by whitespace/punctuation.
  - Right-flanking: not preceded by Unicode whitespace, and either not preceded by punctuation or followed by whitespace/punctuation.
  - Can open emphasis: left-flanking and (if `_`) not preceded by alphanumeric.
  - Can close emphasis: right-flanking and (if `_`) not followed by alphanumeric.
- Implement delimiter stack algorithm for nested emphasis.

**Implementation:**
- Refactor inline parser's emphasis handling.
- Build a delimiter stack as per CommonMark spec §6.2.
- Process delimiters in order, matching openers and closers.

**Diagnostics:**
- None (this is pure parsing logic).

**Tests:**
- `emph_flanking_punctuation.tmd`
- `emph_underscore_intraword.tmd`
- `emph_nested_complex.tmd`
- Import relevant tests from CommonMark test suite (JSON format).

---

### 3.5 Backslash Escaping (Full CommonMark Set)

**Current State (Phase 0):**
- Basic escaping implemented for common punctuation.

**Phase 1 Requirements:**
- Support all ASCII punctuation characters per CommonMark §6.1:
  ```
  ! " # $ % & ' ( ) * + , - . / : ; < = > ? @ [ \ ] ^ _ ` { | } ~
  ```
- Backslash before non-escapable character is treated as a literal backslash.

**Implementation:**
- Extend escape handling in inline parser.
- Use a lookup table or match for all escapable ASCII punctuation.

**Diagnostics:**
- None.

**Tests:**
- `escape_all_punctuation.tmd`
- `escape_non_escapable.tmd`

---

### 3.6 Entity References (Numeric and Named)

**Current State (Phase 0):**
- Basic named entity decoding implemented via `entities.rs`.

**Phase 1 Requirements:**
- Support all HTML5 named entities (already in `entities.rs`).
- Support numeric entities:
  - Decimal: `&#123;`
  - Hexadecimal: `&#xAB;` or `&#XAB;`
- Invalid entities should be treated as literal text.

**Implementation:**
- Extend entity parsing in inline parser.
- Parse `&#` followed by digits or `x`/`X` + hex digits, ending with `;`.
- Convert to Unicode character if valid code point.
- If invalid, treat as literal text (no error).

**Diagnostics:**
- None (invalid entities are silently treated as text per CommonMark).

**Tests:**
- `entity_numeric_decimal.tmd`
- `entity_numeric_hex.tmd`
- `entity_invalid.tmd`
- `entity_named.tmd` (already covered in Phase 0, expand)

---

### 3.7 Link Title Parsing (All Quote Styles)

**Current State (Phase 0):**
- Basic title parsing with `"..."` implemented.

**Phase 1 Requirements:**
- Support all three quote styles:
  - `"title"`
  - `'title'`
  - `(title)`
- Title may contain escaped quotes of the same type.
- Title may span multiple lines (for link reference definitions).

**Implementation:**
- Extend link destination/title parsing.
- Detect opening quote/paren and find matching closing quote/paren.
- Handle backslash escapes inside titles.

**Diagnostics:**
- None (malformed titles result in no title, per CommonMark).

**Tests:**
- `link_title_double_quotes.tmd`
- `link_title_single_quotes.tmd`
- `link_title_parens.tmd`
- `link_title_escaped.tmd`

---

### 3.8 Code Span Parsing (Strict Rules)

**Current State (Phase 0):**
- Basic code span parsing implemented.

**Phase 1 Requirements:**
- Multiple backticks support: ` `` code `` ` allows single backtick inside.
- Strip one leading and one trailing space if both present (normalization rule).
- Line endings are converted to spaces.

**Implementation:**
- Refine code span parsing to count opening backticks.
- Match closing sequence of same length.
- Apply space-stripping rule.

**Diagnostics:**
- None.

**Tests:**
- `code_span_multiple_backticks.tmd`
- `code_span_space_stripping.tmd`
- `code_span_line_endings.tmd`

---

## 4. Testing Strategy

### 4.1 Golden Tests (New Fixtures)

Add test fixtures for each new feature:
- Indented code blocks: 3 fixtures
- Link reference definitions: 4 fixtures
- Reference links: 6 fixtures (full, collapsed, shortcut, missing)
- Emphasis edge cases: 3 fixtures
- Escaping: 2 fixtures
- Entities: 3 fixtures
- Link titles: 4 fixtures
- Code spans: 3 fixtures

**Total new fixtures:** ~28

### 4.2 CommonMark Spec Test Suite

**Goal:** Pass all relevant CommonMark 0.31.2 spec tests (excluding GFM-only features).

**Approach:**
1. Download CommonMark spec test JSON.
2. Write a test runner that parses JSON and runs each example.
3. Filter out examples that require GFM extensions (tables, task lists, strikethrough).
4. Compare HTML output (normalize whitespace and formatting differences).
5. Track pass rate and fix failures iteratively.

**Implementation:**
- `crates/typmark-core/tests/commonmark_spec.rs`
- Use `serde_json` to parse spec JSON.
- Allow a "known failures" list for edge cases we defer.

### 4.3 Property Tests

No new property tests required, but ensure existing property tests still pass:
- Parser never panics on arbitrary input.
- Spans remain in-bounds.

---

## 5. Implementation Plan

### 5.1 Recommended Order

1. **Link Reference Definitions** (block-level, no inline changes yet)
   - Parse and store definitions.
   - Add tests for definition parsing.

2. **Reference Links and Images** (inline-level)
   - Implement full/collapsed/shortcut syntax.
   - Look up definitions during inline parsing.
   - Add tests and diagnostics.

3. **Indented Code Blocks** (block-level)
   - Add parsing logic.
   - Ensure no conflict with fenced code blocks.
   - Add tests.

4. **Emphasis Delimiter Rules** (inline-level)
   - Refactor emphasis parsing.
   - Implement delimiter stack algorithm.
   - Add edge case tests.

5. **Entity References** (inline-level)
   - Extend entity parsing for numeric forms.
   - Add tests for valid and invalid entities.

6. **Backslash Escaping** (inline-level)
   - Extend escape table.
   - Add tests for all escapable characters.

7. **Link Title Parsing** (inline-level)
   - Support all three quote styles.
   - Add tests for each style and escaped quotes.

8. **Code Span Refinement** (inline-level)
   - Implement strict CommonMark rules.
   - Add edge case tests.

9. **CommonMark Spec Test Suite Integration**
   - Write spec test runner.
   - Run full suite and identify failures.
   - Fix issues iteratively until pass rate is acceptable (target: >95%).

### 5.2 Milestones

- **M1:** Link reference definitions + reference links/images (foundation for CommonMark link model)
- **M2:** Indented code blocks + emphasis delimiter rules (block and inline improvements)
- **M3:** Entity references + escaping + link titles + code span refinement (completeness)
- **M4:** CommonMark spec test suite integration (validation)

---

## 6. AST Changes

### 6.1 New Variants or Fields

- `Block::LinkReferenceDef` (optional; may store outside AST in resolver)
  - `label: String`
  - `destination: String`
  - `title: Option<String>`
  - `span: Span`

- `Inline::Link` and `Inline::Image`: add `reference_type` field (optional)
  - `ReferenceType::Inline` (default, Phase 0 behavior)
  - `ReferenceType::Full(label)`
  - `ReferenceType::Collapsed`
  - `ReferenceType::Shortcut`

**Alternative:** Keep reference type internal to parser and emit resolved links as `Inline::Link` with `is_reference: bool` flag for debugging/diagnostics.

### 6.2 Backward Compatibility

All Phase 0 AST nodes remain unchanged. New fields or variants are additive only.

---

## 7. Diagnostics (New Codes)

| Code | Severity | Description |
|------|----------|-------------|
| `W_LINK_DEF_UNUSED` | Warning | Link reference definition is never used |
| `W_LINK_DEF_DUP` | Warning | Duplicate link reference definition (first wins) |
| `W_LINK_REF_MISSING` | Warning | Reference link has no matching definition |

**Note:** Existing diagnostic codes from Phase 0 remain unchanged.

---

## 8. HTML Emission Changes

### 8.1 Link Reference Definitions

No HTML output (invisible in rendering).

### 8.2 Reference Links and Images

Emit identical HTML to inline links/images (resolved URL and title).

### 8.3 Indented Code Blocks

Emit without `<figure>` wrapper (simpler than fenced code blocks):
```html
<pre><code>code content
</code></pre>
```

**Rationale:** Indented code blocks have no language or metadata, so the simpler form is appropriate.

---

## 9. Documentation Updates

### 9.1 User-Facing (`spec.md`)

No changes required. CommonMark features are already documented by reference to the CommonMark spec.

### 9.2 Implementation Guide (`core.md`)

- Update §11.4 to mark Phase 1 complete.
- Document link reference definition storage and lookup.
- Document delimiter stack algorithm reference.

### 9.3 Phase Document (`phase1.md`)

This document serves as the Phase 1 implementation guide.

---

## 10. Definition of Done (DoD)

Phase 1 is complete when:

1. ✅ All new features listed in §3 are implemented and tested.
2. ✅ All new golden tests pass (§4.1).
3. ✅ CommonMark spec test suite achieves >95% pass rate (§4.2).
4. ✅ All existing Phase 0 tests still pass (no regressions).
5. ✅ Property tests pass (parser never panics, spans in-bounds).
6. ✅ New diagnostic codes are stable and documented (§7).
7. ✅ HTML emission is deterministic and matches expected output (§8).
8. ✅ Documentation is updated (`core.md` and this document).

---

## 11. Non-Goals (Deferred to Phase 2)

The following features are **NOT** in scope for Phase 1:

- GFM Tables
- GFM Task Lists
- GFM Strikethrough
- GFM Autolinks (extended URL detection)
- GFM Disallowed Raw HTML

These will be addressed in **Phase 2: GFM Extensions**.

---

## 12. Risks and Mitigations

### 12.1 CommonMark Spec Complexity

**Risk:** CommonMark has many edge cases (especially emphasis and link parsing). Achieving 100% compliance may be time-consuming.

**Mitigation:**
- Target >95% pass rate, not 100%.
- Document known edge case failures in a `KNOWN_ISSUES.md` file.
- Prioritize common use cases over obscure edge cases.

### 12.2 Backward Compatibility

**Risk:** Changes to emphasis or link parsing may break Phase 0 behavior.

**Mitigation:**
- Run full Phase 0 test suite after each change.
- Treat any Phase 0 test failure as a blocker.

### 12.3 Performance

**Risk:** Delimiter stack algorithm for emphasis may be slower than current simple approach.

**Mitigation:**
- Profile hot-reload performance before and after.
- Optimize delimiter processing if needed (e.g., avoid repeated scans).

---

## 13. Success Metrics

- **Test Coverage:** All new features covered by golden tests.
- **Spec Compliance:** >95% CommonMark spec test pass rate.
- **Regression:** 0 Phase 0 test failures.
- **Performance:** Hot-reload time increase <10% vs Phase 0.

---

## Appendix A: CommonMark Spec Test JSON Format

CommonMark provides a `spec.json` file with test examples:
```json
{
  "markdown": "input",
  "html": "expected output",
  "example": 123,
  "start_line": 456,
  "end_line": 789,
  "section": "Section Name"
}
```

Test runner should:
1. Parse JSON array.
2. For each example: `parse(markdown)` → `emit_html()` → compare with `html`.
3. Normalize whitespace (trim, collapse multiple spaces).
4. Report pass/fail and example number.

---

## Appendix B: Useful References

- CommonMark Spec 0.31.2: https://spec.commonmark.org/0.31.2/
- CommonMark Spec JSON: https://spec.commonmark.org/0.31.2/spec.json
- CommonMark Dingus (live tester): https://spec.commonmark.org/dingus/
- GitHub Flavored Markdown Spec: https://github.github.com/gfm/

---

## Appendix C: Example Test Fixture

**File:** `tests/fixtures/link_ref_def_basic.tmd`
```markdown
[foo]: /url "title"

This is [a link][foo].
```

**Expected:** `tests/expect/link_ref_def_basic.html`
```html
<p>This is <a href="/url" title="title">a link</a>.</p>
```

---

**Document Version:** 1.0  
**Last Updated:** (current date will be set on commit)  
**Status:** Draft → Active (when implementation begins)