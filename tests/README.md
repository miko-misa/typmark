# Tests

Layout:
- tests/fixtures/<name>.tmd: input source.
- tests/expect/<name>.html: canonical HTML output (Phase 0 mapping).
- tests/expect/<name>.diag.json: diagnostics in a minimal LSP-like JSON format.

Diagnostics format:
[
  {
    "code": "E_TARGET_ORPHAN",
    "severity": "error",
    "range": {
      "start": { "line": 0, "character": 0 },
      "end": { "line": 0, "character": 9 }
    },
    "related": [
      {
        "range": {
          "start": { "line": 0, "character": 0 },
          "end": { "line": 0, "character": 6 }
        }
      }
    ]
  }
]

Notes:
- line/character are 0-based (LSP style).
- Missing .html or .diag.json means that aspect is not asserted.
- HTML output uses 2-space indentation and LF.
