use crate::ast::{
    Block, BlockKind, BoxBlock, CodeBlock, CodeMeta, Inline, InlineKind, Label, LineRange, List,
    ResolvedRef,
};
use crate::math::render_math;
use ammonia::Builder;
use std::collections::{HashMap, HashSet};

/// Emits raw, un-sanitized HTML from a slice of blocks.
pub fn emit_html(blocks: &[Block]) -> String {
    // Deterministic formatting: 2-space indentation and LF newlines.
    let mut writer = HtmlWriter::new();
    for block in blocks {
        emit_block(&mut writer, block);
    }
    writer.finish()
}

/// Emits HTML from a slice of blocks and sanitizes it according to a safe allow-list.
pub fn emit_html_sanitized(blocks: &[Block]) -> String {
    let raw_html = emit_html(blocks);

    let tags: HashSet<&'static str> = [
        // Standard tags
        "a",
        "abbr",
        "b",
        "blockquote",
        "br",
        "code",
        "dd",
        "del",
        "details",
        "div",
        "dl",
        "dt",
        "em",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "hr",
        "i",
        "img",
        "kbd",
        "li",
        "ol",
        "p",
        "pre",
        "s",
        "strong",
        "sub",
        "summary",
        "sup",
        "u",
        "ul", // TypMark-specific tags
        "figure",
        "span", // SVG tags from core.md
        "svg",
        "g",
        "defs",
        "path",
        "clipPath",
        "use",
    ]
    .iter()
    .copied()
    .collect();

    let mut generic_attributes: HashSet<&'static str> = HashSet::new();
    generic_attributes.insert("class");
    generic_attributes.insert("id");

    let mut tag_attributes = HashMap::new();

    // Standard attributes
    tag_attributes.insert("a", ["href", "title"].iter().copied().collect());
    tag_attributes.insert("abbr", ["title"].iter().copied().collect());
    tag_attributes.insert("img", ["alt", "src", "title"].iter().copied().collect());
    tag_attributes.insert("ol", ["start"].iter().copied().collect());

    // TypMark code block attributes from core.md
    tag_attributes.insert(
        "span",
        [
            "class",
            "data-line",
            "data-highlighted-line",
            "data-diff",
            "data-line-label",
            "id",
        ]
        .iter()
        .copied()
        .collect(),
    );
    tag_attributes.insert(
        "figure",
        ["class", "data-typmark", "data-lang", "id"]
            .iter()
            .copied()
            .collect(),
    );

    // SVG attributes from core.md
    tag_attributes.insert(
        "svg",
        ["xmlns", "viewBox", "width", "height", "class"]
            .iter()
            .copied()
            .collect(),
    );
    tag_attributes.insert(
        "g",
        [
            "transform",
            "fill",
            "stroke",
            "stroke-width",
            "clip-path",
            "class",
        ]
        .iter()
        .copied()
        .collect(),
    );
    tag_attributes.insert(
        "path",
        ["d", "fill", "stroke", "stroke-width", "class"]
            .iter()
            .copied()
            .collect(),
    );
    tag_attributes.insert("clipPath", ["id"].iter().copied().collect());
    tag_attributes.insert("use", ["href", "xlink:href"].iter().copied().collect());

    // Box attributes (data-bg, data-border-style, etc.)
    tag_attributes.insert(
        "div",
        [
            "class",
            "data-typmark",
            "id",
            "data-bg",
            "data-title-bg",
            "data-border-color",
            "data-border-style",
            "data-border-width",
        ]
        .iter()
        .copied()
        .collect(),
    );

    let mut generic_attribute_prefixes = HashSet::new();
    generic_attribute_prefixes.insert("data-");

    Builder::new()
        .tags(tags)
        .generic_attributes(generic_attributes)
        .tag_attributes(tag_attributes)
        .generic_attribute_prefixes(generic_attribute_prefixes)
        .clean(&raw_html)
        .to_string()
}

