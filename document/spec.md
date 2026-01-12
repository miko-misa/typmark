# TypMark v1.0 — Language Specification (User-Facing)

This document specifies **how to write** TypMark documents. TypMark is a Markdown-based document format with:
- CommonMark + GFM compatibility as the baseline.
- A **target-line attribute system** for labels and metadata.
- **Cross-references** with strict rules.
- **Typst-syntax math** via `$...$` and `$$...$$`.
- A fenced **box** block using `:::`.

> Normative keywords: MUST / MUST NOT / SHOULD / MAY.

---

## 1. Baseline Markdown

TypMark adopts:
- **CommonMark** as the base Markdown model (blocks + inlines).
- **GitHub Flavored Markdown (GFM)** extensions such as tables, task lists, strikethrough, and autolinks.

TypMark only overrides behavior where this specification explicitly says so.

Compatibility level:
- Phase 0–1: CommonMark baseline.
- Phase 2: GFM extensions.

Note on autolinks:
- GFM autolink literals are enabled in Phase 2, so bare URLs/emails are linked even when
  CommonMark's autolink examples would render them as plain text.

CommonMark notes (baseline behavior):
- Raw HTML is parsed according to CommonMark's inline/block rules.
- Backslash escapes and entity decoding follow CommonMark.

---

## 2. Core Concepts

### 2.1 Blocks and Inlines
- A document consists of **block** elements (paragraphs, lists, headings, tables, code blocks, etc.).
- Many blocks contain **inline** content (text, emphasis, links, images, code spans, math, references, etc.).

### 2.2 Sections
- Headings form **Sections**.
- A Section begins at a heading and extends until the next heading of **the same or higher level**.
- **References to headings reference the Section**, not only the heading line.

---

## 3. Target-Line Attributes

TypMark uses a single, uniform mechanism to attach labels and metadata: **target lines**.

### 3.1 Target Line Definition
A **target line** is a line that consists only of an attribute list:

```md
{#label key=value key2="value 2"}
```

A target line attaches its attributes to the **next block** (the “target block”).

### 3.2 Scope Rule (No Leaking)
A target line applies only to the next block **within the same container**.
- Inside a list item → applies to the next block inside that list item.
- Inside a blockquote → applies to the next block inside that quote.
- It MUST NOT apply to blocks outside the current container.

### 3.3 Errors
- If a target line is not followed by a targetable block in the same container, it is an **ERROR**.

### 3.4 Attribute List Grammar
An attribute list is:

- `{ item (space item)* }`

Items are:
- `#LabelName` (at most one per list)
- `key=value` pairs (zero or more)

Values:
- `value` MAY be unquoted if it contains no spaces.
- Otherwise it MUST be quoted with `"`.

---

## 4. Labels

### 4.1 Label Name
A label name MUST match:

- `[A-Za-z0-9_-]+`

Rules:
- Digits are allowed.
- Consecutive `-` and `_` are allowed.
- Labels are **case-sensitive** (no normalization).

### 4.2 Uniqueness
A label MUST be unique within a single document.
- Duplicate labels are an **ERROR**.

### 4.3 Label Targets
Labels can be attached (via target line) to **any block element**, including:
- Sections (via the heading block)
- Box blocks
- Block math
- Tables
- Paragraphs, lists, quotes, code blocks, etc.

---

## 5. Cross-References

TypMark uses Typst-like reference syntax.

### 5.1 Reference Forms
- `@LabelName`
- `@LabelName[InlineContent]`

