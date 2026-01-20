use typmark_core::{
    BlockKind, HtmlEmitOptions, ParseResult,
    emit_html_document_sanitized_with_options_and_source_map,
    emit_html_document_with_options_and_source_map, parse, resolve,
};

#[test]
fn emit_source_map_attributes() {
    let source = "Alpha\n";
    let ParseResult {
        document,
        diagnostics,
        source_map,
        link_defs,
    } = parse(source);
    let resolved = resolve(document, source, &source_map, diagnostics, &link_defs);

    let html = emit_html_document_with_options_and_source_map(
        &resolved.document,
        &HtmlEmitOptions::default(),
        &source_map,
    );

    let block = &resolved.document.blocks[0];
    let BlockKind::Paragraph { content } = &block.kind else {
        panic!("expected paragraph block");
    };
    let inline = &content[0];

    let block_range = source_map.range(block.span);
    let inline_range = source_map.range(inline.span);
    let block_attr = format!(
        " data-tm-range=\"{}:{}-{}:{}\"",
        block_range.start.line,
        block_range.start.character,
        block_range.end.line,
        block_range.end.character
    );
    let inline_attr = format!(
        " data-tm-range=\"{}:{}-{}:{}\"",
        inline_range.start.line,
        inline_range.start.character,
        inline_range.end.line,
        inline_range.end.character
    );
    let expected = format!("<p{}><span{}>Alpha</span></p>", block_attr, inline_attr);
    assert!(
        html.contains(&expected),
        "expected source map markup, got: {}",
        html
    );
}

#[test]
fn emit_source_map_attributes_in_sanitized_html() {
    let source = "Alpha\n";
    let ParseResult {
        document,
        diagnostics,
        source_map,
        link_defs,
    } = parse(source);
    let resolved = resolve(document, source, &source_map, diagnostics, &link_defs);

    let html = emit_html_document_sanitized_with_options_and_source_map(
        &resolved.document,
        &HtmlEmitOptions::default(),
        &source_map,
    );

    assert!(
        html.contains("data-tm-range=\""),
        "expected data-tm-range in sanitized HTML"
    );
}