struct HtmlWriter {
    out: String,
    indent: usize,
}

#[derive(Clone, Copy)]
enum RenderContext {
    Normal,
    Title,
    ReferenceText,
}

impl HtmlWriter {
    fn new() -> Self {
        Self {
            out: String::new(),
            indent: 0,
        }
    }

    fn line(&mut self, line: &str) {
        for _ in 0..self.indent {
            self.out.push_str("  ");
        }
        self.out.push_str(line);
        self.out.push('\n');
    }

    fn finish(mut self) -> String {
        if self.out.ends_with('\n') {
            self.out.pop();
        }
        self.out
    }
}

fn emit_block(writer: &mut HtmlWriter, block: &Block) {
    match &block.kind {
        BlockKind::Section {
            level,
            title,
            label,
            children,
        } => {
            let id = id_attr(label.as_ref());
            writer.line(&format!("<section{}>", id));
            writer.indent += 1;
            let heading = format!(
                "<h{}>{}</h{}>",
                level,
                render_inlines_with_context(title, RenderContext::Title),
                level
            );
            writer.line(&heading);
            for child in children {
                emit_block(writer, child);
            }
            writer.indent -= 1;
            writer.line("</section>");
        }
        BlockKind::Heading { level, title } => {
            let id = id_attr(block.attrs.label.as_ref());
            writer.line(&format!(
                "<h{}{}>{}</h{}>",
                level,
                id,
                render_inlines_with_context(title, RenderContext::Title),
                level
            ));
        }
        BlockKind::Paragraph { content } => {
            let id = id_attr(block.attrs.label.as_ref());
            writer.line(&format!(
                "<p{}>{}</p>",
                id,
                render_inlines_with_context(content, RenderContext::Normal)
            ));
        }
        BlockKind::BlockQuote { blocks } => {
            let id = id_attr(block.attrs.label.as_ref());
            writer.line(&format!("<blockquote{}>", id));
            writer.indent += 1;
            for child in blocks {
                emit_block(writer, child);
            }
            writer.indent -= 1;
            writer.line("</blockquote>");
        }
        BlockKind::List(List {
            ordered,
            start,
            items,
            tight,
            ..
        }) => {
            let tag = if *ordered { "ol" } else { "ul" };
            let id = id_attr(block.attrs.label.as_ref());
            let start_attr = if *ordered {
                start
                    .map(|value| format!(" start=\"{}\"", value))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            writer.line(&format!("<{}{}{}>", tag, id, start_attr));
            writer.indent += 1;
            for item in items {
                let mut handled = false;
                if *tight && item.blocks.len() == 1 {
                    if let BlockKind::Paragraph { content } = &item.blocks[0].kind {
                        // This is a single paragraph in a tight list item.
                        // Promote its ID to the `<li>` tag and render its inlines directly,
                        // bypassing the normal `emit_block` for the paragraph.
                        let li_attrs = id_attr(item.blocks[0].attrs.label.as_ref());
                        writer.line(&format!("<li{}>", li_attrs));
                        writer.indent += 1;
                        writer.line(&render_inlines_with_context(content, RenderContext::Normal));
                        handled = true;
                    }
                }

                if !handled {
                    // Default case for loose lists, or tight lists with multiple/non-paragraph blocks.
                    writer.line("<li>");
                    writer.indent += 1;
                    for child in &item.blocks {
                        emit_block(writer, child);
                    }
                }

                writer.indent -= 1;
                writer.line("</li>");
            }
            writer.indent -= 1;
            writer.line(&format!("</{}>", tag));
        }
        BlockKind::Box(BoxBlock { title, blocks }) => {
            let mut attrs = format!("class=\"TypMark-box\" data-typmark=\"box\"");
            if let Some(label) = block.attrs.label.as_ref() {
                attrs.push_str(&format!(" id=\"{}\"", escape_attr(&label.name)));
            }
            for item in &block.attrs.items {
                attrs.push_str(&format!(
                    " data-{}=\"{}\"",
                    escape_attr(&item.key),
                    escape_attr(&item.value.raw)
                ));
            }
            writer.line(&format!("<div {}>", attrs));
            writer.indent += 1;
            if let Some(title) = title {
                writer.line(&format!(
                    "<div class=\"TypMark-box-title\">{}</div>",
                    render_inlines_with_context(title, RenderContext::Title)
                ));
            }
            writer.line("<div class=\"TypMark-box-body\">");
            writer.indent += 1;
            for child in blocks {
                emit_block(writer, child);
            }
            writer.indent -= 1;
            writer.line("</div>");
            writer.indent -= 1;
            writer.line("</div>");
        }
        BlockKind::MathBlock { typst_src } => {
            let id = id_attr(block.attrs.label.as_ref());
            match render_math(typst_src, true) {
                Ok(svg) => writer.line(&format!(
                    "<div class=\"TypMark-math-block\"{}>{}</div>",
                    id, svg
                )),
                Err(source) => writer.line(&format!(
                    "<div class=\"TypMark-math-block--error\"{}>{}</div>",
                    id,
                    escape_html(&source)
                )),
            }
        }
        BlockKind::ThematicBreak => {
            let id = id_attr(block.attrs.label.as_ref());
            writer.line(&format!("<hr{} />", id));
        }
        BlockKind::CodeBlock(CodeBlock {
            lang, meta, text, ..
        }) => {
            emit_code_block(
                writer,
                lang.as_deref(),
                meta,
                text,
                block.attrs.label.as_ref(),
            );
        }
        BlockKind::HtmlBlock { raw } => {
            writer.line(raw);
        }
    }
}

fn emit_code_block(
    writer: &mut HtmlWriter,
    lang: Option<&str>,
    meta: &CodeMeta,
    text: &str,
    label: Option<&Label>,
) {
    // Check if this is a simple indented code block (no language, no metadata, no label)
    let is_simple = lang.is_none()
        && meta.hl.is_empty()
        && meta.diff_add.is_empty()
        && meta.diff_del.is_empty()
        && meta.line_labels.is_empty()
        && label.is_none();

    if is_simple {
        // Emit simple CommonMark-style pre/code for indented code blocks
        let escaped = escape_html(text);
        // Write as single line without indentation for CommonMark compatibility
        writer.out.push_str("<pre><code>");
        writer.out.push_str(&escaped);
        if !escaped.ends_with('\n') {
            writer.out.push('\n');
        }
        writer.out.push_str("</code></pre>\n");
    } else {
        // Emit full TypMark-style figure with line wrappers for fenced code blocks with metadata
        let lang_attr = lang
            .map(|value| format!(" data-lang=\"{}\"", escape_attr(value)))
            .unwrap_or_default();
        let id = id_attr(label);
        writer.line(&format!(
            "<figure class=\"TypMark-codeblock\" data-typmark=\"codeblock\"{}{}>",
            id, lang_attr
        ));
        writer.indent += 1;
        writer.line("<pre class=\"TypMark-pre\">");
        writer.indent += 1;
        let code_class = lang
            .map(|value| format!("language-{}", escape_attr(value)))
            .unwrap_or_else(|| "language-".to_string());
        writer.line(&format!("<code class=\"{}\">", code_class));
        writer.indent += 1;

        let lines = split_lines_preserve(text);
        for (idx, line) in lines.iter().enumerate() {
            let line_no = (idx + 1) as u32;
            let highlighted = line_in_ranges(line_no, &meta.hl);
            let diff = if line_in_ranges(line_no, &meta.diff_add) {
                Some("add")
            } else if line_in_ranges(line_no, &meta.diff_del) {
                Some("del")
            } else {
                None
            };
            let line_label = meta.line_labels.iter().find(|label| label.line == line_no);

            let mut class = String::from("line");
            if highlighted {
                class.push_str(" highlighted");
            }
            if let Some(diff_kind) = diff {
                class.push_str(" diff ");
                class.push_str(diff_kind);
            }
            let mut attrs = format!("class=\"{}\" data-line=\"{}\"", class, line_no);
            if highlighted {
                attrs.push_str(" data-highlighted-line");
            }
            if let Some(diff_kind) = diff {
                attrs.push_str(&format!(" data-diff=\"{}\"", diff_kind));
            }
            if let Some(label) = line_label {
                attrs.push_str(&format!(
                    " id=\"{}\" data-line-label=\"{}\"",
                    escape_attr(&label.label.name),
                    escape_attr(&label.label.name)
                ));
            }
            writer.line(&format!("<span {}>{}</span>", attrs, escape_html(line)));
        }

        writer.indent -= 1;
        writer.line("</code>");
        writer.indent -= 1;
        writer.line("</pre>");
        writer.indent -= 1;
        writer.line("</figure>");
    }
}

fn render_inlines_with_context(inlines: &[Inline], context: RenderContext) -> String {
    let mut out = String::new();
    for inline in inlines {
        match &inline.kind {
            InlineKind::Text(text) => out.push_str(&escape_html(text)),
            InlineKind::CodeSpan(text) => {
                out.push_str("<code>");
                out.push_str(&escape_html(text));
                out.push_str("</code>");
            }
            InlineKind::MathInline { typst_src } => match render_math(typst_src, false) {
                Ok(svg) => out.push_str(&svg),
                Err(source) => {
                    out.push_str("<span class=\"TypMark-math-inline--error\">");
                    out.push_str(&escape_html(&source));
                    out.push_str("</span>");
                }
            },
            InlineKind::SoftBreak => out.push(' '),
            InlineKind::HardBreak => out.push_str("<br />"),
            InlineKind::Ref {
                label,
                bracket,
                resolved,
            } => {
                out.push_str(&render_ref(
                    label,
                    bracket.as_deref(),
                    resolved.as_ref(),
                    context,
                ));
            }
            InlineKind::Emph(children) => {
                out.push_str("<em>");
                out.push_str(&render_inlines_with_context(children, context));
                out.push_str("</em>");
            }
            InlineKind::Strong(children) => {
                out.push_str("<strong>");
                out.push_str(&render_inlines_with_context(children, context));
                out.push_str("</strong>");
            }
            InlineKind::Link {
                url,
                title,
                children,
            } => match context {
                RenderContext::Normal => {
                    out.push_str("<a href=\"");
                    out.push_str(&escape_url_attr(url));
                    out.push('"');
                    if let Some(title) = title {
                        out.push_str(" title=\"");
                        out.push_str(&escape_attr(title));
                        out.push('"');
                    }
                    out.push('>');
                    out.push_str(&render_inlines_with_context(children, context));
                    out.push_str("</a>");
                }
                RenderContext::Title => {
                    out.push_str("<span class=\"TypMark-delink\">");
                    out.push_str(&render_inlines_with_context(children, RenderContext::Title));
                    out.push_str("</span>");
                }
                RenderContext::ReferenceText => {
                    out.push_str("<span class=\"TypMark-delink\">");
                    out.push_str(&render_inlines_with_context(
                        children,
                        RenderContext::ReferenceText,
                    ));
                    out.push_str("</span>");
                }
            },
            InlineKind::LinkRef {
                label,
                children,
                meta,
            } => {
                out.push('[');
                out.push_str(&render_inlines_with_context(children, context));
                out.push(']');
                if meta.label_open_span.is_some() {
                    out.push('[');
                    out.push_str(&escape_html(label));
                    out.push(']');
                }
            }
            InlineKind::Image { url, title, alt } => match context {
                RenderContext::ReferenceText => {
                    out.push_str(&render_inlines_with_context(
                        alt,
                        RenderContext::ReferenceText,
                    ));
                }
                _ => {
                    out.push_str("<img src=\"");
                    out.push_str(&escape_url_attr(url));
                    out.push_str("\" alt=\"");
                    out.push_str(&escape_attr(&render_inlines_with_context(
                        alt,
                        RenderContext::ReferenceText,
                    )));
                    out.push('"');
                    if let Some(title) = title {
                        out.push_str(" title=\"");
                        out.push_str(&escape_attr(title));
                        out.push('"');
                    }
                    out.push('>');
                }
            },
            InlineKind::ImageRef { label, alt, meta } => match context {
                RenderContext::ReferenceText => {
                    out.push_str(&render_inlines_with_context(
                        alt,
                        RenderContext::ReferenceText,
                    ));
                }
                _ => {
                    out.push_str("![");
                    out.push_str(&render_inlines_with_context(alt, context));
                    out.push(']');
                    if meta.label_open_span.is_some() {
                        out.push('[');
                        out.push_str(&escape_html(label));
                        out.push(']');
                    }
                }
            },
            InlineKind::HtmlSpan { raw } => out.push_str(raw),
        }
    }
    out
}

fn render_ref(
    label: &Label,
    bracket: Option<&[Inline]>,
    resolved: Option<&ResolvedRef>,
    context: RenderContext,
) -> String {
    let display = if let Some(bracket) = bracket {
        render_inlines_with_context(bracket, RenderContext::ReferenceText)
    } else if let Some(ResolvedRef::Block {
        display: Some(text),
        ..
    }) = resolved
    {
        render_inlines_with_context(text, RenderContext::ReferenceText)
    } else {
        escape_html(&label.name)
    };

    match context {
        RenderContext::Normal => {
            if resolved.is_some() {
                format!(
                    "<a class=\"TypMark-ref\" href=\"#{}\">{}</a>",
                    escape_attr(&label.name),
                    display
                )
            } else {
                format!(
                    "<span class=\"TypMark-ref ref-unresolved\" data-ref-label=\"{}\">{}</span>",
                    escape_attr(&label.name),
                    display
                )
            }
        }
        RenderContext::Title | RenderContext::ReferenceText => {
            if resolved.is_some() {
                format!("<span class=\"TypMark-delink\">{}</span>", display)
            } else {
                format!(
                    "<span class=\"TypMark-delink ref-unresolved\" data-ref-label=\"{}\">{}</span>",
                    escape_attr(&label.name),
                    display
                )
            }
        }
    }
}

fn line_in_ranges(line: u32, ranges: &[LineRange]) -> bool {
    ranges
        .iter()
        .any(|range| range.start <= line && line <= range.end)
}

fn split_lines_preserve(text: &str) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    let mut start = 0;
    let bytes = text.as_bytes();
    for (idx, byte) in bytes.iter().enumerate() {
        if *byte == b'\n' {
            let mut line = text[start..idx].to_string();
            if line.ends_with('\r') {
                line.pop();
            }
            lines.push(line);
            start = idx + 1;
        }
    }
    if start <= text.len() {
        let mut line = text[start..].to_string();
        if line.ends_with('\r') {
            line.pop();
        }
        lines.push(line);
    }
    lines
}

fn escape_html(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(ch),
        }
    }
    out
}

fn escape_attr(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            _ => out.push(ch),
        }
    }
    out
}

fn escape_url_attr(text: &str) -> String {
    let mut encoded = String::new();
    for &byte in text.as_bytes() {
        match byte {
            b' ' => encoded.push_str("%20"),
            b'\\' => encoded.push_str("%5C"),
            0x00..=0x1F | 0x7F..=0xFF => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
            _ => encoded.push(byte as char),
        }
    }
    escape_attr(&encoded)
}

fn id_attr(label: Option<&Label>) -> String {
    label
        .map(|label| format!(" id=\"{}\"", escape_attr(&label.name)))
        .unwrap_or_default()
}
