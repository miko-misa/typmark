use typmark_core::{emit_html, parse, resolve};

#[test]
fn test_list_item_with_two_tabs() {
    // Example 7 from CommonMark spec
    // "-\t\tfoo" should produce indented code block with "  foo"
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

    println!("Input: {:?}", input.replace('\t', "→"));
    println!("HTML output:\n{}", html);

    // Expected: list item containing indented code block
    assert!(html.contains("<ul>"));
    assert!(html.contains("<li>"));
    assert!(html.contains("<pre><code>"));
    assert!(html.contains("foo"));
}

#[test]
fn test_list_marker_analysis() {
    // Test the list marker parsing for "-\t\tfoo"
    let input = "-\t\tfoo";
    let bytes = input.as_bytes();

    // Simulate parse_list_marker logic
    let mut idx = 0;
    let mut indent_cols = 0;

    // No leading spaces
    assert_eq!(bytes[idx], b'-');

    let marker_pos = idx;
    let marker_width = 1;
    idx += 1;

    // Now at position 1 (first tab)
    assert_eq!(bytes[idx], b'\t');

    let start_col = indent_cols + marker_width; // 0 + 1 = 1

    // Scan post-marker whitespace
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
            // Check the columns consumed so far (before adding this byte)
            let cols_before = col - start_col;
            let cols_after = next_col - start_col;

            // Only include this byte if it doesn't push us over 4 columns
            if cols_after <= 4 {
                // This byte keeps us at or below 4 columns
                content_bytes = bytes_consumed;
            }

            // Stop counting after we reach 4 columns
            if cols_after >= 4 {
                content_done = true;
            }
        }

        col = next_col;
        idx += 1;
    }

    let post_cols = col - start_col;
    let has_nonspace = idx < bytes.len();

    println!("Analysis of '-\\t\\tfoo':");
    println!("  start_col: {}", start_col);
    println!("  post_cols: {}", post_cols);
    println!("  bytes_consumed: {}", bytes_consumed);
    println!("  content_bytes: {}", content_bytes);
    println!("  has_nonspace: {}", has_nonspace);
    println!("  final col: {}", col);

    // Tab 1: col 1 -> 4 (3 columns, reaches 3 total)
    // Tab 2: col 4 -> 8 (4 columns, reaches 7 total)
    assert_eq!(post_cols, 7);
    assert_eq!(bytes_consumed, 2);
    // Tab 1: col 1->4 (cols_after=3, <=4, included)
    // Tab 2: col 4->8 (cols_after=7, >4, NOT included)
    // So content_bytes should be 1
    assert_eq!(
        content_bytes, 1,
        "content_bytes should only include first tab"
    );
    assert_eq!(has_nonspace, true);

    let empty_item = !has_nonspace;
    let content_indent = indent_cols + marker_width + if empty_item { 1 } else { post_cols.min(4) };
    let marker_len = if empty_item {
        marker_pos + marker_width + bytes_consumed
    } else {
        marker_pos + marker_width + content_bytes
    };

    println!("  content_indent: {}", content_indent);
    println!("  marker_len: {}", marker_len);

    // content_indent should be 0 + 1 + 4 = 5
    assert_eq!(content_indent, 5);
    // marker_len should be 0 + 1 + 1 = 2 (marker + first tab only)
    assert_eq!(marker_len, 2);

    // After removing marker_len (2 bytes), we have "\tfoo"
    let remaining = &input[marker_len..];
    println!(
        "  remaining after marker: {:?}",
        remaining.replace('\t', "→")
    );

    // Now remove content_indent (5 columns) from "\tfoo"
    // Tab at col 0 -> col 4 (4 columns)
    // We need to remove 5 columns total, but tab only gives 4
    // So we remove the tab and need 1 more column from "foo"
    // But "foo" starts immediately, so we can't remove 5 columns
    // Actually, after removing marker (col 0-2), remaining starts at col 2
    // Tab at col 2 -> col 4 (2 columns)... wait, this is wrong

    // Let me recalculate:
    // Original: "-\t\tfoo"
    // Col:       0 1234 5678 ...
    // After removing 2 bytes (marker + tab1), we have: "\tfoo"
    // But this starts at BYTE position 2, not COLUMN position
    // The remaining content should be interpreted from column 2

    // Actually, the first line processing in parse_list should use
    // remove_indent_columns on the ENTIRE line, not the remaining part
}

#[test]
fn test_remove_indent_columns_with_tabs() {
    // Simulate remove_indent_columns behavior
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

    println!("remove_indent_columns('-\\t\\tfoo', 5):");
    println!("  col after scan: {}", col);
    println!("  byte_pos: {}", byte_pos);

    // '-' at col 0, non-whitespace, stop immediately
    assert_eq!(col, 0);
    assert_eq!(byte_pos, 0);

    // So the entire line remains unchanged!
    // This is the bug - we're not handling the marker correctly
}

#[test]
fn test_remove_list_indent_function() {
    use typmark_core::parse;

    // Test the actual remove_list_indent function by calling the parser
    // Input: "-\t\tfoo"
    // marker_len = 2 (marker + first tab)
    // content_indent = 5

    let text = "-\t\tfoo";
    let marker_len: usize = 2;
    let content_indent: usize = 5;

    // Manually simulate remove_list_indent
    let bytes = text.as_bytes();

    // Step 1: Calculate column after marker_len bytes
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

    println!("After consuming marker_len={} bytes:", marker_len);
    println!("  byte_pos: {}", byte_pos);
    println!("  col: {}", col);
    println!("  remaining: {:?}", &text[byte_pos..].replace('\t', "→"));

    // After 2 bytes ('-' + tab):
    // - '-' at col 0, advances to col 1
    // - tab at col 1, advances to col 4
    assert_eq!(byte_pos, 2);
    assert_eq!(col, 4);

    // Step 2: Remove additional indent
    let columns_to_remove = content_indent.saturating_sub(col);
    println!(
        "  columns_to_remove: {} (content_indent {} - col {})",
        columns_to_remove, content_indent, col
    );

    // content_indent = 5, col = 4, so columns_to_remove = 1
    assert_eq!(columns_to_remove, 1);

    // Remaining text is "\tfoo"
    // We need to remove 1 more column from this
    // Tab at col 4 -> col 8 (4 columns)
    // Removing 1 column from this tab leaves 3 columns of space

    println!("\nExpected result: 3 spaces + 'foo'");

    // Now test the actual parser output
    let input = "-\t\tfoo\n";
    let parsed = parse(input);

    // Check what the parser actually produced
    println!("\nActual parser AST:");
    println!("{:#?}", parsed.document);

    // The result should have 3 spaces before 'foo' to make it an indented code block
    // (4 spaces required for indented code, but we're at list indent level already)
}
