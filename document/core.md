# TypMark Core (Rust) — Design Notes, Pitfalls, and Implementation Guide

This document is for implementing a **single Rust core** that:
1) parses TypMark source into an AST,
2) performs semantic passes (labels, references, diagnostics),
3) emits HTML (optionally sanitized),
and is used unchanged for both **preview (hot reload)** and **build**.

---

## 1. Goals and Non-Goals

### Goals
- One canonical implementation for parse/resolve/emit used by preview + build.
- Deterministic output and diagnostics.
- LSP-friendly diagnostics (spans, severity, stable codes).
- Fast enough for hot reload (incremental-friendly architecture).

### Non-Goals (core)
- UI embellishments (e.g., heading permalink icons) are renderer/editor responsibilities.
- CSS themes and layout beyond semantic HTML.

---

## 2. Pipeline Overview

Recommended pipeline:

1) **Parse**
- input: `&str` source
- output: `Ast` + `Vec<Diagnostic>` + `SourceMap`

2) **Resolve**
- build symbol table (labels → node id)
- attach resolved references
- apply Title/ReferenceText contexts
- enforce recursion cap, title self-reference rule
- output: enriched AST + more diagnostics

3) **Emit HTML**
- map nodes to HTML
- optionally emit structured metadata (e.g., unresolved refs list)

4) **Sanitize (optional / configurable)**
- if enabled, run output through `ammonia` (allowlist sanitizer)

---

## 3. Parsing Strategy (Core Risk Analysis)

TypMark inherits CommonMark + GFM complexity plus:
- Target-line attributes
- `@label[...]` references
- `$` / `$$` math
- `:::` boxes

You have two primary strategies:

### 3.1 Build on an Existing Markdown Parser
Rust option: `comrak` is CommonMark + GFM compatible and tracks CommonMark 0.31.2 by default.

Pros:
- You avoid years of edge-case work.
- You inherit a spec-tested baseline.

Cons:
- TypMark features must be integrated:
  - target-line attributes and `:::` boxes (block layer)
  - `@...` references and Typst math (inline layer)
- Depending on internal representation, you may need a fork or deep hooks.

### 3.2 Full Custom Parser
Pros:
- Full control over grammar and error recovery.
- You can design for incremental parsing from day one.

Cons:
- CommonMark compliance is a project on its own.
- You must implement GFM table parsing, autolinks, etc.

Practical hybrid:
- Implement TypMark as a distinct grammar but aim for a “CommonMark-like” subset first, then add GFM.

---


### 3.3 Project Decision (Parsing Strategy)
This project will use a **full custom parser**.

Rationale:
- Precise control over spans/diagnostics and error recovery.
- Target-line scoping, `:::` boxes, `@...` references, and Typst math are first-class.
- Flexibility for incremental parsing in hot-reload.

Phased compatibility plan:
- Phase 0: Minimal CommonMark-like subset + TypMark features (target-line, refs, math, box, code meta).
- Phase 1: CommonMark baseline completeness.
- Phase 2: GFM extensions (tables, task lists, autolinks, strikethrough).

## 4. AST Design (Make LSP and Rendering Easy)

### 4.1 Spans Everywhere
Every AST node that maps to source should carry:
- byte range: `[start, end)`
- computed line/col via `SourceMap`

This is critical for diagnostics and editor integrations.

### 4.2 Suggested Node Model (Sketch)
- `Document { blocks: Vec<Block> }`
- `Section { level, title: InlineSeq, id: NodeId, label: Option<Label>, children: Vec<Block> }`
- `Block` variants:
  - Paragraph(InlineSeq)
  - List { items: Vec<ListItem> }
  - Quote(Vec<Block>)
  - CodeBlock { lang: Option<String>, attrs: AttrMap, meta: CodeMeta, text: String }
  - Table { ... } (if supported)
  - HtmlBlock(String)
  - Box { fence_len, title: Option<InlineSeq>, attrs: AttrMap, children: Vec<Block> }
  - MathBlock { typst_src: String, label: Option<Label> }
  - HeadingRaw { level, title: InlineSeq } (optional intermediate before Section-building)


  - `CodeMeta` (attached to CodeBlock):
    - `hl`: set of highlighted line numbers (1-based, including blank lines)
    - `line_labels`: map `line_no -> LabelName` for entries written as `N:Label` in `hl`
    - `diff_add`: set of added line numbers
    - `diff_del`: set of deleted line numbers
    - Invariants:
      - `hl ∩ (diff_add ∪ diff_del) = ∅` (else E_CODE_CONFLICT)
      - `diff_add ∩ diff_del = ∅` (else E_CODE_CONFLICT)


