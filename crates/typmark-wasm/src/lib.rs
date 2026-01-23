use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RenderOptions {
    wrap_sections: Option<bool>,
    simple_code_blocks: Option<bool>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RenderResult {
    html: String,
    diagnostics: Vec<JsDiagnostic>,
    source_map: Vec<JsRange>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsDiagnostic {
    code: String,
    message: String,
    severity: String,
    range: JsRange,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct JsRange {
    start_line: usize,
    start_col: usize,
    end_line: usize,
    end_col: usize,
}

#[wasm_bindgen]
pub fn render_html(source: &str) -> Result<JsValue, JsValue> {
    render_html_with_options(source, JsValue::UNDEFINED)
}

#[wasm_bindgen]
pub fn render_html_with_options(source: &str, options: JsValue) -> Result<JsValue, JsValue> {
    let parsed = typmark_core::parse(source);
    let resolved = typmark_core::resolve(
        parsed.document,
        source,
        &parsed.source_map,
        parsed.diagnostics,
        &parsed.link_defs,
    );

    let emit_options = options_from_js(options)?;
    let html = typmark_core::emit_html_document_with_options_and_source_map(
        &resolved.document,
        &emit_options,
        &parsed.source_map,
    );

    let diagnostics = resolved
        .diagnostics
        .into_iter()
        .map(|diag| JsDiagnostic {
            code: diag.code.to_string(),
            message: diag.message,
            severity: match diag.severity {
                typmark_core::DiagnosticSeverity::Error => "error".to_string(),
                typmark_core::DiagnosticSeverity::Warning => "warning".to_string(),
            },
            range: JsRange {
                start_line: diag.range.start.line,
                start_col: diag.range.start.character,
                end_line: diag.range.end.line,
                end_col: diag.range.end.character,
            },
        })
        .collect();

    let mut source_map = Vec::new();
    collect_block_ranges(
        &resolved.document.blocks,
        &parsed.source_map,
        &mut source_map,
    );

    let result = RenderResult {
        html,
        diagnostics,
        source_map,
    };
    serde_wasm_bindgen::to_value(&result).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn add_font(bytes: Vec<u8>) {
    typmark_core::add_font_bytes(bytes);
}

fn options_from_js(value: JsValue) -> Result<typmark_core::HtmlEmitOptions, JsValue> {
    if value.is_null() || value.is_undefined() {
        return Ok(typmark_core::HtmlEmitOptions::default());
    }
    let parsed: RenderOptions =
        serde_wasm_bindgen::from_value(value).map_err(|err| JsValue::from_str(&err.to_string()))?;
    let mut out = typmark_core::HtmlEmitOptions::default();
    if let Some(wrap_sections) = parsed.wrap_sections {
        out.wrap_sections = wrap_sections;
    }
    if let Some(simple_code_blocks) = parsed.simple_code_blocks {
        out.simple_code_blocks = simple_code_blocks;
    }
    Ok(out)
}

fn collect_block_ranges(
    blocks: &[typmark_core::Block],
    source_map: &typmark_core::SourceMap,
    out: &mut Vec<JsRange>,
) {
    for block in blocks {
        let range = source_map.range(block.span);
        out.push(JsRange {
            start_line: range.start.line,
            start_col: range.start.character,
            end_line: range.end.line,
            end_col: range.end.character,
        });
        match &block.kind {
            typmark_core::BlockKind::Section { children, .. } => {
                collect_block_ranges(children, source_map, out);
            }
            typmark_core::BlockKind::BlockQuote { blocks } => {
                collect_block_ranges(blocks, source_map, out);
            }
            typmark_core::BlockKind::List(list) => {
                for item in &list.items {
                    collect_block_ranges(&item.blocks, source_map, out);
                }
            }
            typmark_core::BlockKind::Box(box_block) => {
                collect_block_ranges(&box_block.blocks, source_map, out);
            }
            _ => {}
        }
    }
}
