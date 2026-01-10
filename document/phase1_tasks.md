# TypMark Phase 1 — Implementation Task List

## Overview

This document breaks down Phase 1 implementation into concrete, actionable tasks.
Each task should be completable in 1-3 hours and result in a commit.

**Phase 1 Goal:** CommonMark baseline completeness
**Status:** Not Started
**Estimated Total Time:** 40-50 hours

---

## Task Breakdown by Milestone

### Milestone 1: Link Reference Infrastructure (8-10 hours)

#### Task 1.1: Add Link Reference Definition Data Structures
**Estimated Time:** 1 hour

- [ ] Add `LinkReferenceDef` struct to `ast.rs`
  - Fields: `label: String`, `destination: String`, `title: Option<String>`, `span: Span`
- [ ] Add `link_defs: HashMap<String, LinkReferenceDef>` to `Document` or `ParseResult`
- [ ] Update `parse()` to return link definitions

**Deliverable:** Commit with updated AST types

---

#### Task 1.2: Parse Link Reference Definitions (Block Level)
**Estimated Time:** 2-3 hours

- [ ] Add link reference definition detection in `parser.rs` block loop
  - Detect pattern: `[label]:` at start of line (0-3 space indent)
  - Parse label (case-insensitive normalization)
  - Parse destination (with or without `<...>`)
  - Parse optional title (all three quote styles)
  - Handle multi-line titles
- [ ] Store in `link_defs` HashMap (lowercase key for case-insensitive lookup)
- [ ] Handle duplicates: first definition wins, emit `W_LINK_DEF_DUP`

**Deliverable:** Commit with link ref def parsing

---

#### Task 1.3: Add Link Reference Definition Tests
**Estimated Time:** 1 hour

- [ ] Create `tests/fixtures/link_ref_def_basic.tmd`
- [ ] Create `tests/fixtures/link_ref_def_multiline.tmd`
- [ ] Create `tests/fixtures/link_ref_def_case_insensitive.tmd`
- [ ] Create `tests/fixtures/link_ref_def_duplicate.tmd` (diagnostics test)
- [ ] Create corresponding `.html` expectation files (empty output)
- [ ] Create `tests/expect/link_ref_def_duplicate.diag.json` for duplicate warning

**Deliverable:** Commit with link ref def tests

---

#### Task 1.4: Implement Reference Link Parsing (Full Syntax)
**Estimated Time:** 2-3 hours

- [ ] Modify inline parser's `[` handling to detect reference syntax
  - After parsing link text, check for `][label]` (full reference)
  - Look up `label` in `link_defs` (case-insensitive)
  - If found, create `Link` with resolved URL and title
  - If not found, emit `W_LINK_REF_MISSING` and render as text
- [ ] Update `Inline::Link` to track whether it's a reference (optional field for debugging)

**Deliverable:** Commit with full reference link parsing

---

#### Task 1.5: Implement Collapsed and Shortcut Reference Links
**Estimated Time:** 1-2 hours

- [ ] Detect `[]` after link text (collapsed reference: label = link text)
- [ ] Detect no `[]` after link text and no `(` (shortcut reference: label = link text)
- [ ] Look up normalized link text in `link_defs`
- [ ] Create resolved `Link` if found, else text

**Deliverable:** Commit with collapsed/shortcut reference links

---

#### Task 1.6: Add Reference Link Tests
**Estimated Time:** 1 hour

- [ ] Create `tests/fixtures/link_ref_full.tmd`
- [ ] Create `tests/fixtures/link_ref_collapsed.tmd`
- [ ] Create `tests/fixtures/link_ref_shortcut.tmd`
- [ ] Create `tests/fixtures/link_ref_missing.tmd` (diagnostics test)
- [ ] Create corresponding `.html` expectation files
- [ ] Create `tests/expect/link_ref_missing.diag.json` for missing warning

**Deliverable:** Commit with reference link tests

---

#### Task 1.7: Implement Reference Images (All Forms)
**Estimated Time:** 1 hour

- [ ] Apply same logic as reference links to `![alt][ref]` syntax
- [ ] Support full, collapsed, and shortcut forms
- [ ] Look up in `link_defs` (same map as links)
- [ ] Create `tests/fixtures/image_ref_{full,collapsed,shortcut}.tmd`
- [ ] Create corresponding `.html` expectation files

