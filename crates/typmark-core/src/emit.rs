use crate::ast::{
    AttrItem, AttrList, Block, BlockKind, BoxBlock, CodeBlock, CodeBlockKind, CodeMeta, Inline,
    InlineKind, Label, LineRange, List, ResolvedRef, Table, TableAlign,
};
use crate::math::{MathSettings, prefix_svg_ids, render_math};
use crate::source_map::SourceMap;
use crate::span::Span;
use ammonia::Builder;
use std::collections::{HashMap, HashSet};

const SVG_ALLOWED_TAGS: &[&str] = &["svg", "g", "defs", "path", "symbol", "use"];

const SVG_ALLOWED_ATTRS: &[(&str, &[&str])] = &[
    ("svg", &["viewBox", "width", "height", "class"]),
    ("g", &["transform", "class"]),
    (
        "path",
        &[
            "d",
            "fill",
            "fill-rule",
            "stroke",
            "stroke-linecap",
            "stroke-linejoin",
            "stroke-miterlimit",
            "stroke-width",
            "transform",
            "class",
        ],
    ),
    ("defs", &["id"]),
    ("symbol", &["id", "overflow"]),
    ("use", &["href", "x", "y", "fill", "fill-rule"]),
];

/// Options for HTML emission.
#[derive(Debug, Clone)]
pub struct HtmlEmitOptions {
    /// Whether to wrap sections in `<section>` tags.
    /// If false, only emits the heading tag (CommonMark-compatible).
    pub wrap_sections: bool,
    /// Whether to use simple code block output (just `<pre><code>`).
    /// If false, uses TypMark's enhanced structure with line spans and figure wrapper.
    pub simple_code_blocks: bool,
}

impl Default for HtmlEmitOptions {
    fn default() -> Self {
        Self {
            wrap_sections: true,
            simple_code_blocks: false,
        }
    }
}

/// Emits raw, un-sanitized HTML from a slice of blocks with default options.
pub fn emit_html(blocks: &[Block]) -> String {
    emit_html_with_options(blocks, &HtmlEmitOptions::default())
}

/// Emits raw, un-sanitized HTML from a slice of blocks with custom options.
pub fn emit_html_with_options(blocks: &[Block], options: &HtmlEmitOptions) -> String {
    // Deterministic formatting: 2-space indentation and LF newlines.
    let mut writer = HtmlWriter::new(options.clone(), MathSettings::default());
    for block in blocks {
        emit_block(&mut writer, block);
    }
    writer.finish()
}

/// Emits raw, un-sanitized HTML from a document with custom options.
pub fn emit_html_document_with_options(
    document: &crate::ast::Document,
    options: &HtmlEmitOptions,
) -> String {
    let math_settings = math_settings_from_attrs(document.settings.as_ref());
    let mut writer = HtmlWriter::new(options.clone(), math_settings);
    for block in &document.blocks {
        emit_block(&mut writer, block);
    }
    writer.finish()
}

/// Emits raw, un-sanitized HTML from a document with source map attributes.
pub fn emit_html_document_with_options_and_source_map(
    document: &crate::ast::Document,
    options: &HtmlEmitOptions,
    source_map: &SourceMap,
) -> String {
    let math_settings = math_settings_from_attrs(document.settings.as_ref());
    let mut writer = HtmlWriter::new_with_source_map(options.clone(), math_settings, source_map);
    for block in &document.blocks {
        emit_block(&mut writer, block);
    }
    writer.finish()
}

/// Emits HTML from a slice of blocks and sanitizes it according to a safe allow-list.
pub fn emit_html_sanitized(blocks: &[Block]) -> String {
    let raw_html = emit_html(blocks);
    sanitize_html(&raw_html)
}

/// Emits HTML from a slice of blocks with custom options and sanitizes it.
pub fn emit_html_sanitized_with_options(blocks: &[Block], options: &HtmlEmitOptions) -> String {
    let raw_html = emit_html_with_options(blocks, options);
    sanitize_html(&raw_html)
}

/// Emits HTML from a document with custom options and sanitizes it.
pub fn emit_html_document_sanitized_with_options(
    document: &crate::ast::Document,
    options: &HtmlEmitOptions,
) -> String {
    let raw_html = emit_html_document_with_options(document, options);
    sanitize_html(&raw_html)
}

/// Emits HTML from a document with source map attributes and sanitizes it.
pub fn emit_html_document_sanitized_with_options_and_source_map(
    document: &crate::ast::Document,
    options: &HtmlEmitOptions,
    source_map: &SourceMap,
) -> String {
    let raw_html = emit_html_document_with_options_and_source_map(document, options, source_map);
    sanitize_html(&raw_html)
}