References are not recognized:
- When the preceding character is alphanumeric or one of `+ - . _`.
- When the token immediately before `@` contains `/` or `\` (URLs/paths).

### 5.2 Title-Bearing Elements
Only these are **title-bearing**:
- **Section** (its heading provides a title)
- **TitleBox** (a box with a title)

### 5.3 Brackets Omission Rule (Strict)
- For **title-bearing** targets: `@label` is allowed (brackets MAY be omitted).
- For **non-title** targets: brackets are **mandatory**.
  - Using `@label` without `[...]` is an **ERROR**.

### 5.4 Display Text Resolution
If `@label[InlineContent]` is used:
- The bracket text MUST be used as the display text (it overrides the target title).

If `@label` is used (only allowed for title-bearing targets):
- The display text is the target element’s title.

### 5.5 Bracket Text Rules
Inside `[...]`:
- Only **inline** content is allowed.
- Newlines are **forbidden** (newline → **ERROR**).
- Backslash escapes are used:
  - `\[` → `[`
  - `\]` → `]`
  - `\\` → `\`

References and links may appear in `[...]`, but see the context rules below (they may be “de-linked”).

### 5.6 Missing Reference Targets
If a referenced label does not exist:
- This is a **WARNING** (not an error).
- Renderers MAY choose how to display unresolved references, but they SHOULD expose the unresolved state (e.g., CSS class, data attribute).

---

## 6. Inline Context Rules

TypMark defines three inline evaluation contexts.

### 6.1 Normal Inline Context
Default context for document bodies:
- Links and references behave normally.

### 6.2 Title Inline Context
Applies to:
- Section titles (headings)
- Box titles

Rules in Title Inline Context:
- Links and references behave normally.
- Inline formatting (emphasis, code spans, math, etc.) is preserved.
- Images are allowed.

### 6.3 Reference Text Context
Applies when generating the visible display text for a reference:
- Links and references MUST be **de-linked** (rendered as spans).
- Images MUST be replaced by their **alt text**.
- Inline formatting is preserved.

---

## 7. Self-Reference Rules

- Self-reference is forbidden **only inside titles** of title-bearing elements:
  - A Section title MUST NOT reference its own label.
  - A TitleBox title MUST NOT reference its own label.
  - Violations are **ERROR**.

- Self-references outside titles are allowed.

Additionally:
- Reference display text generation has a recursion depth limit of **100**.
  - Exceeding the limit is an **ERROR**.

---

## 8. Typst Math

TypMark supports math using **Typst syntax**.

### 8.1 Inline Math
- `$ ... $` denotes inline math.
- Inline math MUST NOT contain newlines (newline → **ERROR**).

### 8.2 Block Math
- `$$ ... $$` denotes block math.
- Block math can appear inside other containers (lists, quotes, boxes, etc.).
- Block math can be labeled via a target line and can be referenced.

---

## 9. Box Block (`:::`)

TypMark supports a fenced box block.

### 9.1 Syntax
A box is defined by a fence of 3+ colons.

Example (titled box):
```md
{#box-note bg="#f8f8f8" border-style="solid" border-width="1px" border-color="#ccc" title-bg="#eee"}
::: box Note
This is inside the box.
:::
```

Rules:
- The opening fence is `:::` (or more colons) followed by `box` and optional title text.
- The closing fence is a line consisting only of colons with length **>= opening fence**.
- Boxes MAY nest.

### 9.2 Title
- If the opening line contains a title, the box becomes a **TitleBox** (title-bearing element).
- Box titles are parsed using **Title Inline Context** rules.

### 9.3 Style Keys
Box styles are specified via the **target line** attribute list using `key=value`.
- Invalid values produce a **WARNING** and renderers MUST fall back to defaults.

(Exact supported keys are renderer/core-defined, but the core SHOULD preserve all key/value pairs.)

Core Phase 0 validation (initial):
- `bg`, `title-bg`, `border-color` must be `#RGB` or `#RRGGBB`.
- `border-width` must be an integer or an integer with `px` (e.g., `1` or `1px`).
- `border-style` must be one of `solid`, `dashed`, `dotted`, `double`, `none`.
- Other keys are preserved and not validated.

---

## 10. Images

TypMark allows images as inline elements, including inside titles:
- `![alt](url "title")` per CommonMark.

Special rule:
- In **Reference Text Context**, images are replaced with their **alt** text.

---


## 11. Code Blocks (Extended)

TypMark inherits fenced code blocks from Markdown (CommonMark/GFM). A fenced code block begins with a line of three backticks or tildes and ends with a matching fence.

```md
```c
int main(void) { return 0; }
```
```

- The optional language token (`c` above) is a *hint* for syntax highlighting (text color). Highlighting is a renderer concern; TypMark only standardizes what metadata exists and how it is interpreted.

### 11.1 Fence Line: Language + Attribute List

TypMark extends the *fence opening line* (the first ```/~~~ line) by allowing an optional **attribute list** after the language token.

```md
```c {hl="1,4:printf" diff_del="7" diff_add="8"}
...
```
```

- The attribute list uses the same grammar as Target-Line Attributes (see §3.4).
- A code block MAY omit the language token.
- A code block MAY include a block-level label (e.g., `{#my_code ...}`), but **code-line labels do not require the block itself to have a label**.

### 11.2 `hl="..."`: Highlighted Lines + Line Labels

`hl` specifies **highlighted lines** (background emphasis) and MAY attach a **label** to a highlighted line.

The value is a comma-separated list of entries. Line numbers are **1-based** and include blank lines.

Each entry is one of:

- `N` — highlight line N
- `A-B` — highlight lines A through B (A ≤ B)
- `N:Label` — highlight line N and declare `Label` as a **line target** for cross-reference

Example:

```md
```c {hl="1,4:printf,7-9"}
#include <stdio.h>

int main(void) {
  printf("hello"); // <- line 4 (labeled `printf`)
}
```
```

#### 11.2.1 Label rules (for `N:Label`)

- `Label` MUST satisfy the global label syntax (§4.1): `[A-Za-z0-9_-]+` (case-sensitive; `-` and `_` may repeat).
- `Label` MUST be unique across the document (same uniqueness rule as other labels).
- A code line is NOT a “titled element”, so reference text cannot be omitted:
  - ✅ `@printf[printf line]`
  - ❌ `@printf` (ERROR)

### 11.3 `diff_add="..."` / `diff_del="..."`: Diff Decoration

Diff decoration marks specific lines as “added” or “deleted”.

- `diff_add` — lines to render as additions (`+` in the gutter, typically)
- `diff_del` — lines to render as deletions (`-` in the gutter, typically)

The TypMark source MUST NOT prefix the code text with `+`/`-`. The renderer is responsible for any visual symbols.

The value is a comma-separated list of `N` or `A-B` (no labels in diff values in v1.0).

Example:

```md
```c {diff_del="7" diff_add="8,10-12"}
...
```
```

### 11.4 Coexistence and Conflicts (ERROR)

- Language hint, `hl`, and `diff_*` MAY coexist.
- However, for any given line:
  - A line MUST NOT be both highlighted (`hl`) and diff-marked (`diff_add`/`diff_del`). This is an ERROR.
  - A line MUST NOT be in both `diff_add` and `diff_del`. This is an ERROR.

### 11.5 Out-of-range Lines (WARNING)

If `hl` or `diff_*` refers to a line number greater than the actual number of lines in the code block, implementations SHOULD emit a WARNING and ignore that entry. (This is common during editing and hot-reload previews.)

### 11.6 Recommended HTML Mapping (Informational)

This spec is user-facing, but implementers benefit from a stable mapping for previews, anchors, and styling.

A tool-friendly mapping is:

- Wrap each rendered line in an element with `data-line="N"`.
- Put the line label on that element as an `id="Label"` (so `#Label` anchors work).
- Mark highlighted lines with `data-highlighted-line`.
- Mark diff lines with `data-diff="add|del"`.

This aligns well with common code-rendering stacks (e.g., Shiki-based pipelines and rehype-pretty-code-style outputs).

### 11.7 Full Example (All Features Together)

```md
Here is the key change at @printf[the printf line].

```c {#hello_world hl="1,4:printf" diff_del="7" diff_add="8"}
#include <stdio.h>

int main(void) {
  printf("hello, world!\n");   // highlighted and labeled: `printf`
  int i = 42;
  --i;
  i++;                         // diff_del
  int j = i++;                 // diff_add
}
```
```

What this means:

- The code block language is `c`, so the renderer MAY apply syntax highlighting.
- Line 1 and line 4 are highlighted. Line 4 also declares a label `printf`.
- Line 7 is marked as a deletion, line 8 as an addition.
- The block itself is labeled `hello_world` (optional). You can reference it like any other unlabeled block target (but it has no title, so `@hello_world[...]` MUST include text).

### 11.8 Common Mistakes (Diagnostic Behavior)

These are considered **ERROR**:

- Overlapping highlight and diff on the same line:
  - `hl="4"` and `diff_add="4"`
- The same line appears in both `diff_add` and `diff_del`.
- Invalid ranges:
  - `hl="5-3"` (A > B)
- Invalid label syntax:
  - `hl="4:my label"` (space is not allowed in labels)
- Omitting reference text to a code line label:
  - `@printf` (code lines are not titled elements)

These are **WARNING** (recommended):

- Out-of-range line numbers:
  - `hl="999"` for a small code block (ignored)



## 12. Diagnostics

TypMark distinguishes:

### 11.1 ERROR (must fail parse/compile or mark invalid)
Examples:
- Unparseable constructs.
- Duplicate labels.
- Missing mandatory bracket text for non-title targets (`@x` where `@x[...]` is required).
- Newlines inside `[...]`.
- Title self-reference.
- Recursion depth > 100.
- Target line with no following target block.

### 11.2 WARNING (valid but potentially problematic)
Examples:
- Reference target not found.
- Invalid box style value (falls back to default).

---

## Appendix: Reference Documents (informational)

Additional implementation-oriented references for code blocks:
- Shiki transformers: https://shiki.style/packages/transformers
- rehype-pretty-code: https://rehype-pretty.pages.dev/
- Prism line-highlight: https://prismjs.com/plugins/line-highlight/
- GitHub diff + language limitation discussion (motivating metadata-based diff): https://github.com/github-linguist/linguist/discussions/5758


```text
CommonMark Spec (current)
GitHub Flavored Markdown (GFM) Spec
Typst documentation: html.frame, SVG
```