- `Inline` variants:
  - Text
  - Emph/Strong/CodeSpan
  - Link { url, title, children }
  - Image { url, title, alt }
  - Ref { label, bracket: Option<InlineSeq>, resolved: Option<TargetId> }
  - MathInline { typst_src }
  - HtmlSpan(String)

### 4.3 Section Tree Construction
If your parser yields headings as blocks, a **post-pass** should:
- convert headings into Section nodes
- place subsequent blocks under the correct Section by heading level rules

---


### 4.4 Span + SourceMap (Base Types)
- `Span { start, end }` uses byte offsets into the original source (half-open [start, end)).
- `SourceMap` stores line start offsets and converts `Span` -> LSP `Range` (0-based line/column).
- For synthetic nodes (e.g., Section), use the heading span as the node span.
- Phase 0 uses byte offsets for `Range.character` (UTF-8). Upgrade to UTF-16 when LSP integration needs it.

### 4.5 Diagnostics Model (Minimum)
- `Diagnostic { range, severity, code, message, related }`
- Codes are stable strings with `E_` / `W_` prefix and live in a central list.
- Use `related` ranges for duplicate labels or cross-node errors.

### 4.6 Initial Diagnostic Codes (v0)
Error codes:
- `E_ATTR_SYNTAX` (attribute list parse error)
- `E_TARGET_ORPHAN` (target line without a block)
- `E_LABEL_DUP` (duplicate label)
- `E_REF_OMIT` (missing bracket text for non-title target)
- `E_REF_BRACKET_NL` (newline inside `[...]`)
- `E_REF_SELF_TITLE` (self-reference in a title)
- `E_REF_DEPTH` (reference display depth > 100)
- `E_MATH_INLINE_NL` (newline inside `$...$`)
- `E_CODE_CONFLICT` (hl/diff overlap or diff_add vs diff_del)

Warning codes:
- `W_REF_MISSING` (unresolved reference target)
- `W_CODE_RANGE_OOB` (out-of-range line in code meta)
- `W_BOX_STYLE_INVALID` (invalid box style value)

## 5. Target-Line Attributes (Parsing and Attachment)

Implement target lines as a parse-time mechanism:

- While parsing blocks within a container:
  - If a line is `{...}` only, parse it as `PendingAttrs`.
  - The next block parsed consumes it, attaching:
    - optional label (#name)
    - key/value attrs

Error recovery:
- If a container ends while `PendingAttrs` exists → emit ERROR diagnostic and discard.

Key risk:
- Scope: the pending attrs must be stored per-container stack frame to avoid leaking out of lists/quotes/boxes.

---


### 5.4 Code Fence Attributes (Language + `{...}` Meta)

Code blocks use the Markdown “info string” after the opening fence. TypMark extends this by allowing an attribute list `{...}` after the language token.

Examples:

- ```c {hl="1,4:printf" diff_add="8" diff_del="7"}
- ``` {hl="1"}   (no language; still allowed)

Parsing rules:
1) Split the fence info line into:
   - optional `lang` token (first non-space token, up to the first space or `{`)
   - optional `{...}` attribute list (must be balanced and on the same line)
2) Parse `{...}` with the same attribute parser as Target-Line Attributes (§5.1–§5.3).
3) Interpret known keys:
   - `hl` (string), `diff_add` (string), `diff_del` (string)
   - unknown keys: store in `attrs` for forward compatibility (or emit W_UNKNOWN_ATTR if you prefer).
4) Parse `hl` / `diff_*` values into normalized line sets:
   - `hl`: supports `N`, `A-B`, `N:Label`
   - `diff_*`: supports `N`, `A-B` only
5) Validate invariants (E_CODE_CONFLICT on overlaps).

Normalization recommendation:
- Convert lists/ranges into a canonical representation (e.g., sorted non-overlapping ranges).
- Keep both a range form (for compact storage) and an expanded form (e.g., a boolean/bitset) only if the code block is small enough; otherwise ranges are sufficient.