**Deliverable:** Commit with reference image support and tests

---

### Milestone 2: Indented Code Blocks + Emphasis (6-8 hours)

#### Task 2.1: Implement Indented Code Block Parsing
**Estimated Time:** 2-3 hours

- [ ] Add detection for 4+ space or 1 tab indentation in block parser
- [ ] Parse consecutive indented lines as code block
- [ ] End block when:
  - Non-indented line encountered
  - Blank line followed by non-indented line
  - Container ends
- [ ] Ensure cannot interrupt paragraph (blank line required)
- [ ] Store as `CodeBlock` with `lang: None` and no metadata

**Deliverable:** Commit with indented code block parsing

---

#### Task 2.2: Add Indented Code Block Tests
**Estimated Time:** 1 hour

- [ ] Create `tests/fixtures/indented_code_basic.tmd`
- [ ] Create `tests/fixtures/indented_code_after_paragraph.tmd`
- [ ] Create `tests/fixtures/indented_code_in_list.tmd`
- [ ] Create corresponding `.html` expectation files
- [ ] Verify output is `<pre><code>...</code></pre>` without `<figure>`

**Deliverable:** Commit with indented code block tests

---

#### Task 2.3: Refactor Emphasis Parsing for Delimiter Stack
**Estimated Time:** 3-4 hours

- [ ] Study CommonMark spec §6.2 (Emphasis and strong emphasis)
- [ ] Implement delimiter run detection:
  - Left-flanking: not followed by Unicode whitespace, and either not followed by punctuation or preceded by whitespace/punctuation
  - Right-flanking: not preceded by Unicode whitespace, and either not preceded by punctuation or followed by whitespace/punctuation
- [ ] Implement "can open" and "can close" rules (special handling for `_`)
- [ ] Build delimiter stack during inline parsing
- [ ] Process delimiters after inline parse to match openers and closers
- [ ] Handle nested emphasis correctly

**Deliverable:** Commit with refactored emphasis parsing

---

#### Task 2.4: Add Emphasis Edge Case Tests
**Estimated Time:** 1 hour

- [ ] Create `tests/fixtures/emph_flanking_punctuation.tmd`
- [ ] Create `tests/fixtures/emph_underscore_intraword.tmd`
- [ ] Create `tests/fixtures/emph_nested_complex.tmd`
- [ ] Create corresponding `.html` expectation files
- [ ] Verify Phase 0 emphasis tests still pass

**Deliverable:** Commit with emphasis edge case tests

---

### Milestone 3: Inline Refinements (8-10 hours)

#### Task 3.1: Extend Backslash Escaping to Full CommonMark Set
**Estimated Time:** 1 hour

- [ ] Update escape handling in inline parser
- [ ] Support all ASCII punctuation per CommonMark:
  `! " # $ % & ' ( ) * + , - . / : ; < = > ? @ [ \ ] ^ _ ` { | } ~`
- [ ] Backslash before non-escapable character = literal backslash
- [ ] Create `tests/fixtures/escape_all_punctuation.tmd`
- [ ] Create `tests/fixtures/escape_non_escapable.tmd`
- [ ] Create corresponding `.html` expectation files

**Deliverable:** Commit with full backslash escaping support and tests

---

#### Task 3.2: Implement Numeric Entity References
**Estimated Time:** 2 hours

- [ ] Extend entity parsing in inline parser
- [ ] Detect `&#` followed by digits (decimal) or `x`/`X` + hex digits
- [ ] Must end with `;`
- [ ] Convert to Unicode character if valid code point (0x0 - 0x10FFFF, excluding surrogates)
- [ ] If invalid, treat as literal text (no error)
- [ ] Create `tests/fixtures/entity_numeric_decimal.tmd`
- [ ] Create `tests/fixtures/entity_numeric_hex.tmd`
- [ ] Create `tests/fixtures/entity_invalid.tmd`
- [ ] Create corresponding `.html` expectation files

**Deliverable:** Commit with numeric entity reference support and tests

---