fn sanitize_html(raw_html: &str) -> String {
    let mut tags: HashSet<&'static str> = [
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
        "table",
        "thead",
        "tbody",
        "tr",
        "th",
        "td",
        "input",
        "figure",
        "span",
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
    tag_attributes.insert("th", ["align"].iter().copied().collect());
    tag_attributes.insert("td", ["align"].iter().copied().collect());
    tag_attributes.insert(
        "input",
        ["type", "checked", "disabled"].iter().copied().collect(),
    );

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
    for tag in SVG_ALLOWED_TAGS {
        tags.insert(*tag);
    }
    for (tag, attrs) in SVG_ALLOWED_ATTRS {
        tag_attributes.insert(*tag, attrs.iter().copied().collect());
    }

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
        .clean(raw_html)
        .to_string()
}

struct HtmlWriter {
    out: String,
    indent: usize,
    options: HtmlEmitOptions,
    math_counter: usize,
    math_settings: MathSettings,
    source_map: Option<SourceMap>,
}

#[derive(Clone, Copy)]
enum RenderContext {
    Normal,
    Title,
    ReferenceText,
}

impl HtmlWriter {
    fn new(options: HtmlEmitOptions, math_settings: MathSettings) -> Self {
        Self {
            out: String::new(),
            indent: 0,
            options,
            math_counter: 0,
            math_settings,
            source_map: None,
        }
    }

