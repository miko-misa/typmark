use typmark_core::{HtmlEmitOptions, emit_html_with_options, parse, resolve};

#[test]
fn emit_simple_code_blocks_keep_attrs() {
    let source = "{#code foo=bar}\n```rs\nlet x = 1;\n```\n";
    let parsed = parse(source);
    let resolved = resolve(
        parsed.document,
        source,
        &parsed.source_map,
        parsed.diagnostics,
        &parsed.link_defs,
    );

    let options = HtmlEmitOptions {
        simple_code_blocks: true,
        ..Default::default()
    };

    let html = emit_html_with_options(&resolved.document.blocks, &options);
    let expected =
        "<pre id=\"code\" data-foo=\"bar\"><code class=\"language-rs\">let x = 1;\n</code></pre>";
    assert_eq!(html.trim_end(), expected);
}