#### Task 3.3: Extend Link Title Parsing to All Quote Styles
**Estimated Time:** 2 hours

- [ ] Update link destination/title parser in inline parser
- [ ] Support `"title"` (already done in Phase 0)
- [ ] Support `'title'` (single quotes)
- [ ] Support `(title)` (parentheses)
- [ ] Handle escaped quotes inside titles (`\"`, `\'`)
- [ ] Support multi-line titles (for link ref defs)
- [ ] Create `tests/fixtures/link_title_double_quotes.tmd`
- [ ] Create `tests/fixtures/link_title_single_quotes.tmd`
- [ ] Create `tests/fixtures/link_title_parens.tmd`
- [ ] Create `tests/fixtures/link_title_escaped.tmd`
- [ ] Create corresponding `.html` expectation files

**Deliverable:** Commit with extended link title parsing and tests

---

#### Task 3.4: Refine Code Span Parsing (Strict CommonMark Rules)
**Estimated Time:** 2 hours

- [ ] Update code span parsing in inline parser
- [ ] Support multiple backticks (e.g., ``` `` code `` ```)
- [ ] Count opening backticks, match closing sequence of same length
- [ ] Apply space-stripping rule:
  - If both first and last character are spaces, strip one from each end
- [ ] Convert line endings inside code span to spaces
- [ ] Create `tests/fixtures/code_span_multiple_backticks.tmd`
- [ ] Create `tests/fixtures/code_span_space_stripping.tmd`
- [ ] Create `tests/fixtures/code_span_line_endings.tmd`
- [ ] Create corresponding `.html` expectation files

**Deliverable:** Commit with refined code span parsing and tests

---

#### Task 3.5: Add Unused Link Definition Warning
**Estimated Time:** 1 hour

- [ ] After parsing and resolving, check which link definitions were used
- [ ] Track usage during reference link/image resolution
- [ ] Emit `W_LINK_DEF_UNUSED` for each unused definition
- [ ] Create `tests/fixtures/link_ref_def_unused.tmd` (diagnostics test)
- [ ] Create `tests/expect/link_ref_def_unused.diag.json`

**Deliverable:** Commit with unused link definition warning and test

---

### Milestone 4: CommonMark Spec Compliance (10-15 hours)

#### Task 4.1: Set Up CommonMark Spec Test Infrastructure
**Estimated Time:** 2 hours

- [ ] Download CommonMark 0.31.2 `spec.json`
- [ ] Add to project (e.g., `tests/commonmark/spec.json`)
- [ ] Create `crates/typmark-core/tests/commonmark_spec.rs`
- [ ] Write test runner that:
  - Parses JSON array
  - Filters GFM-only examples (if needed)
  - Runs `parse()` → `emit_html()` for each example
  - Normalizes whitespace and compares output
  - Reports pass/fail with example number
- [ ] Add `serde_json` dependency for JSON parsing

**Deliverable:** Commit with spec test infrastructure

---

#### Task 4.2: Run Initial CommonMark Spec Test Suite
**Estimated Time:** 1 hour

- [ ] Run `cargo test commonmark_spec`
- [ ] Collect results: pass/fail count, failing example numbers
- [ ] Create `COMMONMARK_STATUS.md` documenting initial pass rate
- [ ] Identify categories of failures (emphasis, links, etc.)

**Deliverable:** Commit with spec test runner and status document

---

#### Task 4.3: Fix High-Priority CommonMark Failures (Iterative)
**Estimated Time:** 6-10 hours

This is an iterative task. For each category of failures:

- [ ] Analyze failing examples
- [ ] Identify root cause (parser bug, missing feature, edge case)
- [ ] Fix implementation
- [ ] Verify fix resolves failures without breaking Phase 0 tests
- [ ] Update `COMMONMARK_STATUS.md` with new pass rate

**Priority order:**
1. Links and images (reference and inline)
2. Emphasis and strong
3. Code spans
4. Escaping and entities
5. Headings and paragraphs
6. Lists and blockquotes

**Target:** >95% pass rate

**Deliverable:** Multiple commits, one per category or logical fix

---

#### Task 4.4: Document Known Issues and Edge Cases
**Estimated Time:** 1 hour