    fn new_with_source_map(
        options: HtmlEmitOptions,
        math_settings: MathSettings,
        source_map: &SourceMap,
    ) -> Self {
        let mut writer = Self::new(options, math_settings);
        writer.source_map = Some(source_map.clone());
        writer
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

fn math_settings_from_attrs(settings: Option<&AttrList>) -> MathSettings {
    let mut out = MathSettings::default();
    let Some(settings) = settings else {
        return out;
    };
    for item in &settings.items {
        match item.key.as_str() {
            "math-inline-size" => out.inline_size = Some(item.value.raw.clone()),
            "math-block-size" => out.block_size = Some(item.value.raw.clone()),
            "math-font" => out.font = Some(item.value.raw.clone()),
            _ => {}
        }
    }
    out
}

fn emit_block(writer: &mut HtmlWriter, block: &Block) {
    match &block.kind {
        BlockKind::Section {
            level,
            title,
            label,
            children,
        } => {
            if writer.options.wrap_sections {
                let attrs = compose_block_attrs_with_span(
                    label.as_ref(),
                    &block.attrs.items,
                    block.span,
                    writer.source_map.as_ref(),
                );
                writer.line(&format!("<section{}>", attrs));
                writer.indent += 1;
                let title_html = render_inlines_with_context(
                    title,
                    RenderContext::Title,
                    &mut writer.math_counter,
                    &writer.math_settings,
                    writer.source_map.as_ref(),
                );
                let heading = format!("<h{}>{}</h{}>", level, title_html, level);
                writer.line(&heading);
                for child in children {
                    emit_block(writer, child);
                }
                writer.indent -= 1;
                writer.line("</section>");
            } else {
                // CommonMark-compatible: just emit heading without wrapper
                let attrs = compose_block_attrs_with_span(
                    label.as_ref(),
                    &block.attrs.items,
                    block.span,
                    writer.source_map.as_ref(),
                );
                let title_html = render_inlines_with_context(
                    title,
                    RenderContext::Title,
                    &mut writer.math_counter,
                    &writer.math_settings,
                    writer.source_map.as_ref(),
                );
                writer.line(&format!("<h{}{}>{}</h{}>", level, attrs, title_html, level));
                for child in children {
                    emit_block(writer, child);
                }
            }
        }
        BlockKind::Heading { level, title } => {
            let attrs = compose_block_attrs_with_span(
                block.attrs.label.as_ref(),
                &block.attrs.items,
                block.span,
                writer.source_map.as_ref(),
            );
            let title_html = render_inlines_with_context(
                title,
                RenderContext::Title,
                &mut writer.math_counter,
                &writer.math_settings,
                writer.source_map.as_ref(),
            );
            writer.line(&format!("<h{}{}>{}</h{}>", level, attrs, title_html, level));
        }
        BlockKind::Paragraph { content } => {
            let attrs = compose_block_attrs_with_span(
                block.attrs.label.as_ref(),
                &block.attrs.items,
                block.span,
                writer.source_map.as_ref(),
            );
            let inline_html = render_inlines_with_context(
                content,
                RenderContext::Normal,
                &mut writer.math_counter,
                &writer.math_settings,
                writer.source_map.as_ref(),
            );
            writer.line(&format!("<p{}>{}</p>", attrs, inline_html));
        }
        BlockKind::BlockQuote { blocks } => {
            let attrs = compose_block_attrs_with_span(
                block.attrs.label.as_ref(),
                &block.attrs.items,
                block.span,
                writer.source_map.as_ref(),
            );
            writer.line(&format!("<blockquote{}>", attrs));
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
            let attrs = compose_block_attrs_with_span(
                block.attrs.label.as_ref(),
                &block.attrs.items,
                block.span,
                writer.source_map.as_ref(),
            );
            let start_attr = if *ordered {
                start
                    .filter(|&value| value != 1) // Omit start="1" (default value)
                    .map(|value| format!(" start=\"{}\"", value))
                    .unwrap_or_default()
            } else {
                String::new()
            };
            let has_tasks = items.iter().any(|item| item.task.is_some());
            let list_class = if has_tasks {
                " class=\"task-list\""
            } else {
                ""
            };
            writer.line(&format!("<{}{}{}{}>", tag, attrs, start_attr, list_class));
            writer.indent += 1;
            for item in items {
                let task_prefix = item.task.map(task_input_html);
                let task_class = if item.task.is_some() {
                    " class=\"task-list-item\""
                } else {
                    ""
                };
                let item_span = span_attr(item.span, writer.source_map.as_ref());
                if item.blocks.is_empty() {
                    writer.line(&format!("<li{}{}></li>", task_class, item_span));
                    continue;
                }
                if *tight && !item.blocks.is_empty() {
                    // In tight lists, unwrap the first paragraph (if any)
                    if let BlockKind::Paragraph { content } = &item.blocks[0].kind {
                        // Render <li> and first paragraph inline without newline
                        let inline_content = render_inlines_with_context(
                            content,
                            RenderContext::Normal,
                            &mut writer.math_counter,
                            &writer.math_settings,
                            writer.source_map.as_ref(),
                        );
                        writer.out.push_str(&"  ".repeat(writer.indent));
                        writer.out.push_str("<li");
                        writer.out.push_str(task_class);
                        writer.out.push_str(&item_span);
                        writer.out.push('>');
                        if let Some(prefix) = &task_prefix {
                            writer.out.push_str(prefix);
                        }
                        writer.out.push_str(&inline_content);

                        // Render remaining blocks normally
                        if item.blocks.len() > 1 {
                            writer.out.push('\n');
                            writer.indent += 1;
                            let mut last_ended = true;
                            for (idx, child) in item.blocks[1..].iter().enumerate() {
                                let ended = emit_block_tight(writer, child);
                                if !ended && idx + 1 < item.blocks.len() - 1 {
                                    writer.out.push('\n');
                                }
                                last_ended = ended;
                            }
                            writer.indent -= 1;
                            if last_ended {
                                writer.line("</li>");
                            } else {
                                writer.out.push_str("</li>\n");
                            }
                        } else {
                            // Only one paragraph, close on same line
                            writer.out.push_str("</li>\n");
                        }
                    } else {
                        // First block is not a paragraph, render all blocks with tight semantics
                        writer.line(&format!("<li{}>", task_class));
                        writer.indent += 1;
                        if let Some(prefix) = &task_prefix {
                            writer.line(prefix);
                        }
                        let mut last_ended = true;
                        for (idx, child) in item.blocks.iter().enumerate() {
                            let ended = emit_block_tight(writer, child);
                            if !ended && idx + 1 < item.blocks.len() {
                                writer.out.push('\n');
                            }
                            last_ended = ended;
                        }
                        writer.indent -= 1;
                        if last_ended {
                            writer.line("</li>");
                        } else {
                            writer.out.push_str("</li>\n");
                        }
                    }
                } else {
                    // Loose list: render all blocks normally (paragraphs get <p> tags)
                    writer.line(&format!("<li{}{}>", task_class, item_span));
                    writer.indent += 1;
                    for (idx, child) in item.blocks.iter().enumerate() {
                        if idx == 0
                            && let BlockKind::Paragraph { content } = &child.kind
                            && let Some(prefix) = &task_prefix
                        {
                            emit_paragraph_with_prefix(writer, content, prefix);
                            continue;
                        }
                        emit_block(writer, child);
                    }
                    writer.indent -= 1;
                    writer.line("</li>");
                }
            }
            writer.indent -= 1;
            writer.line(&format!("</{}>", tag));
        }
        BlockKind::Table(table) => {
            let attrs = compose_block_attrs_with_span(
                block.attrs.label.as_ref(),
                &block.attrs.items,
                block.span,
                writer.source_map.as_ref(),
            );
            emit_table(writer, table, &attrs);
        }
        BlockKind::Box(BoxBlock { title, blocks }) => {
            let mut attrs = "class=\"TypMark-box\" data-typmark=\"box\"".to_string();
            attrs.push_str(&span_attr(block.span, writer.source_map.as_ref()));
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
                let title_html = render_inlines_with_context(
                    title,
                    RenderContext::Title,
                    &mut writer.math_counter,
                    &writer.math_settings,
                    writer.source_map.as_ref(),
                );
                writer.line(&format!(
                    "<div class=\"TypMark-box-title\">{}</div>",
                    title_html
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
            let attrs = compose_block_attrs_with_span(
                block.attrs.label.as_ref(),
                &block.attrs.items,
                block.span,
                writer.source_map.as_ref(),
            );
            match render_math_with_prefix(
                typst_src,
                true,
                &mut writer.math_counter,
                &writer.math_settings,
            ) {
                Ok(svg) => writer.line(&format!(
                    "<div class=\"TypMark-math-block\"{}>{}</div>",
                    attrs, svg
                )),
                Err(source) => writer.line(&format!(
                    "<div class=\"TypMark-math-block--error\"{}>{}</div>",
                    attrs,
                    escape_text(&source)
                )),
            }
        }
        BlockKind::ThematicBreak => {
            let attrs = compose_block_attrs_with_span(
                block.attrs.label.as_ref(),
                &block.attrs.items,
                block.span,
                writer.source_map.as_ref(),
            );
            writer.line(&format!("<hr{} />", attrs));
        }
        BlockKind::CodeBlock(CodeBlock {
            kind,
            lang,
            info_attrs,
            meta,
            text,
        }) => {
            let attrs = compose_block_attrs_with_span(
                block.attrs.label.as_ref(),
                &block.attrs.items,
                block.span,
                writer.source_map.as_ref(),
            );
            let data = CodeBlockRender {
                attrs,
                kind: *kind,
                lang: lang.as_deref(),
                info_items: &info_attrs.items,
                meta,
                text,
            };
            emit_code_block(writer, data);
        }
        BlockKind::HtmlBlock { raw } => {
            let attrs = compose_block_attrs_with_span(
                block.attrs.label.as_ref(),
                &block.attrs.items,
                block.span,
                writer.source_map.as_ref(),
            );
            if attrs.is_empty() {
                writer.line(raw);
            } else {
                writer.line(&format!(
                    "<div class=\"TypMark-html\" data-typmark=\"html\"{}>",
                    attrs
                ));
                writer.indent += 1;
                writer.line(raw);
                writer.indent -= 1;
                writer.line("</div>");
            }
        }
    }
}

fn emit_block_tight(writer: &mut HtmlWriter, block: &Block) -> bool {
    match &block.kind {
        BlockKind::Paragraph { content } => {
            let inline = render_inlines_with_context(
                content,
                RenderContext::Normal,
                &mut writer.math_counter,
                &writer.math_settings,
                writer.source_map.as_ref(),
            );
            writer.out.push_str(&"  ".repeat(writer.indent));
            writer.out.push_str(&inline);
            false
        }
        BlockKind::Section {
            level,
            title,
            label,
            children,
        } => {
            if writer.options.wrap_sections {
                let attrs = compose_block_attrs_with_span(
                    label.as_ref(),
                    &block.attrs.items,
                    block.span,
                    writer.source_map.as_ref(),
                );
                writer.line(&format!("<section{}>", attrs));
                writer.indent += 1;
                let title_html = render_inlines_with_context(
                    title,
                    RenderContext::Title,
                    &mut writer.math_counter,
                    &writer.math_settings,
                    writer.source_map.as_ref(),
                );
                let heading = format!("<h{}>{}</h{}>", level, title_html, level);
                writer.line(&heading);
                for (idx, child) in children.iter().enumerate() {
                    let ended = emit_block_tight(writer, child);
                    if !ended && idx + 1 < children.len() {
                        writer.out.push('\n');
                    }
                }
                writer.indent -= 1;
                writer.line("</section>");
                true
            } else {
                let attrs = compose_block_attrs_with_span(
                    label.as_ref(),
                    &block.attrs.items,
                    block.span,
                    writer.source_map.as_ref(),
                );
                let title_html = render_inlines_with_context(
                    title,
                    RenderContext::Title,
                    &mut writer.math_counter,
                    &writer.math_settings,
                    writer.source_map.as_ref(),
                );
                writer.line(&format!("<h{}{}>{}</h{}>", level, attrs, title_html, level));
                let mut last_ended = true;
                for (idx, child) in children.iter().enumerate() {
                    let ended = emit_block_tight(writer, child);
                    if !ended && idx + 1 < children.len() {
                        writer.out.push('\n');
                    }
                    last_ended = ended;
                }
                last_ended
            }
        }
        _ => {
            emit_block(writer, block);
            true
        }
    }
}

struct CodeBlockRender<'a> {
    attrs: String,
    kind: CodeBlockKind,
    lang: Option<&'a str>,
    info_items: &'a [AttrItem],
    meta: &'a CodeMeta,
    text: &'a str,
}

fn emit_code_block(writer: &mut HtmlWriter, data: CodeBlockRender<'_>) {
    let mut attrs = data.attrs;
    attrs.push_str(&data_attrs(data.info_items));
    if writer.options.simple_code_blocks {
        // CommonMark-compatible simple output
        // Use code-specific escaping for code contents.
        let escaped = escape_html_code(data.text);
        let lang_class = data
            .lang
            .map(|value| format!(" class=\"language-{}\"", escape_attr(value)))
            .unwrap_or_default();
        writer
            .out
            .push_str(&format!("<pre{}><code{}>", attrs, lang_class));
        writer.out.push_str(&escaped);
        // Only add newline if text is non-empty and doesn't already end with one
        if !escaped.is_empty() && !escaped.ends_with('\n') {
            writer.out.push('\n');
        }
        writer.out.push_str("</code></pre>\n");
    } else if data.kind == CodeBlockKind::Indented {
        // Emit simple CommonMark-style pre/code for indented code blocks
        // Use code-specific escaping for code contents.
        let escaped = escape_html_code(data.text);
        // Write as single line without indentation for CommonMark compatibility
        writer.out.push_str(&format!("<pre{}><code>", attrs));
        writer.out.push_str(&escaped);
        // Only add newline if text is non-empty and doesn't already end with one
        if !escaped.is_empty() && !escaped.ends_with('\n') {
            writer.out.push('\n');
        }
        writer.out.push_str("</code></pre>\n");
    } else {
        // Emit full TypMark-style figure with line wrappers for fenced code blocks with metadata
        let lang_attr = data
            .lang
            .map(|value| format!(" data-lang=\"{}\"", escape_attr(value)))
            .unwrap_or_default();
        writer.line(&format!(
            "<figure class=\"TypMark-codeblock\" data-typmark=\"codeblock\"{}{}>",
            attrs, lang_attr
        ));
        writer.indent += 1;
        let code_class = data
            .lang
            .map(|value| format!("language-{}", escape_attr(value)))
            .unwrap_or_else(|| "language-".to_string());
        writer.out.push_str(&"  ".repeat(writer.indent));
        writer.out.push_str(&format!(
            "<pre class=\"TypMark-pre\"><code class=\"{}\">",
            code_class
        ));

        let lines = split_lines_preserve(data.text);
        let mut display_line_no = 1u32;
        for (idx, line) in lines.iter().enumerate() {
            let line_no = (idx + 1) as u32;
            let highlighted = line_in_ranges(line_no, &data.meta.hl);
            let diff = if line_in_ranges(line_no, &data.meta.diff_add) {
                Some("add")
            } else if line_in_ranges(line_no, &data.meta.diff_del) {
                Some("del")
            } else {
                None
            };
            let line_label = data
                .meta
                .line_labels
                .iter()
                .find(|label| label.line == line_no);

            let mut class = String::from("line");
            if highlighted {
                class.push_str(" highlighted");
            }
            if let Some(diff_kind) = diff {
                class.push_str(" diff ");
                class.push_str(diff_kind);
            }
            let mut attrs = format!("class=\"{}\"", class);
            if diff != Some("del") {
                attrs.push_str(&format!(" data-line=\"{}\"", display_line_no));
                display_line_no += 1;
            }
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
            writer.out.push_str(&format!(
                "<span {}>{}</span>",
                attrs,
                escape_html_code(line)
            ));
        }

        writer.out.push_str("</code></pre>\n");
        writer.indent -= 1;
        writer.line("</figure>");
    }
}

fn render_math_with_prefix(
    typst_src: &str,
    display: bool,
    math_counter: &mut usize,
    math_settings: &MathSettings,
) -> Result<String, String> {
    *math_counter += 1;
    let prefix = format!("tm-m{}", *math_counter);
    render_math(typst_src, display, math_settings).map(|svg| prefix_svg_ids(&svg, &prefix))
}

fn render_inlines_with_context(
    inlines: &[Inline],
    context: RenderContext,
    math_counter: &mut usize,
    math_settings: &MathSettings,
    source_map: Option<&SourceMap>,
) -> String {
    let mut out = String::new();
    for inline in inlines {
        let span_attr = span_attr(inline.span, source_map);
        match &inline.kind {
            InlineKind::Text(text) => {
                if span_attr.is_empty() {
                    out.push_str(&escape_text(text));
                } else {
                    out.push_str("<span");
                    out.push_str(&span_attr);
                    out.push('>');
                    out.push_str(&escape_text(text));
                    out.push_str("</span>");
                }
            }
            InlineKind::CodeSpan(text) => {
                if span_attr.is_empty() {
                    out.push_str("<code>");
                } else {
                    out.push_str("<code");
                    out.push_str(&span_attr);
                    out.push('>');
                }
                out.push_str(&escape_html_code(text));
                out.push_str("</code>");
            }
            InlineKind::MathInline { typst_src } => {
                match render_math_with_prefix(typst_src, false, math_counter, math_settings) {
                    Ok(svg) => {
                        if span_attr.is_empty() {
                            out.push_str("<span class=\"TypMark-math-inline\">");
                        } else {
                            out.push_str("<span class=\"TypMark-math-inline\"");
                            out.push_str(&span_attr);
                            out.push('>');
                        }
                        out.push_str(
                            "<span class=\"TypMark-math-inline-strut\" aria-hidden=\"true\"></span>",
                        );
                        out.push_str(&svg);
                        out.push_str("</span>");
                    }
                    Err(source) => {
                        if span_attr.is_empty() {
                            out.push_str("<span class=\"TypMark-math-inline--error\">");
                        } else {
                            out.push_str("<span class=\"TypMark-math-inline--error\"");
                            out.push_str(&span_attr);
                            out.push('>');
                        }
                        out.push_str(&escape_text(&source));
                        out.push_str("</span>");
                    }
                }
            }
            InlineKind::SoftBreak => out.push('\n'),
            InlineKind::HardBreak => {
                if span_attr.is_empty() {
                    out.push_str("<br />\n");
                } else {
                    out.push_str("<br");
                    out.push_str(&span_attr);
                    out.push_str(" />\n");
                }
            }
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
                    math_counter,
                    math_settings,
                    source_map,
                    inline.span,
                ));
            }
            InlineKind::Emph(children) => {
                if span_attr.is_empty() {
                    out.push_str("<em>");
                } else {
                    out.push_str("<em");
                    out.push_str(&span_attr);
                    out.push('>');
                }
                out.push_str(&render_inlines_with_context(
                    children,
                    context,
                    math_counter,
                    math_settings,
                    source_map,
                ));
                out.push_str("</em>");
            }
            InlineKind::Strong(children) => {
                if span_attr.is_empty() {
                    out.push_str("<strong>");
                } else {
                    out.push_str("<strong");
                    out.push_str(&span_attr);
                    out.push('>');
                }
                out.push_str(&render_inlines_with_context(
                    children,
                    context,
                    math_counter,
                    math_settings,
                    source_map,
                ));
                out.push_str("</strong>");
            }
            InlineKind::Strikethrough(children) => {
                if span_attr.is_empty() {
                    out.push_str("<del>");
                } else {
                    out.push_str("<del");
                    out.push_str(&span_attr);
                    out.push('>');
                }
                out.push_str(&render_inlines_with_context(
                    children,
                    context,
                    math_counter,
                    math_settings,
                    source_map,
                ));
                out.push_str("</del>");
            }
            InlineKind::Link {
                url,
                title,
                children,
            } => match context {
                RenderContext::Normal | RenderContext::Title => {
                    out.push_str("<a href=\"");
                    out.push_str(&escape_url_attr(url));
                    out.push('"');
                    if let Some(title) = title {
                        out.push_str(" title=\"");
                        out.push_str(&escape_attr(title));
                        out.push('"');
                    }
                    out.push_str(&span_attr);
                    out.push('>');
                    out.push_str(&render_inlines_with_context(
                        children,
                        context,
                        math_counter,
                        math_settings,
                        source_map,
                    ));
                    out.push_str("</a>");
                }
                RenderContext::ReferenceText => {
                    if span_attr.is_empty() {
                        out.push_str("<span class=\"TypMark-delink\">");
                    } else {
                        out.push_str("<span class=\"TypMark-delink\"");
                        out.push_str(&span_attr);
                        out.push('>');
                    }
                    out.push_str(&render_inlines_with_context(
                        children,
                        RenderContext::ReferenceText,
                        math_counter,
                        math_settings,
                        source_map,
                    ));
                    out.push_str("</span>");
                }
            },
            InlineKind::LinkRef {
                label,
                children,
                meta,
            } => {
                if span_attr.is_empty() {
                    out.push('[');
                    out.push_str(&render_inlines_with_context(
                        children,
                        context,
                        math_counter,
                        math_settings,
                        source_map,
                    ));
                    out.push(']');
                    if meta.label_open_span.is_some() {
                        out.push('[');
                        out.push_str(&escape_text(label));
                        out.push(']');
                    }
                } else {
                    out.push_str("<span");
                    out.push_str(&span_attr);
                    out.push('>');
                    out.push('[');
                    out.push_str(&render_inlines_with_context(
                        children,
                        context,
                        math_counter,
                        math_settings,
                        source_map,
                    ));
                    out.push(']');
                    if meta.label_open_span.is_some() {
                        out.push('[');
                        out.push_str(&escape_text(label));
                        out.push(']');
                    }
                    out.push_str("</span>");
                }
            }
            InlineKind::Image { url, title, alt } => match context {
                RenderContext::ReferenceText => {
                    out.push_str(&render_inlines_with_context(
                        alt,
                        RenderContext::ReferenceText,
                        math_counter,
                        math_settings,
                        source_map,
                    ));
                }
                _ => {
                    out.push_str("<img src=\"");
                    out.push_str(&escape_url_attr(url));
                    out.push_str("\" alt=\"");
                    out.push_str(&escape_attr(&render_inlines_text(alt)));
                    out.push('"');
                    if let Some(title) = title {
                        out.push_str(" title=\"");
                        out.push_str(&escape_attr(title));
                        out.push('"');
                    }
                    out.push_str(&span_attr);
                    out.push_str(" />");
                }
            },
            InlineKind::ImageRef { label, alt, meta } => match context {
                RenderContext::ReferenceText => {
                    out.push_str(&render_inlines_with_context(
                        alt,
                        RenderContext::ReferenceText,
                        math_counter,
                        math_settings,
                        source_map,
                    ));
                }
                _ => {
                    if span_attr.is_empty() {
                        out.push_str("![");
                        out.push_str(&render_inlines_with_context(
                            alt,
                            context,
                            math_counter,
                            math_settings,
                            source_map,
                        ));
                        out.push(']');
                        if meta.label_open_span.is_some() {
                            out.push('[');
                            out.push_str(&escape_text(label));
                            out.push(']');
                        }
                    } else {
                        out.push_str("<span");
                        out.push_str(&span_attr);
                        out.push('>');
                        out.push_str("![");
                        out.push_str(&render_inlines_with_context(
                            alt,
                            context,
                            math_counter,
                            math_settings,
                            source_map,
                        ));
                        out.push(']');
                        if meta.label_open_span.is_some() {
                            out.push('[');
                            out.push_str(&escape_text(label));
                            out.push(']');
                        }
                        out.push_str("</span>");
                    }
                }
            },
            InlineKind::HtmlSpan { raw } => {
                if span_attr.is_empty() {
                    out.push_str(raw);
                } else {
                    out.push_str("<span");
                    out.push_str(&span_attr);
                    out.push('>');
                    out.push_str(raw);
                    out.push_str("</span>");
                }
            }
        }
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn render_ref(
    label: &Label,
    bracket: Option<&[Inline]>,
    resolved: Option<&ResolvedRef>,
    context: RenderContext,
    math_counter: &mut usize,
    math_settings: &MathSettings,
    source_map: Option<&SourceMap>,
    span: Span,
) -> String {
    let span_attr = span_attr(span, source_map);
    let display = if let Some(bracket) = bracket {
        render_inlines_with_context(
            bracket,
            RenderContext::ReferenceText,
            math_counter,
            math_settings,
            source_map,
        )
    } else if let Some(ResolvedRef::Block {
        display: Some(text),
        ..
    }) = resolved
    {
        render_inlines_with_context(
            text,
            RenderContext::ReferenceText,
            math_counter,
            math_settings,
            source_map,
        )
    } else {
        escape_text(&label.name)
    };

    match context {
        RenderContext::Normal | RenderContext::Title => {
            if resolved.is_some() {
                format!(
                    "<a class=\"TypMark-ref\"{} href=\"#{}\">{}</a>",
                    span_attr,
                    escape_attr(&label.name),
                    display
                )
            } else {
                format!(
                    "<span class=\"TypMark-ref ref-unresolved\"{} data-ref-label=\"{}\">{}</span>",
                    span_attr,
                    escape_attr(&label.name),
                    display
                )
            }
        }
        RenderContext::ReferenceText => {
            if resolved.is_some() {
                format!(
                    "<span class=\"TypMark-delink\"{}>{}</span>",
                    span_attr, display
                )
            } else {
                format!(
                    "<span class=\"TypMark-delink ref-unresolved\"{} data-ref-label=\"{}\">{}</span>",
                    span_attr,
                    escape_attr(&label.name),
                    display
                )
            }
        }
    }
}

fn task_input_html(checked: bool) -> String {
    if checked {
        "<input type=\"checkbox\" disabled=\"\" checked=\"\" /> ".to_string()
    } else {
        "<input type=\"checkbox\" disabled=\"\" /> ".to_string()
    }
}

fn emit_paragraph_with_prefix(writer: &mut HtmlWriter, content: &[Inline], prefix: &str) {
    let inline = render_inlines_with_context(
        content,
        RenderContext::Normal,
        &mut writer.math_counter,
        &writer.math_settings,
        writer.source_map.as_ref(),
    );
    writer.out.push_str(&"  ".repeat(writer.indent));
    writer.out.push_str("<p>");
    writer.out.push_str(prefix);
    writer.out.push_str(&inline);
    writer.out.push_str("</p>\n");
}

fn emit_table(writer: &mut HtmlWriter, table: &Table, attrs: &str) {
    writer.line(&format!("<table{}>", attrs));
    writer.indent += 1;
    writer.line("<thead>");
    writer.indent += 1;
    writer.line("<tr>");
    writer.indent += 1;
    for (idx, cell) in table.headers.iter().enumerate() {
        let align_attr = table_align_attr(table.aligns.get(idx).copied());
        let inline = render_inlines_with_context(
            cell,
            RenderContext::Normal,
            &mut writer.math_counter,
            &writer.math_settings,
            writer.source_map.as_ref(),
        );
        writer.line(&format!("<th{}>{}</th>", align_attr, inline));
    }
    writer.indent -= 1;
    writer.line("</tr>");
    writer.indent -= 1;
    writer.line("</thead>");
    if !table.rows.is_empty() {
        writer.line("<tbody>");
        writer.indent += 1;
        for row in &table.rows {
            writer.line("<tr>");
            writer.indent += 1;
            for (idx, cell) in row.iter().enumerate() {
                let align_attr = table_align_attr(table.aligns.get(idx).copied());
                let inline = render_inlines_with_context(
                    cell,
                    RenderContext::Normal,
                    &mut writer.math_counter,
                    &writer.math_settings,
                    writer.source_map.as_ref(),
                );
                writer.line(&format!("<td{}>{}</td>", align_attr, inline));
            }
            writer.indent -= 1;
            writer.line("</tr>");
        }
        writer.indent -= 1;
        writer.line("</tbody>");
    }
    writer.indent -= 1;
    writer.line("</table>");
}

fn table_align_attr(align: Option<TableAlign>) -> &'static str {
    match align.unwrap_or(TableAlign::None) {
        TableAlign::None => "",
        TableAlign::Left => " align=\"left\"",
        TableAlign::Center => " align=\"center\"",
        TableAlign::Right => " align=\"right\"",
    }
}

fn render_inlines_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match &inline.kind {
            InlineKind::Text(text) => out.push_str(text),
            InlineKind::CodeSpan(text) => out.push_str(text),
            InlineKind::MathInline { typst_src } => out.push_str(typst_src),
            InlineKind::SoftBreak | InlineKind::HardBreak => out.push('\n'),
            InlineKind::Ref { label, bracket, .. } => {
                if let Some(bracket) = bracket.as_deref() {
                    out.push_str(&render_inlines_text(bracket));
                } else {
                    out.push_str(&label.name);
                }
            }
            InlineKind::Emph(children)
            | InlineKind::Strong(children)
            | InlineKind::Strikethrough(children)
            | InlineKind::Link { children, .. }
            | InlineKind::LinkRef { children, .. } => {
                out.push_str(&render_inlines_text(children));
            }
            InlineKind::Image { alt, .. } | InlineKind::ImageRef { alt, .. } => {
                out.push_str(&render_inlines_text(alt));
            }
            InlineKind::HtmlSpan { raw } => out.push_str(raw),
        }
    }
    out
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

