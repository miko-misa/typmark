use typmark_core::{emit_html, parse, resolve};

#[test]
fn test_list_item_with_two_tabs() {
    // CommonMark example: "-\t\tfoo" should produce an indented code block with "  foo".
    let input = "-\t\tfoo\n";

    let parsed = parse(input);
    let resolved = resolve(
        parsed.document,
        input,
        &parsed.source_map,
        parsed.diagnostics,
        &parsed.link_defs,
    );
    let html = emit_html(&resolved.document.blocks);

    // Expected: list item containing an indented code block.
    assert!(html.contains("<ul>"));
    assert!(html.contains("<li>"));
    assert!(html.contains("<pre><code>"));
    assert!(html.contains("foo"));
}

#[test]
fn test_list_marker_analysis() {
    // Verify list marker parsing for "-\t\tfoo".
    let input = "-\t\tfoo";
    let bytes = input.as_bytes();

    // Simulate parse_list_marker logic.
    let mut idx = 0;
    let indent_cols = 0;

    // No leading spaces.
    assert_eq!(bytes[idx], b'-');

    let marker_pos = idx;
    let marker_width = 1;
    idx += 1;

    // Now at position 1 (first tab).
    assert_eq!(bytes[idx], b'\t');

    let start_col = indent_cols + marker_width; // 0 + 1 = 1

    // Scan post-marker whitespace.
    let mut col = start_col;
    let mut bytes_consumed = 0;
    let mut content_bytes = 0;
    let mut content_done = false;

    while idx < bytes.len() {
        let b = bytes[idx];
        let next_col = match b {
            b' ' => Some(col + 1),
            b'\t' => Some(col + (4 - (col % 4))),
            _ => None,
        };

        let next_col = match next_col {
            Some(n) => n,
            None => break,
        };

        bytes_consumed += 1;

        if !content_done {
            // Check the columns consumed so far (before adding this byte).
            let cols_after = next_col - start_col;

            // Only include this byte if it doesn't push us over 4 columns.
            if cols_after <= 4 {
                content_bytes = bytes_consumed;
            }

            // Stop counting after we reach 4 columns.
            if cols_after >= 4 {
                content_done = true;
            }
        }

        col = next_col;
        idx += 1;
    }

    let post_cols = col - start_col;
    let has_nonspace = idx < bytes.len();

    // Tab 1: col 1 -> 4 (3 columns, reaches 3 total).
    // Tab 2: col 4 -> 8 (4 columns, reaches 7 total).
    assert_eq!(post_cols, 7);
    assert_eq!(bytes_consumed, 2);
    // Tab 1: col 1->4 (cols_after=3, <=4, included).
    // Tab 2: col 4->8 (cols_after=7, >4, NOT included).
    // So content_bytes should be 1.
    assert_eq!(
        content_bytes, 1,
        "content_bytes should only include first tab"
    );
    assert!(has_nonspace);

    let empty_item = !has_nonspace;
    let content_indent = indent_cols + marker_width + if empty_item { 1 } else { post_cols.min(4) };
    let marker_len = if empty_item {
        marker_pos + marker_width + bytes_consumed
    } else {
        marker_pos + marker_width + content_bytes
    };

    // content_indent should be 0 + 1 + 4 = 5.
    assert_eq!(content_indent, 5);
    // marker_len should be 0 + 1 + 1 = 2 (marker + first tab only).
    assert_eq!(marker_len, 2);
}

#[test]
fn test_commonmark_list_nest_tab_space() {
    // CommonMark example: list nesting with mixed spaces and tabs.
    let input = "- foo\n    - bar\n\t- baz\n";
    let parsed = parse(input);
    let resolved = resolve(
        parsed.document,
        input,
        &parsed.source_map,
        parsed.diagnostics,
        &parsed.link_defs,
    );
    let html = emit_html(&resolved.document.blocks);

    // Expected: nested UL/LI structure.
    assert!(html.contains("<ul>"));
    assert!(html.contains("<li>foo"));
    assert!(html.contains("<ul>"));
    assert!(html.contains("<li>bar"));
    assert!(html.contains("<li>baz"));
}

#[test]
fn test_commonmark_list_tab_indent_paragraph_continuation() {
    // CommonMark: tabs without a blank line continue the paragraph in a list item.
    let input = "- foo\n\t\tbar\n";
    let parsed = parse(input);
    let resolved = resolve(
        parsed.document,
        input,
        &parsed.source_map,
        parsed.diagnostics,
        &parsed.link_defs,
    );
    let html = emit_html(&resolved.document.blocks);

    // Expected: paragraph continuation in list item.
    assert!(html.contains("<ul>"));
    assert!(html.contains("<li>"));
    assert!(!html.contains("<pre><code>"));
    assert!(html.contains("bar"));
}

#[test]
fn test_remove_indent_columns_with_tabs() {
    // Simulate remove_indent_columns behavior.
    let text = "-\t\tfoo";

    // We want to remove content_indent (5) columns from the entire line
    let columns_to_remove = 5;

    let bytes = text.as_bytes();
    let mut col = 0;
    let mut byte_pos = 0;

    while byte_pos < bytes.len() && col < columns_to_remove {
        match bytes[byte_pos] {
            b' ' => col += 1,
            b'\t' => {
                let next_col = col + (4 - (col % 4));
                if next_col > columns_to_remove {
                    break;
                }
                col = next_col;
            }
            _ => break,
        }
        byte_pos += 1;
    }

    // '-' at col 0, non-whitespace, stop immediately.
    assert_eq!(col, 0);
    assert_eq!(byte_pos, 0);
}

#[test]
fn test_link_label_casefold_eszett() {
    let input = "[ẞ]\n\n[SS]: /url\n";
    let parsed = parse(input);
    let resolved = resolve(
        parsed.document,
        input,
        &parsed.source_map,
        parsed.diagnostics,
        &parsed.link_defs,
    );
    let html = emit_html(&resolved.document.blocks);
    assert!(html.contains("<a href=\"/url\">ẞ</a>"));
}

#[test]
fn test_remove_list_indent_function() {
    // Validate that list indentation with tabs produces an indented code block.
    let text = "-\t\tfoo";
    let marker_len: usize = 2;
    let content_indent: usize = 5;

    // Manually simulate remove_list_indent.
    let bytes = text.as_bytes();

    // Step 1: Calculate column after marker_len bytes.
    let mut col = 0;
    let mut byte_pos = 0;

    while byte_pos < bytes.len() && byte_pos < marker_len {
        match bytes[byte_pos] {
            b' ' => col += 1,
            b'\t' => col += 4 - (col % 4),
            _ => col += 1,
        }
        byte_pos += 1;
    }

    // After 2 bytes ('-' + tab), column should be 4.
    assert_eq!(byte_pos, 2);
    assert_eq!(col, 4);

    // Step 2: Remove additional indent.
    let columns_to_remove = content_indent.saturating_sub(col);

    // content_indent = 5, col = 4, so columns_to_remove = 1.
    assert_eq!(columns_to_remove, 1);

    // Now test the actual parser output.
    let input = "-\t\tfoo\n";
    let parsed = parse(input);
    let html = emit_html(&parsed.document.blocks);

    // The result should include an indented code block with "  foo".
    assert!(html.contains("<pre><code>  foo"));
}