- [ ] For remaining failures (<5%), document in `KNOWN_ISSUES.md`
- [ ] Explain why each is not fixed (e.g., obscure edge case, low priority)
- [ ] Link to CommonMark example numbers
- [ ] Decide if any are blockers for Phase 1 completion

**Deliverable:** Commit with known issues documentation

---

### Milestone 5: Diagnostics and Polish (4-6 hours)

#### Task 5.1: Add New Diagnostic Codes
**Estimated Time:** 1 hour

- [ ] Add to `diagnostic.rs`:
  - `W_LINK_DEF_UNUSED`
  - `W_LINK_DEF_DUP`
  - `W_LINK_REF_MISSING`
- [ ] Update `lib.rs` exports
- [ ] Document codes in `diagnostic.rs` comments

**Deliverable:** Commit with new diagnostic codes

---

#### Task 5.2: Verify All Phase 0 Tests Still Pass
**Estimated Time:** 1 hour

- [ ] Run full test suite: `cargo test --workspace`
- [ ] Fix any regressions introduced by Phase 1 changes
- [ ] Ensure property tests still pass (no panics, spans in-bounds)

**Deliverable:** Commit with regression fixes (if any)

---

#### Task 5.3: Update Documentation
**Estimated Time:** 1 hour

- [ ] Update `document/core.md` §11.4 to mark Phase 1 complete
- [ ] Document link reference definition storage strategy
- [ ] Document delimiter stack algorithm reference
- [ ] Update `phase1.md` status to "Complete"

**Deliverable:** Commit with documentation updates

---

#### Task 5.4: Performance Benchmarking
**Estimated Time:** 2 hours

- [ ] Create benchmark suite (e.g., using `criterion`)
- [ ] Benchmark representative documents (small, medium, large)
- [ ] Compare Phase 1 vs Phase 0 performance
- [ ] Verify hot-reload time increase is <10%
- [ ] Document results in `BENCHMARKS.md`
- [ ] If performance is degraded >10%, profile and optimize

**Deliverable:** Commit with benchmarks and performance report

---

#### Task 5.5: Final Integration Testing
**Estimated Time:** 1 hour

- [ ] Run all tests: `cargo test --workspace`
- [ ] Verify CommonMark spec pass rate >95%
- [ ] Verify no Phase 0 regressions
- [ ] Verify all new diagnostics are generated correctly
- [ ] Manual smoke test: parse various documents and inspect HTML output

**Deliverable:** Sign-off that Phase 1 is complete

---

## Summary

**Total Tasks:** 29
**Total Estimated Time:** 40-50 hours
**Milestones:** 5

### Milestone Breakdown
- M1: Link Reference Infrastructure (8-10 hours)
- M2: Indented Code Blocks + Emphasis (6-8 hours)
- M3: Inline Refinements (8-10 hours)
- M4: CommonMark Spec Compliance (10-15 hours)
- M5: Diagnostics and Polish (4-6 hours)

---

## Task Status Legend

- [ ] Not Started
- [🚧] In Progress
- [✅] Complete
- [⏭️] Deferred

---

## Notes for AI Agent

1. **Work in Order:** Complete tasks in the order listed. Dependencies flow downward within each milestone.

2. **Commit Granularity:** Each task should result in 1-2 commits. Use conventional commit format:
   - `feat(parser): add link reference definition parsing`
   - `test(parser): add link reference definition tests`
   - `fix(parser): handle escaped quotes in link titles`

3. **Test Coverage:** Never commit implementation without corresponding tests. Tests come immediately after or in the same commit as the feature.

4. **Regression Prevention:** After each commit, run `cargo test --workspace` to ensure no Phase 0 tests break.

5. **Diagnostics First:** When adding a feature that can error/warn, implement the diagnostic code in the same commit as the feature.

6. **Document as You Go:** Update inline code comments and doc comments. Leave the big documentation update for M5.

7. **Ask for Help:** If a task is unclear or seems blocked, stop and ask the user for guidance rather than making assumptions.

8. **Iterate on CommonMark Failures:** Task 4.3 is iterative and may take multiple sessions. Focus on categories with the most failures first.

---

**Last Updated:** (will be set on commit)
**Document Version:** 1.0