fn escape_text(text: &str) -> String {
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

fn escape_html_code(text: &str) -> String {
    // Escape HTML for code contents.
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
            b'`' => encoded.push_str("%60"),
            b'\\' => encoded.push_str("%5C"),
            b'"' => encoded.push_str("%22"),
            0x00..=0x1F | 0x7F..=0xFF => {
                encoded.push_str(&format!("%{:02X}", byte));
            }
            _ => encoded.push(byte as char),
        }
    }
    escape_attr(&encoded)
}

fn data_attrs(items: &[AttrItem]) -> String {
    let mut out = String::new();
    for item in items {
        out.push_str(&format!(
            " data-{}=\"{}\"",
            escape_attr(&item.key),
            escape_attr(&item.value.raw)
        ));
    }
    out
}

fn span_attr(span: Span, source_map: Option<&SourceMap>) -> String {
    let Some(source_map) = source_map else {
        return String::new();
    };
    let range = source_map.range(span);
    format!(
        " data-tm-range=\"{}:{}-{}:{}\"",
        range.start.line, range.start.character, range.end.line, range.end.character
    )
}

fn compose_block_attrs_with_span(
    label: Option<&Label>,
    items: &[AttrItem],
    span: Span,
    source_map: Option<&SourceMap>,
) -> String {
    let mut out = id_attr(label);
    out.push_str(&span_attr(span, source_map));
    out.push_str(&data_attrs(items));
    out
}