Newline handling (important for stable `data-line`):
- Treat `\\r\\n` and `\\n` as line breaks.
- Strip a trailing `\\r` from each line when splitting.
- Preserve empty lines so line numbers match what users see in the source.


## 6. Reference Resolution and Context Rendering

### 6.1 Symbol Table
Build:
- `HashMap<LabelName, NodeId>` for all labeled blocks/sections.

Diagnostics:
- duplicates → ERROR (point to both spans)

### 6.2 Resolve References
For each `Inline::Ref`:
- determine target existence
  - missing → WARNING and keep unresolved marker
- enforce bracket omission rules:
  - if target is not title-bearing and `[...]` missing → ERROR

### 6.3 Title / ReferenceText Contexts
Implement an inline “re-writer” for:
- TitleInlineContext: links and refs behave normally; preserve formatting.
- ReferenceTextContext: same de-linking, but images → alt text.

Recommendation:
- Represent de-linking as a transformation step that returns a new InlineSeq plus diagnostics if needed.
- Do not do it during parsing; do it during resolve/emit to keep responsibilities clean.

### 6.4 Recursion Limit (100)
When generating display text from titles that may include refs:
- track depth and visited labels
- if depth > 100 → ERROR and cut off (emit placeholder text)

### 6.5 Title Self-Reference Check
While resolving a title-bearing element:
- scan its title InlineSeq for references to its own label
- if found → ERROR

---

## 7. Typst Math Rendering in Core

### 7.1 Rendering Target
For HTML output, prefer SVG:
- Typst provides `html.frame` which embeds content as inline SVG.
- Rust crate `typst-svg` provides `svg_html_frame` to export a frame suitable for embedding into HTML.

### 7.2 Caching (Hot Reload Critical)
Do not recompile all math on each keystroke.
- Cache by `(typst_src, math_mode, options_hash)` → `SvgString`
- Consider an LRU cache if documents are large.

### 7.3 Security Boundary
Even if math SVG is generated, treat it as untrusted if you ever allow user-supplied SVG/HTML.
If you run sanitization, ensure your policy keeps required SVG tags/attrs.

---

## 8. Sanitization (When Core Must Do It)

### 8.1 Ammonia
`ammonia` is a whitelist-based HTML sanitizer designed to prevent XSS and layout breaking, and uses `html5ever` for browser-grade parsing.

Implementation notes:
- Use a fixed, versioned allowlist profile for determinism.
- Add only the SVG tags/attrs you need for Typst math.

### 8.2 Suggested Minimal SVG Allowlist for Typst Math
Start with:
- tags: `svg`, `g`, `defs`, `path`, `clipPath`, `use`
- attributes: `xmlns`, `viewBox`, `width`, `height`, `d`, `transform`, `fill`, `stroke`, `stroke-width`, `clip-path`, `id`, `class`
- allow `href`/`xlink:href` only if required; disallow external targets

Explicitly disallow:
- `script`, `foreignObject`
- all `on*` event attributes

### 8.3 Test-Driven Allowlist
Do not guess:
- generate a suite of representative Typst math expressions
- render to SVG
- extract actual tag/attr usage
- lock allowlist to the measured set
- re-run when bumping Typst dependencies

---

## 9. HTML Emission (Deterministic and Tool-Friendly)

### 9.1 ID Placement
Rule:
- any labeled block emits `id="LabelName"` on its root element.
- Sections emit `<section id="...">`, with heading inside.

### 9.2 Unresolved References
Emit a stable marker:
- class: `ref-unresolved`
- data: `data-ref-label="..."`

### 9.3 Avoid Nested Anchors
Because HTML anchors cannot contain anchors:
- in TitleInlineContext and ReferenceTextContext, ensure all links and refs are de-linked to spans.

---


### 9.4 Code Blocks: Normalized HTML Schema (Line-Wrapped)

TypMark code blocks can carry **line-level metadata** (highlight, diff, and line labels). The renderer is easiest to implement if HTML is emitted in a *normalized* form where **each physical line is wrapped**.

Recommended shape (token spans may appear inside each line wrapper):

