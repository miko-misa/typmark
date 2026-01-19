use typmark_core::{parse, resolve};

#[test]
fn document_settings_are_parsed_once() {
    let source = "{ font-size=16px line-height=1.7 }\n\nParagraph.";
    let parsed = parse(source);
    let resolved = resolve(
        parsed.document,
        source,
        &parsed.source_map,
        parsed.diagnostics,
        &parsed.link_defs,
    );

    let settings = resolved.document.settings.expect("settings");
    assert!(settings.label.is_none());
    assert_eq!(settings.items.len(), 2);
    assert_eq!(settings.items[0].key, "font-size");
    assert_eq!(settings.items[0].value.raw, "16px");
    assert_eq!(settings.items[1].key, "line-height");
    assert_eq!(settings.items[1].value.raw, "1.7");

    assert_eq!(resolved.document.blocks.len(), 1);
    assert!(resolved.document.blocks[0].attrs.items.is_empty());
}

#[test]
fn labeled_target_line_is_not_document_settings() {
    let source = "{#intro}\n# Title";
    let parsed = parse(source);
    let resolved = resolve(
        parsed.document,
        source,
        &parsed.source_map,
        parsed.diagnostics,
        &parsed.link_defs,
    );

    assert!(resolved.document.settings.is_none());
    assert_eq!(resolved.document.blocks.len(), 1);
    assert_eq!(
        resolved.document.blocks[0]
            .attrs
            .label
            .as_ref()
            .unwrap()
            .name,
        "intro"
    );
}