fn id_attr(label: Option<&Label>) -> String {
    label
        .map(|label| format!(" id=\"{}\"", escape_attr(&label.name)))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::{SVG_ALLOWED_ATTRS, SVG_ALLOWED_TAGS};
    use crate::math::{MathSettings, render_math};
    use std::collections::{BTreeMap, BTreeSet};

    fn collect_svg_tags(svg: &str) -> BTreeMap<String, BTreeSet<String>> {
        let document = roxmltree::Document::parse(svg).expect("parse svg");
        let mut tags: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

        for node in document.descendants().filter(|node| node.is_element()) {
            let tag = node.tag_name().name().to_string();
            let entry = tags.entry(tag).or_default();
            for attr in node.attributes() {
                entry.insert(attr.name().to_string());
            }
        }

        tags
    }

    #[test]
    fn svg_allowlist_matches_rendered_math() {
        let samples = [
            "x",
            "x^2",
            "a / b",
            "sqrt(2)",
            "sum_(i=1)^n i",
            "integral_0^1 x^2 dif x",
            "vec(a, b, c)",
            "mat(1, 2; 3, 4)",
            "cases(1 \"if\" x > 0, 0 \"else\")",
        ];

        let mut observed: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for display in [false, true] {
            for sample in samples {
                let svg = render_math(sample, display, &MathSettings::default())
                    .unwrap_or_else(|_| panic!("math render failed for: {}", sample));
                let tags = collect_svg_tags(&svg);
                for (tag, attrs) in tags {
                    observed.entry(tag).or_default().extend(attrs);
                }
            }
        }

        let expected_tags: BTreeSet<String> = SVG_ALLOWED_TAGS
            .iter()
            .map(|tag| (*tag).to_string())
            .collect();
        let observed_tags: BTreeSet<String> = observed.keys().cloned().collect();
        assert_eq!(observed_tags, expected_tags, "SVG tag allowlist mismatch");

        let mut expected_attrs: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for (tag, attrs) in SVG_ALLOWED_ATTRS {
            let entry = expected_attrs.entry((*tag).to_string()).or_default();
            for attr in *attrs {
                entry.insert((*attr).to_string());
            }
        }

        assert_eq!(observed, expected_attrs, "SVG attribute allowlist mismatch");
    }
}