```html
<figure class="TypMark-codeblock" data-typmark="codeblock" data-lang="c">
  <pre class="TypMark-pre">
    <code class="language-c">
      <span class="line" data-line="1">#include &lt;stdio.h&gt;</span>
      <span class="line" data-line="2"></span>

      <span class="line highlighted"
            data-line="4"
            data-highlighted-line
            id="printf"
            data-line-label="printf">
        int main(void) { /* ... */ }
      </span>

      <span class="line diff del" data-line="7" data-diff="del">i++;</span>
      <span class="line diff add" data-line="8" data-diff="add">int j = i++;</span>
    </code>
  </pre>
</figure>
```

Rules:
- `data-line` is **1-based** and counts blank lines.
- Line splitting MUST be deterministic:
  - split on `\n`
  - remove a trailing `\r` if the source was CRLF
  - if the source ends with a newline, keep the last empty line (so copy/paste and “jump to line” behave predictably)
- Every `.line` wrapper SHOULD be present even when the line is empty (emit an empty element).
- `id="Label"` is placed on the line wrapper when `hl` contains `N:Label`.
- Highlighted lines MUST have `data-highlighted-line` (good interop with common pipelines).
- Diff lines MUST have `data-diff="add|del"` and SHOULD also include `class="diff add|del"` to keep CSS simple.
- Token-level highlighting (colors) is orthogonal: emit nested `<span class="token ...">...</span>` inside each `.line` element as needed.

Why this shape:
- Many code-rendering stacks already reason about “a line wrapper element” and attach attributes to it.
- It makes anchors (`#Label`) trivial and independent of the syntax highlighter.

### 9.5 Syntax Highlight / Code Renderer Interop

TypMark core does not mandate a specific highlighter, but the normalized schema above was chosen to interoperate with common approaches:

- **Shiki-style output / transformers** often attaches classes to line wrapper spans (e.g., `class="line focused"`). This aligns directly with `.line` wrappers. See Shiki transformers docs and examples.  
  References (informational): https://shiki.style/packages/transformers

- **rehype-pretty-code** highlights lines via the meta range syntax and adds `data-highlighted-line` to the affected line `<span>` elements. This is compatible with TypMark’s `data-highlighted-line` convention, and TypMark metadata can be translated into its meta form if you use a rehype pipeline.  
  References (informational): https://rehype-pretty.pages.dev/

- **Prism line-highlight plugin** expects line ranges on the `<pre>` element via `data-line="1,4-6"`. This is useful for simple “background highlight” but cannot represent per-line anchors (`id`) or diff classes by itself. If Prism is used, keep the `.line` wrappers for labels/diff, and optionally add `data-line` to `<pre>` for `hl` lines only.  
  References (informational): https://prismjs.com/plugins/line-highlight/

Additionally, GitHub-flavored Markdown cannot combine diff highlighting and language highlighting in a single fence (it forces ` ```diff `). TypMark avoids this by keeping diff as metadata while retaining the language token.  
Reference (informational): https://github.com/github-linguist/linguist/discussions/5758


Practical integration pattern (recommended):
- Do NOT generate “final HTML” first and then attempt to run a highlighter over it (you will lose stable line wrappers or accidentally nest wrappers).
- Instead, generate tokenized output per line (from the highlighter) and *place tokens inside* the `.line` wrappers you control.
  - In JS land, this is natural with Shiki token APIs.
  - In Rust-only deployments, consider a Rust-native highlighter (e.g., syntect) while keeping the same `.line` wrapper schema.

### 9.6 Sanitization Allowlist (If Core Sanitizes HTML)

If core performs sanitization (e.g., for embedded HTML or unsafe Typst SVG), ensure the allowlist includes the attributes required by the normalized schema:
- elements: `figure`, `pre`, `code`, `span`
- attributes: `id`, `class`, `data-line`, `data-highlighted-line`, `data-diff`, `data-line-label`, `data-lang`

If sanitization strips these, line anchors and styling will break.


## 10. Diagnostics and Editor Integration (LSP-Compatible)

Adopt a diagnostic structure aligned with LSP:
- `range`, `severity`, `code`, `message`, optional `relatedInformation`.

Core MUST produce:
- stable `code` identifiers (e.g., `E_LABEL_DUP`, `E_REF_OMIT`, `W_REF_MISSING`)
- primary span for highlighting
- optional related spans for duplicates

---

## 11. Testing Strategy

### 11.1 Golden Tests
- source → HTML snapshots (per feature)
- diagnostics snapshots (ensure stable codes and spans)

### 11.2 Fuzz / Property Tests
- parser robustness (no panics)
- span monotonicity and bounds

### 11.3 Spec Compliance
If you target CommonMark/GFM compatibility:
- run or adapt CommonMark test suites where possible.
- if using comrak, validate your extensions do not break baseline semantics.

---


### 11.4 Implementation Order (v0)
- Phase 0 (Minimum + TypMark):
  - Core types: `Span`, `SourceMap`, `Diagnostic`, AST definitions.
  - Block parser: container stack + target-line attrs; support headings, paragraphs, lists, blockquotes, code fences, boxes, math blocks.
  - Inline parser: refs, inline math, code spans, emphasis, links/images; enforce newline rules.
  - Section builder: heading -> section tree conversion.
  - Resolver: label table, ref resolution, bracket omission rules, title self-reference, depth limit.
  - HTML emitter: deterministic output + code line wrappers schema.
- Phase 1: CommonMark baseline completeness for remaining blocks/inlines.
- Phase 2: GFM extensions (tables, task lists, autolinks, strikethrough).

### 11.5 Phase Gates (DoD)
- Each phase adds golden HTML + diagnostics snapshots for every implemented MUST/ERROR rule.
- Property tests ensure parser never panics and spans stay in-bounds.
- No phase completes until diagnostic codes are stable and documented.
- Phase 1 requires CommonMark spec tests to pass (excluding explicitly skipped cases).

### 11.5.1 Diagnostic Code Registry
Documented diagnostic codes (stable identifiers):
- Errors: `E_ATTR_SYNTAX`, `E_TARGET_ORPHAN`, `E_LABEL_DUP`, `E_REF_OMIT`, `E_REF_BRACKET_NL`,
  `E_REF_SELF_TITLE`, `E_REF_DEPTH`, `E_MATH_INLINE_NL`, `E_CODE_CONFLICT`.
- Warnings: `W_REF_MISSING`, `W_CODE_RANGE_OOB`, `W_BOX_STYLE_INVALID`.


### 11.6 Phase 0 Minimum Feature Set
- Blocks: headings, paragraphs, lists, blockquotes, code fences, boxes, math blocks.
- Inlines: text, emphasis, code span, link/image, ref, inline math.
- Target-line attrs + label attachment with container scoping.
- Section tree construction from headings.
- Resolver: label uniqueness, ref resolution, bracket omission rule, title self-reference, depth limit.
- HTML emission: deterministic output + code line wrapper schema.

### 11.7 Initial Test Matrix (v0)
Golden HTML:
- `box_basic`, `box_titled`, `box_nested`.
- `math_inline`, `math_block`.
- `code_lines_hl`, `code_lines_diff`, `code_line_label`.
- `target_line_section`, `target_line_list_scope`.

Diagnostics:
- `E_TARGET_ORPHAN`, `E_LABEL_DUP`, `E_REF_OMIT`.
- `E_REF_SELF_TITLE`, `E_REF_DEPTH`, `E_REF_BRACKET_NL`.
- `E_MATH_INLINE_NL`, `E_CODE_CONFLICT`.
- `W_REF_MISSING`, `W_CODE_RANGE_OOB`, `W_BOX_STYLE_INVALID`.

Property tests:
- Parser never panics on random input.
- All spans are in-bounds and monotonic within a node.

## 12. Common Failure Modes (Checklist)

- Target-line attrs leaking across containers (lists/quotes/boxes).
- `$` interpreted as math in currency contexts.
- `:::` fences interacting with code fences (must not parse inside code blocks).
- Reference recursion via mutual references (A↔B) not caught until deep.
- Sanitizer deleting required SVG attributes (math disappears).
- Diagnostics ranges pointing to the wrong place (bad editor UX).

---

## Appendix: Key External Specs / Crates (informational)

```text
CommonMark Spec (current)
GitHub Flavored Markdown (GFM) Spec
comrak (Rust CommonMark+GFM parser)
Typst: html.frame and SVG documentation
typst-svg crate: svg_html_frame
ammonia crate: HTML sanitizer (allowlist, html5ever-based)
Language Server Protocol (LSP) diagnostics model
```
