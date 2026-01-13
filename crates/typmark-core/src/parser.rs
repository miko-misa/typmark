use crate::ast::{
    AttrItem, AttrList, AttrValue, Block, BlockKind, BoxBlock, CodeBlock, CodeBlockKind, CodeMeta,
    Document, Inline, InlineKind, InlineSeq, Label, LineLabel, LineRange, LinkDefinition,
    LinkRefMeta, List, ListItem, Table, TableAlign,
};
use crate::diagnostic::{
    Diagnostic, DiagnosticSeverity, E_ATTR_SYNTAX, E_CODE_CONFLICT, E_MATH_INLINE_NL,
    E_REF_BRACKET_NL, E_TARGET_ORPHAN, W_BOX_STYLE_INVALID, W_CODE_RANGE_OOB,
};
use crate::entities::lookup_named_entity;
use crate::label::{is_label_escape, normalize_link_label};
use crate::source_map::SourceMap;
use crate::span::Span;
use std::collections::HashMap;

pub struct ParseResult {
    pub document: Document,
    pub diagnostics: Vec<Diagnostic>,
    pub source_map: SourceMap,
    pub link_defs: HashMap<String, LinkDefinition>,
}

pub fn parse(source: &str) -> ParseResult {
    let mut prepass = Parser::new(source);
    let _ = prepass.parse_document_with_mode(false);
    let mut parser = Parser::new(source);
    parser.link_defs = prepass.link_defs;
    let document = parser.parse_document();
    ParseResult {
        document,
        diagnostics: parser.diagnostics,
        source_map: parser.source_map,
        link_defs: parser.link_defs,
    }
}

struct Parser {
    source: String,
    lines: Vec<Line>,
    diagnostics: Vec<Diagnostic>,
    source_map: SourceMap,
    link_defs: HashMap<String, LinkDefinition>,
}

#[derive(Clone, Debug)]
struct Line {
    text: String,
    start: usize,
    end: usize,
    has_newline: bool,
    lazy_continuation: bool,
}

#[derive(Clone, Debug)]
struct Delimiter {
    ch: u8,
    len: usize,
    node_index: usize,
    can_open: bool,
    can_close: bool,
    orig_can_open: bool,
    orig_can_close: bool,
}

#[derive(Clone, Debug)]
struct BracketEntry {
    node_index: usize,
    start: usize,
    image: bool,
    active: bool,
}

impl Parser {
    fn new(source: &str) -> Self {
        let source_map = SourceMap::new(source);
        let lines = split_lines(source);
        Self {
            source: source.to_string(),
            lines,
            diagnostics: Vec::new(),
            source_map,
            link_defs: HashMap::new(),
        }
    }

    fn parse_document(&mut self) -> Document {
        self.parse_document_with_mode(true)
    }

    fn parse_document_with_mode(&mut self, parse_inlines: bool) -> Document {
        let span = Span {
            start: 0,
            end: self.source.len(),
        };
        // Phase 0: line-based block parsing with a minimal block set.
        let lines = self.lines.clone();
        let blocks = self.parse_blocks(&lines, parse_inlines);
        Document { span, blocks }
    }

    fn parse_blocks(&mut self, lines: &[Line], parse_inlines: bool) -> Vec<Block> {
        let mut blocks = Vec::new();
        let mut i = 0;
        // Target-line attributes are scoped to the current container only.
        let mut pending_attrs: Option<AttrList> = None;

        while i < lines.len() {
            let line = &lines[i];
            if line.text.trim().is_empty() {
                i += 1;
                continue;
            }

            if let Some(attrs) = self.try_parse_target_line(line) {
                if let Some(prev) = pending_attrs.take()
                    && let Some(span) = prev.span
                {
                    self.push_diag(
                        span,
                        DiagnosticSeverity::Error,
                        E_TARGET_ORPHAN,
                        "target line has no following block",
                    );
                }
                pending_attrs = Some(attrs);
                i += 1;
                continue;
            }

            if let Some((block, next)) = self.parse_code_block(lines, i) {
                let mut block = block;
                self.finalize_block(&mut block, &mut pending_attrs);
                blocks.push(block);
                i = next;
                continue;
            }

            if let Some((block, next)) = self.parse_indented_code_block(lines, i) {
                let mut block = block;
                self.finalize_block(&mut block, &mut pending_attrs);
                blocks.push(block);
                i = next;
                continue;
            }

            if let Some((block, next)) = self.parse_math_block(lines, i) {
                let mut block = block;
                self.finalize_block(&mut block, &mut pending_attrs);
                blocks.push(block);
                i = next;
                continue;
            }

            if let Some((block, next)) = self.parse_box_block(lines, i, parse_inlines) {
                let mut block = block;
                self.finalize_block(&mut block, &mut pending_attrs);
                blocks.push(block);
                i = next;
                continue;
            }

            if let Some((block, next)) = self.parse_html_block(lines, i) {
                let mut block = block;
                self.finalize_block(&mut block, &mut pending_attrs);
                blocks.push(block);
                i = next;
                continue;
            }

            if let Some((block, next)) = self.parse_thematic_break(lines, i) {
                let mut block = block;
                self.finalize_block(&mut block, &mut pending_attrs);
                blocks.push(block);
                i = next;
                continue;
            }

            if let Some((block, next)) = self.parse_block_quote(lines, i, parse_inlines) {
                let mut block = block;
                self.finalize_block(&mut block, &mut pending_attrs);
                blocks.push(block);
                i = next;
                continue;
            }

            if let Some((block, next)) = self.parse_list(lines, i, parse_inlines) {
                let mut block = block;
                self.finalize_block(&mut block, &mut pending_attrs);
                blocks.push(block);
                i = next;
                continue;
            }

            if let Some((block, next)) = self.parse_table(lines, i, parse_inlines) {
                let mut block = block;
                self.finalize_block(&mut block, &mut pending_attrs);
                blocks.push(block);
                i = next;
                continue;
            }

            if let Some((block, next)) = self.parse_heading(lines, i, parse_inlines) {
                let mut block = block;
                self.finalize_block(&mut block, &mut pending_attrs);
                blocks.push(block);
                i = next;
                continue;
            }

            let (block, next) = self.parse_paragraph(lines, i, parse_inlines);
            if let Some(mut block) = block {
                self.finalize_block(&mut block, &mut pending_attrs);
                blocks.push(block);
            }
            i = next;
        }

        if let Some(attrs) = pending_attrs
            && let Some(span) = attrs.span
        {
            self.push_diag(
                span,
                DiagnosticSeverity::Error,
                E_TARGET_ORPHAN,
                "target line has no following block",
            );
        }

        blocks
    }

    fn apply_pending_attrs(&mut self, block: &mut Block, pending: &mut Option<AttrList>) {
        if let Some(mut attrs) = pending.take() {
            if let Some(label) = attrs.label.take() {
                if block.attrs.label.is_some() {
                    self.push_diag(
                        label.span,
                        DiagnosticSeverity::Error,
                        E_ATTR_SYNTAX,
                        "duplicate label",
                    );
                } else {
                    block.attrs.label = Some(label);
                }
            }
            if let Some(span) = attrs.span {
                block.attrs.span = Some(span);
            }
            if !attrs.items.is_empty() {
                block.attrs.items.extend(attrs.items);
            }
        }
    }

    fn finalize_block(&mut self, block: &mut Block, pending: &mut Option<AttrList>) {
        self.apply_pending_attrs(block, pending);
        if let BlockKind::Box(_) = block.kind {
            self.validate_box_styles(&block.attrs);
        }
    }

    fn parse_heading(
        &mut self,
        lines: &[Line],
        i: usize,
        parse_inlines: bool,
    ) -> Option<(Block, usize)> {
        let line = &lines[i];
        let (level, content_start, content_end) = parse_atx_heading(&line.text)?;
        let rest = &line.text[content_start..content_end];
        let title = if parse_inlines {
            self.parse_inline(rest, line.start + content_start)
        } else {
            Vec::new()
        };
        let span = Span {
            start: line.start,
            end: line.end,
        };
        Some((
            Block {
                span,
                attrs: AttrList::default(),
                kind: BlockKind::Heading { level, title },
            },
            i + 1,
        ))
    }

    fn parse_table(
        &mut self,
        lines: &[Line],
        start: usize,
        parse_inlines: bool,
    ) -> Option<(Block, usize)> {
        let line = lines.get(start)?;
        let (header_offset, header_text) = table_line_view(&line.text)?;
        let (header_cells, header_has_pipe) = split_table_cells(header_text, header_offset);
        if !header_has_pipe {
            return None;
        }

        let separator = lines.get(start + 1)?;
        let (sep_offset, sep_text) = table_line_view(&separator.text)?;
        let aligns = parse_table_separator(sep_text, sep_offset)?;
        if aligns.is_empty() {
            return None;
        }

        let headers =
            parse_table_cells(self, line.start, &header_cells, aligns.len(), parse_inlines);

        let mut rows = Vec::new();
        let mut i = start + 2;
        while i < lines.len() {
            let row_line = &lines[i];
            if row_line.text.trim().is_empty() {
                break;
            }
            let (row_offset, row_text) = match table_line_view(&row_line.text) {
                Some(value) => value,
                None => break,
            };
            let (row_cells, row_has_pipe) = split_table_cells(row_text, row_offset);
            if !row_has_pipe {
                break;
            }
            let row = parse_table_cells(
                self,
                row_line.start,
                &row_cells,
                aligns.len(),
                parse_inlines,
            );
            rows.push(row);
            i += 1;
        }

        let span = Span {
            start: line.start,
            end: lines[i.saturating_sub(1)].end,
        };
        Some((
            Block {
                span,
                attrs: AttrList::default(),
                kind: BlockKind::Table(Table {
                    headers,
                    aligns,
                    rows,
                }),
            },
            i,
        ))
    }

    fn parse_thematic_break(&self, lines: &[Line], start: usize) -> Option<(Block, usize)> {
        let line = &lines[start];
        if !is_thematic_break_line(&line.text) {
            return None;
        }
        let span = Span {
            start: line.start,
            end: line.end,
        };
        Some((
            Block {
                span,
                attrs: AttrList::default(),
                kind: BlockKind::ThematicBreak,
            },
            start + 1,
        ))
    }

    fn parse_paragraph(
        &mut self,
        lines: &[Line],
        start: usize,
        parse_inlines: bool,
    ) -> (Option<Block>, usize) {
        let mut i = start;
        let mut content_lines: Vec<Line> = Vec::new();
        let mut setext_level = None;
        let mut setext_end = start;

        while i < lines.len() {
            let line = &lines[i];
            if line.text.trim().is_empty() {
                break;
            }
            if let Some(kind) = match_html_block_start(&line.text) {
                if !matches!(kind, HtmlBlockKind::Type7) {
                    break;
                }
            } else if let Some(marker) = parse_list_marker(&line.text) {
                if !marker.empty && (!marker.ordered || marker.start == Some(1)) {
                    break;
                }
            } else if self.is_block_start(line) {
                break;
            }
            if let Some((label, definition, next)) = parse_link_reference_definition_lines(lines, i)
                && content_lines.is_empty()
            {
                self.link_defs.entry(label).or_insert(definition);
                i = next;
                continue;
            }
            content_lines.push(line.clone());
            if let Some(next) = lines.get(i + 1)
                && !line.lazy_continuation
                && let Some(level) = setext_underline_level(&next.text)
            {
                setext_level = Some(level);
                setext_end = i + 1;
                break;
            }
            i += 1;
        }

        if content_lines.is_empty() {
            return (None, i);
        }

        let (span_start, span_end) = match (content_lines.first(), content_lines.last()) {
            (Some(first), Some(last)) => (first.start, last.end),
            _ => (lines[start].start, lines[start].end),
        };
        if let Some(level) = setext_level {
            let (buffer, offsets) = self.build_heading_buffer(&content_lines);
            let content = if parse_inlines {
                self.parse_inline_buffer(&buffer, &offsets)
            } else {
                Vec::new()
            };
            let span = Span {
                start: span_start,
                end: lines[setext_end].end,
            };
            return (
                Some(Block {
                    span,
                    attrs: AttrList::default(),
                    kind: BlockKind::Heading {
                        level,
                        title: content,
                    },
                }),
                setext_end + 1,
            );
        }

        let (buffer, offsets) = self.build_inline_buffer(&content_lines);
        let content = if parse_inlines {
            self.parse_inline_buffer(&buffer, &offsets)
        } else {
            Vec::new()
        };

        let block = Block {
            span: Span {
                start: span_start,
                end: span_end,
            },
            attrs: AttrList::default(),
            kind: BlockKind::Paragraph { content },
        };
        (Some(block), i)
    }

    fn parse_code_block(&mut self, lines: &[Line], start: usize) -> Option<(Block, usize)> {
        let line = &lines[start];
        let (indent_len, fence_len, fence_char, info) = parse_fence_open(&line.text)?;
        let (lang, info_attrs) = self.parse_fence_info(line, fence_len, info);

        let mut code_lines: Vec<String> = Vec::new();
        let mut i = start + 1;
        while i < lines.len() {
            let candidate = &lines[i];
            if is_fence_close(&candidate.text, fence_len, fence_char) {
                i += 1;
                break;
            }
            let text = strip_leading_spaces(&candidate.text, indent_len);
            code_lines.push(text.to_string());
            i += 1;
        }
        let text = code_lines.join("\n");
        let meta = self.parse_code_meta(&info_attrs, &text, line.start, line.end);
        let mut block_attrs = AttrList::default();
        if let Some(label) = info_attrs.label.clone() {
            block_attrs.span = info_attrs.span;
            block_attrs.label = Some(label);
        }
        let span = Span {
            start: line.start,
            end: if i == 0 {
                line.end
            } else {
                lines[i.saturating_sub(1)].end
            },
        };
        Some((
            Block {
                span,
                attrs: block_attrs,
                kind: BlockKind::CodeBlock(CodeBlock {
                    kind: CodeBlockKind::Fenced,
                    lang,
                    info_attrs,
                    meta,
                    text,
                }),
            },
            i,
        ))
    }

    fn parse_indented_code_block(&self, lines: &[Line], start: usize) -> Option<(Block, usize)> {
        let line = &lines[start];
        indent_prefix_len(&line.text, 4)?;
        let mut code_lines: Vec<String> = Vec::new();
        let mut pending_blank: Vec<usize> = Vec::new();
        let mut i = start;
        let mut last_line_idx = start;

        while i < lines.len() {
            let current = &lines[i];
            if current.text.trim().is_empty() {
                pending_blank.push(i);
                i += 1;
                continue;
            }
            if indent_prefix_len(&current.text, 4).is_none() {
                break;
            }
            if !pending_blank.is_empty() {
                for _ in pending_blank.drain(..) {
                    code_lines.push(String::new());
                }
            }
            // Remove 4 columns of indentation, properly handling tabs
            let content = remove_indent_columns(&current.text, 4);
            code_lines.push(content);
            last_line_idx = i;
            i += 1;
        }

        let text = code_lines.join("\n");
        let span = Span {
            start: line.start,
            end: lines[last_line_idx].end,
        };
        let meta = CodeMeta {
            hl: Vec::new(),
            diff_add: Vec::new(),
            diff_del: Vec::new(),
            line_labels: Vec::new(),
        };
        Some((
            Block {
                span,
                attrs: AttrList::default(),
                kind: BlockKind::CodeBlock(CodeBlock {
                    kind: CodeBlockKind::Indented,
                    lang: None,
                    info_attrs: AttrList::default(),
                    meta,
                    text,
                }),
            },
            i,
        ))
    }

    fn parse_math_block(&mut self, lines: &[Line], start: usize) -> Option<(Block, usize)> {
        let line = &lines[start];
        let trimmed = line.text.trim();
        if !trimmed.starts_with("$$") {
            return None;
        }
        if trimmed != "$$" && trimmed.ends_with("$$") && trimmed.len() > 4 {
            let content = trimmed.trim_start_matches("$$");
            let content = content.trim_end_matches("$$");
            let typst_src = content.to_string();
            let span = Span {
                start: line.start,
                end: line.end,
            };
            return Some((
                Block {
                    span,
                    attrs: AttrList::default(),
                    kind: BlockKind::MathBlock { typst_src },
                },
                start + 1,
            ));
        }
        let mut i = start + 1;
        let mut body_lines = Vec::new();
        while i < lines.len() {
            let candidate = &lines[i];
            if candidate.text.trim() == "$$" {
                i += 1;
                break;
            }
            body_lines.push(candidate.text.clone());
            i += 1;
        }
        let typst_src = body_lines.join("\n");
        let span = Span {
            start: line.start,
            end: if i == 0 {
                line.end
            } else {
                lines[i.saturating_sub(1)].end
            },
        };
        Some((
            Block {
                span,
                attrs: AttrList::default(),
                kind: BlockKind::MathBlock { typst_src },
            },
            i,
        ))
    }

    fn parse_box_block(
        &mut self,
        lines: &[Line],
        start: usize,
        parse_inlines: bool,
    ) -> Option<(Block, usize)> {
        let line = &lines[start];
        if !line.text.starts_with(":::") {
            return None;
        }
        let fence_len = line.text.chars().take_while(|c| *c == ':').count();
        if fence_len < 3 {
            return None;
        }
        let rest = line.text[fence_len..].trim_start();
        if !rest.starts_with("box") {
            return None;
        }
        let title_text = rest.strip_prefix("box").unwrap_or("").trim_start();
        let title = if title_text.is_empty() {
            None
        } else if parse_inlines {
            Some(self.parse_inline(
                title_text,
                line.start + (line.text.len() - title_text.len()),
            ))
        } else {
            None
        };

        let mut i = start + 1;
        let mut inner_lines = Vec::new();
        let mut fence_stack = vec![fence_len];
        while i < lines.len() {
            let candidate = &lines[i];
            let trimmed = candidate.text.trim();
            if let Some((_, inner_fence_len, fence_char, _)) = parse_fence_open(&candidate.text) {
                inner_lines.push(candidate.clone());
                i += 1;
                while i < lines.len() {
                    let inner = &lines[i];
                    inner_lines.push(inner.clone());
                    let inner_trimmed = inner.text.trim();
                    let fence_count = inner_trimmed
                        .chars()
                        .take_while(|c| *c as u8 == fence_char)
                        .count();
                    i += 1;
                    if fence_count >= inner_fence_len
                        && inner_trimmed.chars().all(|c| c as u8 == fence_char)
                    {
                        break;
                    }
                }
                continue;
            }
            if trimmed == "$$" {
                inner_lines.push(candidate.clone());
                i += 1;
                while i < lines.len() {
                    let inner = &lines[i];
                    inner_lines.push(inner.clone());
                    i += 1;
                    if inner.text.trim() == "$$" {
                        break;
                    }
                }
                continue;
            }
            if self.is_box_open(&candidate.text) {
                let nested_len = candidate.text.chars().take_while(|c| *c == ':').count();
                fence_stack.push(nested_len);
                inner_lines.push(candidate.clone());
                i += 1;
                continue;
            }
            let colons = trimmed.chars().take_while(|c| *c == ':').count();
            if colons >= 3
                && trimmed.chars().all(|c| c == ':')
                && let Some(&top) = fence_stack.last()
                && colons >= top
            {
                fence_stack.pop();
                if fence_stack.is_empty() {
                    i += 1;
                    break;
                }
                inner_lines.push(candidate.clone());
                i += 1;
                continue;
            }
            inner_lines.push(candidate.clone());
            i += 1;
        }
        let blocks = self.parse_blocks(&inner_lines, parse_inlines);
        let span = Span {
            start: line.start,
            end: if i == 0 {
                line.end
            } else {
                lines[i.saturating_sub(1)].end
            },
        };
        Some((
            Block {
                span,
                attrs: AttrList::default(),
                kind: BlockKind::Box(BoxBlock { title, blocks }),
            },
            i,
        ))
    }

    fn parse_html_block(&mut self, lines: &[Line], start: usize) -> Option<(Block, usize)> {
        let line = &lines[start];
        let kind = match_html_block_start(&line.text)?;
        let mut raw_lines = vec![line.text.clone()];
        let mut i = start + 1;

        if !matches!(kind, HtmlBlockKind::Type6 | HtmlBlockKind::Type7)
            && html_block_end(kind, &line.text)
        {
            let span = Span {
                start: line.start,
                end: line.end,
            };
            return Some((
                Block {
                    span,
                    attrs: AttrList::default(),
                    kind: BlockKind::HtmlBlock {
                        raw: raw_lines.join("\n"),
                    },
                },
                i,
            ));
        }

        match kind {
            HtmlBlockKind::Type6 | HtmlBlockKind::Type7 => {
                while i < lines.len() {
                    let next = &lines[i];
                    if next.text.trim().is_empty() {
                        break;
                    }
                    raw_lines.push(next.text.clone());
                    i += 1;
                }
            }
            _ => {
                while i < lines.len() {
                    let next = &lines[i];
                    raw_lines.push(next.text.clone());
                    if html_block_end(kind, &next.text) {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
            }
        }

        let end_line_idx = if i == 0 { start } else { i.saturating_sub(1) };
        let span = Span {
            start: line.start,
            end: lines[end_line_idx].end,
        };
        Some((
            Block {
                span,
                attrs: AttrList::default(),
                kind: BlockKind::HtmlBlock {
                    raw: raw_lines.join("\n"),
                },
            },
            i,
        ))
    }

    fn parse_block_quote(
        &mut self,
        lines: &[Line],
        start: usize,
        parse_inlines: bool,
    ) -> Option<(Block, usize)> {
        let line = &lines[start];
        blockquote_prefix_info(&line.text)?;
        let mut i = start;
        let mut quote_lines = Vec::new();
        let mut can_lazy = false;
        while i < lines.len() {
            let candidate = &lines[i];
            if let Some((prefix_bytes, partially_consumed_tab, remaining_tab_cols, current_col)) =
                blockquote_prefix_info(&candidate.text)
            {
                let mut text = String::new();
                let mut col = current_col;

                // If we partially consumed a tab, emit the remaining columns as spaces
                // and skip the tab byte itself
                let content_start = if partially_consumed_tab {
                    for _ in 0..remaining_tab_cols {
                        text.push(' ');
                        col += 1;
                    }
                    prefix_bytes + 1 // Skip the partially consumed tab byte
                } else {
                    prefix_bytes
                };

                // Append the rest of the line, expanding tabs relative to current column
                let rest = &candidate.text[content_start..];
                for byte in rest.bytes() {
                    match byte {
                        b'\t' => {
                            let next_tab_stop = col + (4 - (col % 4));
                            while col < next_tab_stop {
                                text.push(' ');
                                col += 1;
                            }
                        }
                        b => {
                            text.push(b as char);
                            col += 1;
                        }
                    }
                }

                let line = Line {
                    text,
                    start: candidate.start + prefix_bytes,
                    end: candidate.end,
                    has_newline: candidate.has_newline,
                    lazy_continuation: false,
                };
                let list_allows_lazy = parse_list_marker(&line.text).is_some_and(|marker| {
                    remove_list_indent(&line.text, marker.marker_len, marker.content_indent)
                        .trim_start()
                        .starts_with('>')
                });
                can_lazy = self.line_can_continue_paragraph(&line)
                    || line.text.trim_start().starts_with('>')
                    || list_allows_lazy;
                quote_lines.push(line);
                i += 1;
                continue;
            }
            if candidate.text.trim().is_empty() {
                break;
            }
            if can_lazy {
                if !self.line_can_continue_paragraph(candidate)
                    && setext_underline_level(&candidate.text).is_none()
                {
                    break;
                }
                if is_thematic_break_line(&candidate.text) {
                    break;
                }
                if let Some(last) = quote_lines.last()
                    && indent_prefix_len(&last.text, 4).is_some()
                {
                    break;
                }
                quote_lines.push(Line {
                    text: candidate.text.clone(),
                    start: candidate.start,
                    end: candidate.end,
                    has_newline: candidate.has_newline,
                    lazy_continuation: true,
                });
                can_lazy = true;
                i += 1;
                continue;
            }
            break;
        }
        let blocks = self.parse_blocks(&quote_lines, parse_inlines);
        let span = Span {
            start: line.start,
            end: if i == 0 {
                line.end
            } else {
                lines[i.saturating_sub(1)].end
            },
        };
        Some((
            Block {
                span,
                attrs: AttrList::default(),
                kind: BlockKind::BlockQuote { blocks },
            },
            i,
        ))
    }

    fn parse_list(
        &mut self,
        lines: &[Line],
        start: usize,
        parse_inlines: bool,
    ) -> Option<(Block, usize)> {
        let line = &lines[start];
        let marker = parse_list_marker(&line.text)?;
        let mut i = start;
        let mut items = Vec::new();
        let mut item_blanks = Vec::new();
        let mut list_has_blank = false;
        let list_start = marker.start;
        let mut list_end = lines[start].end;

        while i < lines.len() {
            let current = &lines[i];
            let current_marker = match parse_list_marker(&current.text) {
                Some(marker) => marker,
                None => break,
            };
            if current_marker.ordered != marker.ordered || current_marker.marker != marker.marker {
                break;
            }
            let marker_len = current_marker.marker_len;
            let content_indent = current_marker.content_indent;
            let mut item_lines = Vec::new();
            let mut last_line_idx = i;
            // For the first line, we need to remove marker + content_indent
            // This properly handles tabs in list markers
            let first_text = remove_list_indent(&current.text, marker_len, content_indent);
            let mut seen_content = !first_text.trim().is_empty();
            let mut initial_blank_lines = if seen_content { 0 } else { 1 };
            item_lines.push(Line {
                text: first_text,
                start: current.start + marker_len,
                end: current.end,
                has_newline: current.has_newline,
                lazy_continuation: false,
            });
            let mut can_lazy = {
                let line = item_lines.last().unwrap_or(current);
                self.line_can_continue_paragraph(line) || line.text.trim_start().starts_with('>')
            };
            let mut j = i + 1;
            let mut pending_blank: Vec<Line> = Vec::new();
            while j < lines.len() {
                let next = &lines[j];
                if next.text.trim().is_empty() {
                    if !seen_content {
                        if initial_blank_lines >= 1 {
                            // Allow list continuation after a blank line if a new item follows.
                            let mut k = j + 1;
                            while k < lines.len() && lines[k].text.trim().is_empty() {
                                list_has_blank = true;
                                k += 1;
                            }
                            if k < lines.len()
                                && let Some(next_marker) = parse_list_marker(&lines[k].text)
                                && next_marker.ordered == marker.ordered
                                && next_marker.marker == marker.marker
                            {
                                list_has_blank = true;
                                j = k;
                                break;
                            }
                            break;
                        }
                        initial_blank_lines += 1;
                    }
                    pending_blank.push(next.clone());
                    can_lazy = false;
                    j += 1;
                    continue;
                }
                if indent_prefix_len(&next.text, content_indent).is_some() {
                    if !pending_blank.is_empty() {
                        for blank in pending_blank.drain(..) {
                            item_lines.push(Line {
                                text: String::new(),
                                start: blank.start,
                                end: blank.end,
                                has_newline: blank.has_newline,
                                lazy_continuation: false,
                            });
                        }
                    }
                    // Use remove_indent_columns to properly expand tabs
                    let content_text = remove_indent_columns(&next.text, content_indent);
                    item_lines.push(Line {
                        text: content_text,
                        start: next.start,
                        end: next.end,
                        has_newline: next.has_newline,
                        lazy_continuation: false,
                    });
                    seen_content = true;
                    can_lazy = {
                        let line = item_lines.last().unwrap_or(next);
                        self.line_can_continue_paragraph(line)
                            || line.text.trim_start().starts_with('>')
                    };
                    last_line_idx = j;
                    j += 1;
                    continue;
                }
                if let Some(next_marker) = parse_list_marker(&next.text) {
                    if next_marker.ordered == marker.ordered
                        && next_marker.marker == marker.marker
                        && !pending_blank.is_empty()
                    {
                        list_has_blank = true;
                    }
                    break;
                }
                if pending_blank.is_empty()
                    && can_lazy
                    && setext_underline_level(&next.text).is_none()
                    && self.line_can_continue_paragraph(next)
                {
                    item_lines.push(next.clone());
                    seen_content = true;
                    can_lazy = true;
                    last_line_idx = j;
                    j += 1;
                    continue;
                }
                break;
            }
            let mut blocks = self.parse_blocks(&item_lines, parse_inlines);
            let item_has_blank = item_has_blank_between_blocks(&item_lines, &blocks);
            let span = Span {
                start: current.start,
                end: lines[last_line_idx].end,
            };
            let task = if parse_inlines {
                detect_task_marker(&mut blocks)
            } else {
                None
            };
            items.push(ListItem { span, blocks, task });
            item_blanks.push(item_has_blank);
            list_end = span.end;
            i = j;
        }

        let mut tight = !list_has_blank;
        if tight {
            for item_blank in &item_blanks {
                if *item_blank {
                    tight = false;
                    break;
                }
            }
        }

        let span = Span {
            start: lines[start].start,
            end: list_end,
        };
        Some((
            Block {
                span,
                attrs: AttrList::default(),
                kind: BlockKind::List(List {
                    ordered: marker.ordered,
                    start: list_start,
                    tight,
                    items,
                }),
            },
            i,
        ))
    }

    fn is_block_start(&self, line: &Line) -> bool {
        self.is_code_fence_line(&line.text)
            || line.text.trim() == "$$"
            || self.is_box_open(&line.text)
            || self.is_html_block_start(&line.text)
            || blockquote_prefix_len(&line.text).is_some()
            || is_thematic_break_line(&line.text)
            || parse_list_marker(&line.text).is_some()
            || self.is_heading_line(&line.text)
            || self.is_target_line_text(&line.text)
    }

    fn is_html_block_start(&self, text: &str) -> bool {
        match_html_block_start(text).is_some()
    }

    fn line_can_continue_paragraph(&self, line: &Line) -> bool {
        if line.text.trim().is_empty() {
            return false;
        }
        if setext_underline_level(&line.text).is_some() {
            return false;
        }
        if let Some(kind) = match_html_block_start(&line.text) {
            if !matches!(kind, HtmlBlockKind::Type7) {
                return false;
            }
        } else if let Some(marker) = parse_list_marker(&line.text) {
            if !marker.empty && (!marker.ordered || marker.start == Some(1)) {
                return false;
            }
        } else if self.is_block_start(line) {
            return false;
        }
        true
    }

    fn parse_inline(&mut self, text: &str, start_offset: usize) -> InlineSeq {
        if text.is_empty() {
            return Vec::new();
        }
        let mut offsets = Vec::with_capacity(text.len());
        for idx in 0..text.len() {
            offsets.push(start_offset + idx);
        }
        self.parse_inline_buffer(text, &offsets)
    }

    fn build_inline_buffer(&self, lines: &[Line]) -> (String, Vec<usize>) {
        // Flatten paragraph lines into a single buffer with explicit newlines.
        let mut buffer = String::new();
        let mut offsets = Vec::new();
        for (idx, line) in lines.iter().enumerate() {
            let mut text = line.text.as_str();
            let mut start_offset = line.start;
            let mut removed = 0usize;
            for ch in text.chars() {
                if ch == ' ' && removed < 3 {
                    removed += 1;
                } else {
                    break;
                }
            }
            if removed > 0 {
                text = &text[removed..];
                start_offset = start_offset.saturating_add(removed);
            }
            if start_offset > line.end {
                start_offset = line.end;
            }
            if idx + 1 == lines.len() {
                text = text.trim_end_matches([' ', '\t']);
            }
            buffer.push_str(text);
            // Map each byte in the buffer back to the original source offset.
            for byte_idx in 0..text.len() {
                let max_len = line.end.saturating_sub(start_offset);
                let offset = if byte_idx < max_len {
                    start_offset + byte_idx
                } else if line.end > 0 {
                    line.end - 1
                } else {
                    0
                };
                offsets.push(offset);
            }
            if line.has_newline && idx + 1 < lines.len() {
                buffer.push('\n');
                offsets.push(line.end);
            }
        }
        (buffer, offsets)
    }

    fn build_heading_buffer(&self, lines: &[Line]) -> (String, Vec<usize>) {
        let (buffer, offsets) = self.build_inline_buffer(lines);
        let bytes = buffer.as_bytes();
        let mut start = 0;
        while start < bytes.len() && (bytes[start] == b' ' || bytes[start] == b'\t') {
            start += 1;
        }
        let mut end = bytes.len();
        while end > start && (bytes[end - 1] == b' ' || bytes[end - 1] == b'\t') {
            end -= 1;
        }
        if start >= end {
            return (String::new(), Vec::new());
        }
        (buffer[start..end].to_string(), offsets[start..end].to_vec())
    }

    fn parse_inline_buffer(&mut self, buffer: &str, offsets: &[usize]) -> InlineSeq {
        self.parse_inline_range(buffer, offsets, 0, buffer.len())
    }

    fn parse_inline_range(
        &mut self,
        buffer: &str,
        offsets: &[usize],
        start: usize,
        end: usize,
    ) -> InlineSeq {
        // Inline parsing uses delimiter and bracket stacks (ASCII-focused CommonMark).
        let bytes = buffer.as_bytes();
        let mut out: InlineSeq = Vec::new();
        let mut delims: Vec<Delimiter> = Vec::new();
        let mut brackets: Vec<BracketEntry> = Vec::new();
        let mut text_buf: Vec<u8> = Vec::new();
        let mut text_start = start;
        let mut i = start;

        while i < end {
            let b = bytes[i];
            match b {
                b'\\' => {
                    if i + 1 < end {
                        let next = bytes[i + 1];
                        if next == b'\n' {
                            self.flush_text_buf(
                                &mut out,
                                offsets,
                                &mut text_buf,
                                &mut text_start,
                                i,
                            );
                            let span = self.span_from_offsets(offsets, i, i + 2);
                            out.push(Inline {
                                span,
                                kind: InlineKind::HardBreak,
                            });
                            i += 2;
                            text_start = i;
                            continue;
                        }
                        if is_ascii_punctuation(next) {
                            if text_buf.is_empty() {
                                text_start = i;
                            }
                            text_buf.push(next);
                            i += 2;
                            continue;
                        }
                    }
                    if text_buf.is_empty() {
                        text_start = i;
                    }
                    text_buf.push(b'\\');
                    i += 1;
                    continue;
                }
                b'`' => {
                    if let Some((inline, next)) = self.parse_code_span(buffer, offsets, i, end) {
                        self.flush_text_buf(&mut out, offsets, &mut text_buf, &mut text_start, i);
                        out.push(inline);
                        i = next;
                        text_start = i;
                        continue;
                    }
                    let run_len = count_run(bytes, i, end, b'`');
                    if text_buf.is_empty() {
                        text_start = i;
                    }
                    text_buf.extend(std::iter::repeat_n(b'`', run_len));
                    i += run_len;
                    continue;
                }
                b'$' => {
                    if let Some((inline, next)) = self.parse_inline_math(buffer, offsets, i, end) {
                        self.flush_text_buf(&mut out, offsets, &mut text_buf, &mut text_start, i);
                        out.push(inline);
                        i = next;
                        text_start = i;
                        continue;
                    }
                }
                b'<' => {
                    if let Some((inline, next)) = self.parse_autolink(buffer, offsets, i, end) {
                        self.flush_text_buf(&mut out, offsets, &mut text_buf, &mut text_start, i);
                        out.push(inline);
                        i = next;
                        text_start = i;
                        continue;
                    }
                    if let Some((inline, next)) = self.parse_html_span(buffer, offsets, i, end) {
                        self.flush_text_buf(&mut out, offsets, &mut text_buf, &mut text_start, i);
                        out.push(inline);
                        i = next;
                        text_start = i;
                        continue;
                    }
                }
                b'&' => {
                    if let Some((decoded, next)) = decode_entity(bytes, i, end) {
                        if text_buf.is_empty() {
                            text_start = i;
                        }
                        text_buf.extend_from_slice(&decoded);
                        i = next;
                        continue;
                    }
                }
                b'@' => {
                    if let Some((inline, next)) =
                        self.parse_reference_inline(buffer, offsets, i, end)
                    {
                        self.flush_text_buf(&mut out, offsets, &mut text_buf, &mut text_start, i);
                        out.push(inline);
                        i = next;
                        text_start = i;
                        continue;
                    }
                }
                b'!' => {
                    if i + 1 < end && bytes[i + 1] == b'[' {
                        self.flush_text_buf(&mut out, offsets, &mut text_buf, &mut text_start, i);
                        self.push_text_node(&mut out, offsets, i, i + 2, "![");
                        let node_index = out.len().saturating_sub(1);
                        brackets.push(BracketEntry {
                            node_index,
                            start: i,
                            image: true,
                            active: true,
                        });
                        i += 2;
                        text_start = i;
                        continue;
                    }
                }
                b'[' => {
                    self.flush_text_buf(&mut out, offsets, &mut text_buf, &mut text_start, i);
                    self.push_text_node(&mut out, offsets, i, i + 1, "[");
                    let node_index = out.len().saturating_sub(1);
                    brackets.push(BracketEntry {
                        node_index,
                        start: i,
                        image: false,
                        active: true,
                    });
                    i += 1;
                    text_start = i;
                    continue;
                }
                b']' => {
                    self.flush_text_buf(&mut out, offsets, &mut text_buf, &mut text_start, i);
                    if let Some(next) = self.try_close_link(
                        buffer,
                        offsets,
                        end,
                        i,
                        &mut out,
                        &mut delims,
                        &mut brackets,
                    ) {
                        i = next;
                        text_start = i;
                        continue;
                    }
                    if text_buf.is_empty() {
                        text_start = i;
                    }
                    text_buf.push(b']');
                    i += 1;
                    continue;
                }
                b'*' | b'_' | b'~' => {
                    let run_len = count_run(bytes, i, end, b);
                    if b == b'~' && run_len < 2 {
                        if text_buf.is_empty() {
                            text_start = i;
                        }
                        text_buf.push(b'~');
                        i += 1;
                        continue;
                    }
                    let (can_open, can_close) =
                        delimiter_properties(buffer, start, end, i, run_len, b);
                    self.flush_text_buf(&mut out, offsets, &mut text_buf, &mut text_start, i);
                    let text = std::iter::repeat_n(b as char, run_len).collect::<String>();
                    let span = self.span_from_offsets(offsets, i, i + run_len);
                    out.push(Inline {
                        span,
                        kind: InlineKind::Text(text),
                    });
                    if can_open || can_close {
                        delims.push(Delimiter {
                            ch: b,
                            len: run_len,
                            node_index: out.len().saturating_sub(1),
                            can_open,
                            can_close,
                            orig_can_open: can_open,
                            orig_can_close: can_close,
                        });
                    }
                    i += run_len;
                    text_start = i;
                    continue;
                }
                b'\n' => {
                    let trailing = text_buf
                        .iter()
                        .rev()
                        .take_while(|byte| **byte == b' ')
                        .count();
                    let hard_break = trailing >= 2;
                    if trailing > 0 {
                        for _ in 0..trailing {
                            text_buf.pop();
                        }
                    }
                    self.flush_text_buf(&mut out, offsets, &mut text_buf, &mut text_start, i);
                    let span = self.span_from_offsets(offsets, i, i + 1);
                    out.push(Inline {
                        span,
                        kind: if hard_break {
                            InlineKind::HardBreak
                        } else {
                            InlineKind::SoftBreak
                        },
                    });
                    i += 1;
                    text_start = i;
                    continue;
                }
                _ => {}
            }
            if text_buf.is_empty() {
                text_start = i;
            }
            text_buf.push(b);
            i += 1;
        }

        self.flush_text_buf(&mut out, offsets, &mut text_buf, &mut text_start, end);
        self.process_emphasis(&mut out, &mut delims);
        autolink_inlines(&mut out);
        out
    }

    fn flush_text_buf(
        &self,
        out: &mut InlineSeq,
        offsets: &[usize],
        text_buf: &mut Vec<u8>,
        text_start: &mut usize,
        current: usize,
    ) {
        if text_buf.is_empty() {
            *text_start = current;
            return;
        }
        let span = self.span_from_offsets(offsets, *text_start, current);
        let bytes = std::mem::take(text_buf);
        let text = match String::from_utf8(bytes) {
            Ok(value) => value,
            Err(err) => String::from_utf8_lossy(&err.into_bytes()).to_string(),
        };
        out.push(Inline {
            span,
            kind: InlineKind::Text(text),
        });
        *text_start = current;
    }

    fn push_text_node(
        &self,
        out: &mut InlineSeq,
        offsets: &[usize],
        start: usize,
        end: usize,
        text: &str,
    ) {
        let span = self.span_from_offsets(offsets, start, end);
        out.push(Inline {
            span,
            kind: InlineKind::Text(text.to_string()),
        });
    }

    fn parse_code_span(
        &self,
        buffer: &str,
        offsets: &[usize],
        start: usize,
        end: usize,
    ) -> Option<(Inline, usize)> {
        let bytes = buffer.as_bytes();
        let run_len = count_run(bytes, start, end, b'`');
        let mut i = start + run_len;
        while i < end {
            if bytes[i] == b'`' {
                let close_len = count_run(bytes, i, end, b'`');
                if close_len == run_len {
                    let mut content = buffer[start + run_len..i].replace('\n', " ");
                    if content.starts_with(' ') && content.ends_with(' ') && content.len() >= 2 {
                        let has_non_space = content.bytes().any(|b| b != b' ');
                        if has_non_space {
                            content = content[1..content.len() - 1].to_string();
                        }
                    }
                    let span = self.span_from_offsets(offsets, start, i + run_len);
                    return Some((
                        Inline {
                            span,
                            kind: InlineKind::CodeSpan(content),
                        },
                        i + run_len,
                    ));
                }
                i += close_len;
                continue;
            }
            i += 1;
        }
        None
    }

    fn parse_inline_math(
        &mut self,
        buffer: &str,
        offsets: &[usize],
        start: usize,
        end: usize,
    ) -> Option<(Inline, usize)> {
        let bytes = buffer.as_bytes();
        let mut i = start + 1;
        while i < end {
            let b = bytes[i];
            if b == b'\\' {
                i += 2;
                continue;
            }
            if b == b'$' {
                let has_newline = bytes[start + 1..i].contains(&b'\n');
                if has_newline {
                    let span = self.span_from_offsets(offsets, start, i + 1);
                    self.push_diag(
                        span,
                        DiagnosticSeverity::Error,
                        E_MATH_INLINE_NL,
                        "newline in inline math",
                    );
                }
                let content = buffer[start + 1..i].to_string();
                let span = self.span_from_offsets(offsets, start, i + 1);
                return Some((
                    Inline {
                        span,
                        kind: InlineKind::MathInline { typst_src: content },
                    },
                    i + 1,
                ));
            }
            i += 1;
        }
        None
    }

    fn parse_autolink(
        &self,
        buffer: &str,
        offsets: &[usize],
        start: usize,
        end: usize,
    ) -> Option<(Inline, usize)> {
        let bytes = buffer.as_bytes();
        if start + 2 >= end {
            return None;
        }
        let mut i = start + 1;
        while i < end {
            let b = bytes[i];
            if b == b'>' {
                break;
            }
            if b.is_ascii_whitespace() || b == b'<' {
                return None;
            }
            i += 1;
        }
        if i >= end || bytes[i] != b'>' {
            return None;
        }
        let inner = &buffer[start + 1..i];
        let (url, display) = if is_autolink_scheme(inner) {
            (percent_encode_autolink_url(inner), inner.to_string())
        } else if is_autolink_email(inner) {
            (format!("mailto:{}", inner), inner.to_string())
        } else {
            return None;
        };

        let span = self.span_from_offsets(offsets, start, i + 1);
        let child_span = self.span_from_offsets(offsets, start + 1, i);
        let child = Inline {
            span: child_span,
            kind: InlineKind::Text(display),
        };
        Some((
            Inline {
                span,
                kind: InlineKind::Link {
                    url,
                    title: None,
                    children: vec![child],
                },
            },
            i + 1,
        ))
    }

    fn parse_html_span(
        &self,
        buffer: &str,
        offsets: &[usize],
        start: usize,
        end: usize,
    ) -> Option<(Inline, usize)> {
        let bytes = buffer.as_bytes();
        if start + 1 >= end {
            return None;
        }
        if bytes[start + 1] == b'!' {
            if start + 3 < end && bytes[start + 2] == b'-' && bytes[start + 3] == b'-' {
                if start + 4 < end && bytes[start + 4] == b'>' {
                    let raw = buffer[start..start + 5].to_string();
                    let span = self.span_from_offsets(offsets, start, start + 5);
                    return Some((
                        Inline {
                            span,
                            kind: InlineKind::HtmlSpan { raw },
                        },
                        start + 5,
                    ));
                }
                if start + 5 < end && bytes[start + 4] == b'-' && bytes[start + 5] == b'>' {
                    let raw = buffer[start..start + 6].to_string();
                    let span = self.span_from_offsets(offsets, start, start + 6);
                    return Some((
                        Inline {
                            span,
                            kind: InlineKind::HtmlSpan { raw },
                        },
                        start + 6,
                    ));
                }
                let mut i = start + 4;
                while i + 2 < end {
                    if bytes[i] == b'-' && bytes[i + 1] == b'-' && bytes[i + 2] == b'>' {
                        let raw = buffer[start..i + 3].to_string();
                        let span = self.span_from_offsets(offsets, start, i + 3);
                        return Some((
                            Inline {
                                span,
                                kind: InlineKind::HtmlSpan { raw },
                            },
                            i + 3,
                        ));
                    }
                    i += 1;
                }
                return None;
            }
            if start + 8 < end
                && bytes[start + 2] == b'['
                && bytes[start + 3..start + 9] == *b"CDATA["
            {
                let mut i = start + 9;
                while i + 2 < end {
                    if bytes[i] == b']' && bytes[i + 1] == b']' && bytes[i + 2] == b'>' {
                        let raw = buffer[start..i + 3].to_string();
                        let span = self.span_from_offsets(offsets, start, i + 3);
                        return Some((
                            Inline {
                                span,
                                kind: InlineKind::HtmlSpan { raw },
                            },
                            i + 3,
                        ));
                    }
                    i += 1;
                }
                return None;
            }
            if start + 2 < end && bytes[start + 2].is_ascii_alphabetic() {
                let mut i = start + 2;
                while i < end {
                    if bytes[i] == b'>' {
                        let raw = buffer[start..i + 1].to_string();
                        let span = self.span_from_offsets(offsets, start, i + 1);
                        return Some((
                            Inline {
                                span,
                                kind: InlineKind::HtmlSpan { raw },
                            },
                            i + 1,
                        ));
                    }
                    i += 1;
                }
                return None;
            }
            return None;
        }
        if bytes[start + 1] == b'?' {
            let mut i = start + 2;
            while i + 1 < end {
                if bytes[i] == b'?' && bytes[i + 1] == b'>' {
                    let raw = buffer[start..i + 2].to_string();
                    let span = self.span_from_offsets(offsets, start, i + 2);
                    return Some((
                        Inline {
                            span,
                            kind: InlineKind::HtmlSpan { raw },
                        },
                        i + 2,
                    ));
                }
                i += 1;
            }
            return None;
        }

        let mut i = start + 1;
        let mut closing = false;
        if bytes[i] == b'/' {
            closing = true;
            i += 1;
        }
        if i >= end || !bytes[i].is_ascii_alphabetic() {
            return None;
        }
        let name_start = i;
        i += 1;
        while i < end {
            let b = bytes[i];
            if b.is_ascii_alphanumeric() || b == b'-' {
                i += 1;
                continue;
            }
            break;
        }
        if i == name_start || i >= end {
            return None;
        }
        if !bytes[i].is_ascii_whitespace()
            && bytes[i] != b'>'
            && !(bytes[i] == b'/' && i + 1 < end && bytes[i + 1] == b'>')
        {
            return None;
        }
        if closing {
            while i < end && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i < end && bytes[i] == b'>' {
                let raw = buffer[start..i + 1].to_string();
                let span = self.span_from_offsets(offsets, start, i + 1);
                return Some((
                    Inline {
                        span,
                        kind: InlineKind::HtmlSpan { raw },
                    },
                    i + 1,
                ));
            }
            return None;
        }
        loop {
            while i < end && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= end {
                return None;
            }
            if bytes[i] == b'>' {
                let raw = buffer[start..i + 1].to_string();
                let span = self.span_from_offsets(offsets, start, i + 1);
                return Some((
                    Inline {
                        span,
                        kind: InlineKind::HtmlSpan { raw },
                    },
                    i + 1,
                ));
            }
            if bytes[i] == b'/' && i + 1 < end && bytes[i + 1] == b'>' {
                let raw = buffer[start..i + 2].to_string();
                let span = self.span_from_offsets(offsets, start, i + 2);
                return Some((
                    Inline {
                        span,
                        kind: InlineKind::HtmlSpan { raw },
                    },
                    i + 2,
                ));
            }
            if !is_attr_name_start(bytes[i]) {
                return None;
            }
            i += 1;
            while i < end && is_attr_name_continue(bytes[i]) {
                i += 1;
            }
            let mut ws = i;
            while ws < end && bytes[ws].is_ascii_whitespace() {
                ws += 1;
            }
            if ws < end && bytes[ws] == b'=' {
                i = ws + 1;
                while i < end && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                if i >= end {
                    return None;
                }
                let quote = bytes[i];
                if quote == b'"' || quote == b'\'' {
                    i += 1;
                    while i < end && bytes[i] != quote {
                        i += 1;
                    }
                    if i >= end {
                        return None;
                    }
                    i += 1;
                } else {
                    let mut consumed = false;
                    while i < end {
                        let b = bytes[i];
                        if b.is_ascii_whitespace() || b == b'>' {
                            break;
                        }
                        if b == b'/' && i + 1 < end && bytes[i + 1] == b'>' {
                            break;
                        }
                        if matches!(b, b'"' | b'\'' | b'=' | b'<' | b'>' | b'`') {
                            return None;
                        }
                        consumed = true;
                        i += 1;
                    }
                    if !consumed {
                        return None;
                    }
                }
            }
            if i < end {
                let b = bytes[i];
                if !(b.is_ascii_whitespace()
                    || b == b'>'
                    || (b == b'/' && i + 1 < end && bytes[i + 1] == b'>'))
                {
                    return None;
                }
            }
        }
    }

    fn parse_reference_inline(
        &mut self,
        buffer: &str,
        offsets: &[usize],
        start: usize,
        end: usize,
    ) -> Option<(Inline, usize)> {
        if start > 0 {
            if let Some(prev) = buffer[..start].chars().next_back()
                && (prev.is_alphanumeric() || matches!(prev, '+' | '-' | '.' | '_'))
            {
                return None;
            }
            let last_ws = buffer[..start]
                .rfind(|ch: char| ch.is_whitespace())
                .map(|idx| idx + 1)
                .unwrap_or(0);
            let token = &buffer[last_ws..start];
            if token.contains('/') || token.contains('\\') {
                return None;
            }
        }
        let bytes = buffer.as_bytes();
        let (label, label_end) = parse_label(bytes, start + 1, end)?;
        let label_span = self.span_from_offsets(offsets, start + 1, label_end);
        let mut bracket = None;
        let mut next = label_end;
        if label_end < end
            && bytes[label_end] == b'['
            && let Some((close, had_newline)) = find_bracket_end(bytes, label_end + 1, end)
        {
            let content_start = label_end + 1;
            let content_end = close;
            let (content, bracket_newline) =
                self.parse_bracket_inlines(buffer, offsets, content_start, content_end);
            if had_newline || bracket_newline {
                let span = self.span_from_offsets(offsets, start, close + 1);
                self.push_diag(
                    span,
                    DiagnosticSeverity::Error,
                    E_REF_BRACKET_NL,
                    "newline in reference text",
                );
            }
            bracket = Some(content);
            next = close + 1;
        }
        let span = self.span_from_offsets(offsets, start, next);
        Some((
            Inline {
                span,
                kind: InlineKind::Ref {
                    label: Label {
                        name: label,
                        span: label_span,
                    },
                    bracket,
                    resolved: None,
                },
            },
            next,
        ))
    }

    fn parse_bracket_inlines(
        &mut self,
        buffer: &str,
        offsets: &[usize],
        start: usize,
        end: usize,
    ) -> (InlineSeq, bool) {
        let bytes = buffer.as_bytes();
        let had_newline = bytes
            .get(start..end)
            .map(|slice| slice.contains(&b'\n'))
            .unwrap_or(false);
        let inlines = self.parse_inline_range(buffer, offsets, start, end);
        (inlines, had_newline)
    }

    #[allow(clippy::too_many_arguments)]
    fn try_close_link(
        &mut self,
        buffer: &str,
        offsets: &[usize],
        end: usize,
        current: usize,
        out: &mut InlineSeq,
        delims: &mut Vec<Delimiter>,
        brackets: &mut Vec<BracketEntry>,
    ) -> Option<usize> {
        let opener_pos = brackets.iter().rposition(|entry| entry.active)?;
        let opener = brackets.get(opener_pos)?.clone();
        if opener.image
            && let Some(inactive_pos) = brackets
                .iter()
                .rposition(|entry| !entry.active && !entry.image)
            && inactive_pos > opener_pos
        {
            brackets.remove(inactive_pos);
            return None;
        }
        enum ParsedLink {
            Inline {
                url: String,
                title: Option<String>,
                close: usize,
            },
            Reference {
                label: String,
                meta: LinkRefMeta,
                close: usize,
            },
        }
        let parsed = if let Some((inline_url, inline_title, inline_close)) =
            parse_inline_link_destination(buffer, current + 1, end)
        {
            ParsedLink::Inline {
                url: inline_url,
                title: inline_title,
                close: inline_close,
            }
        } else {
            let bytes = buffer.as_bytes();
            let mut next = current + 1;
            let mut label = None;
            let mut label_open_span = None;
            let mut label_span = None;
            let mut label_close_span = None;

            if next < end && bytes[next] == b'[' {
                let label_start = next + 1;
                if let Some((label_end, had_newline)) = find_bracket_end(bytes, label_start, end) {
                    if had_newline {
                        return None;
                    }
                    label_open_span = Some(self.span_from_offsets(offsets, next, next + 1));
                    label_close_span =
                        Some(self.span_from_offsets(offsets, label_end, label_end + 1));
                    if label_end > label_start {
                        label_span = Some(self.span_from_offsets(offsets, label_start, label_end));
                    }
                    let raw = &bytes[label_start..label_end];
                    let raw_label = String::from_utf8_lossy(raw).to_string();
                    if !raw_label.is_empty() {
                        label = Some(raw_label);
                    }
                    next = label_end + 1;
                } else {
                    return None;
                }
            }

            let content_start = if opener.image {
                opener.start + 2
            } else {
                opener.start + 1
            };
            let text_label = if current >= content_start {
                String::from_utf8_lossy(&bytes[content_start..current]).to_string()
            } else {
                String::new()
            };
            let lookup = match label {
                Some(value) if !value.is_empty() => value,
                _ => text_label,
            };
            if lookup.is_empty() {
                return None;
            }
            let normalized_lookup = normalize_link_label(lookup.as_bytes());
            if !self.link_defs.contains_key(&normalized_lookup) {
                brackets.remove(opener_pos);
                return None;
            }

            let opener_len = if opener.image { 2 } else { 1 };
            let meta = LinkRefMeta {
                opener_span: self.span_from_offsets(
                    offsets,
                    opener.start,
                    opener.start + opener_len,
                ),
                closer_span: self.span_from_offsets(offsets, current, current + 1),
                label_open_span,
                label_span,
                label_close_span,
            };
            let close = next.saturating_sub(1);
            ParsedLink::Reference {
                label: lookup,
                meta,
                close,
            }
        };
        if opener.node_index >= out.len() {
            return None;
        }
        let close = match parsed {
            ParsedLink::Inline { close, .. } => close,
            ParsedLink::Reference { close, .. } => close,
        };
        let span = self.span_from_offsets(offsets, opener.start, close + 1);

        let mut children = out.split_off(opener.node_index + 1);
        out.pop()?;

        let mut child_delims = Vec::new();
        let mut remaining = Vec::new();
        for delim in delims.drain(..) {
            if delim.node_index > opener.node_index {
                let mut shifted = delim;
                shifted.node_index = shifted.node_index.saturating_sub(opener.node_index + 1);
                child_delims.push(shifted);
            } else {
                remaining.push(delim);
            }
        }
        *delims = remaining;
        if !child_delims.is_empty() {
            self.process_emphasis(&mut children, &mut child_delims);
        }

        let kind = match parsed {
            ParsedLink::Inline { url, title, .. } => {
                if opener.image {
                    InlineKind::Image {
                        url,
                        title,
                        alt: children,
                    }
                } else {
                    InlineKind::Link {
                        url,
                        title,
                        children,
                    }
                }
            }
            ParsedLink::Reference { label, meta, .. } => {
                if opener.image {
                    InlineKind::ImageRef {
                        label,
                        alt: children,
                        meta,
                    }
                } else {
                    InlineKind::LinkRef {
                        label,
                        children,
                        meta,
                    }
                }
            }
        };
        out.push(Inline { span, kind });

        if !opener.image {
            for entry in brackets.iter_mut() {
                if !entry.image {
                    entry.active = false;
                }
            }
        }
        brackets.retain(|entry| entry.node_index < opener.node_index);
        Some(close + 1)
    }

    fn process_emphasis(&self, out: &mut InlineSeq, delims: &mut Vec<Delimiter>) {
        loop {
            let mut closer_index = None;
            for (idx, delim) in delims.iter().enumerate() {
                if delim.can_close {
                    closer_index = Some(idx);
                    break;
                }
            }
            let closer_index = match closer_index {
                Some(idx) => idx,
                None => break,
            };
            let closer = match delims.get(closer_index) {
                Some(entry) => entry.clone(),
                None => break,
            };
            let mut opener_index = None;
            let mut use_len = 1;
            for idx in (0..closer_index).rev() {
                let opener = match delims.get(idx) {
                    Some(entry) => entry,
                    None => continue,
                };
                if opener.ch != closer.ch || !opener.can_open {
                    continue;
                }
                let candidate = if opener.ch == b'~' {
                    // GFM strikethrough follows emphasis-like nesting rules.
                    if opener.len >= 2 && closer.len >= 2 {
                        2
                    } else {
                        continue;
                    }
                } else if opener.len >= 2 && closer.len >= 2 {
                    2
                } else {
                    1
                };
                if opener.ch != b'~' && candidate == 1 && delimiter_blocked(opener, &closer) {
                    continue;
                }
                opener_index = Some(idx);
                use_len = candidate;
                break;
            }
            let opener_index = match opener_index {
                Some(idx) => idx,
                None => {
                    if let Some(entry) = delims.get_mut(closer_index) {
                        entry.can_close = false;
                    }
                    continue;
                }
            };
            self.apply_emphasis(out, delims, opener_index, closer_index, use_len);
        }
    }

    fn apply_emphasis(
        &self,
        out: &mut InlineSeq,
        delims: &mut Vec<Delimiter>,
        opener_index: usize,
        closer_index: usize,
        use_len: usize,
    ) {
        let opener = match delims.get(opener_index) {
            Some(entry) => entry.clone(),
            None => return,
        };
        let closer = match delims.get(closer_index) {
            Some(entry) => entry.clone(),
            None => return,
        };
        if opener.node_index >= closer.node_index {
            return;
        }
        let removed_len = closer.node_index + 1 - opener.node_index;
        let removed: Vec<Inline> = out
            .drain(opener.node_index..closer.node_index + 1)
            .collect();
        let mut iter = removed.into_iter();
        let opener_node = match iter.next() {
            Some(node) => node,
            None => return,
        };
        let closer_node = match iter.next_back() {
            Some(node) => node,
            None => return,
        };
        let children: Vec<Inline> = iter.collect();

        let opener_remain = opener.len.saturating_sub(use_len);
        let closer_remain = closer.len.saturating_sub(use_len);
        let mut replacement = Vec::new();
        // Remaining opener text.
        if opener_remain > 0 {
            let span = Span {
                start: opener_node.span.start,
                end: opener_node.span.start + opener_remain,
            };
            let text = std::iter::repeat_n(opener.ch as char, opener_remain).collect::<String>();
            replacement.push(Inline {
                span,
                kind: InlineKind::Text(text),
            });
        }

        // Emphasis node.
        let emph_span = Span {
            start: opener_node.span.start + opener_remain,
            end: closer_node.span.end.saturating_sub(closer_remain),
        };
        let emph_kind = if opener.ch == b'~' {
            InlineKind::Strikethrough(children)
        } else if use_len == 2 {
            InlineKind::Strong(children)
        } else {
            InlineKind::Emph(children)
        };
        replacement.push(Inline {
            span: emph_span,
            kind: emph_kind,
        });

        // Remaining closer text.
        if closer_remain > 0 {
            let span = Span {
                start: closer_node.span.end.saturating_sub(closer_remain),
                end: closer_node.span.end,
            };
            let text = std::iter::repeat_n(closer.ch as char, closer_remain).collect::<String>();
            replacement.push(Inline {
                span,
                kind: InlineKind::Text(text),
            });
        }

        let replacement_len = replacement.len();
        out.splice(opener.node_index..opener.node_index, replacement);

        let delta = replacement_len as isize - removed_len as isize;
        let mut updated = Vec::new();
        for (idx, delim) in delims.iter().enumerate() {
            if idx == opener_index || idx == closer_index {
                continue;
            }
            if delim.node_index < opener.node_index {
                updated.push(delim.clone());
            } else if delim.node_index > closer.node_index {
                let mut shifted = delim.clone();
                if delta.is_negative() {
                    shifted.node_index = shifted.node_index.saturating_sub(delta.unsigned_abs());
                } else {
                    shifted.node_index = shifted.node_index.saturating_add(delta.unsigned_abs());
                }
                updated.push(shifted);
            }
        }

        let mut next_index = opener.node_index;
        if opener_remain > 0 {
            updated.push(Delimiter {
                ch: opener.ch,
                len: opener_remain,
                node_index: next_index,
                can_open: opener.can_open,
                can_close: opener.can_close,
                orig_can_open: opener.orig_can_open,
                orig_can_close: opener.orig_can_close,
            });
            next_index += 1;
        }
        next_index += 1;
        if closer_remain > 0 {
            updated.push(Delimiter {
                ch: closer.ch,
                len: closer_remain,
                node_index: next_index,
                can_open: closer.can_open,
                can_close: closer.can_close,
                orig_can_open: closer.orig_can_open,
                orig_can_close: closer.orig_can_close,
            });
        }
        updated.sort_by_key(|delim| delim.node_index);
        *delims = updated;
    }

    fn span_from_offsets(&self, offsets: &[usize], start: usize, end: usize) -> Span {
        let source_end = self.source.len();
        let start_off = offsets.get(start).copied().unwrap_or(source_end);
        let mut end_off = if end < offsets.len() {
            offsets[end]
        } else if let Some(last) = offsets.last() {
            last.saturating_add(1)
        } else {
            source_end
        };

        if end_off > source_end {
            end_off = source_end;
        }

        Span {
            start: start_off,
            end: end_off,
        }
    }

    fn try_parse_target_line(&mut self, line: &Line) -> Option<AttrList> {
        if !self.is_target_line_text(&line.text) {
            return None;
        }
        let trimmed = line.text.trim();
        let open_idx = line.text.find('{').unwrap_or(0);
        let close_idx = line
            .text
            .rfind('}')
            .unwrap_or(line.text.len().saturating_sub(1));
        if trimmed.len() < 2 || !trimmed.starts_with('{') || !trimmed.ends_with('}') {
            self.push_diag(
                Span {
                    start: line.start,
                    end: line.end,
                },
                DiagnosticSeverity::Error,
                E_ATTR_SYNTAX,
                "invalid attribute list",
            );
            return Some(AttrList::default());
        }
        let base_offset = line.start + open_idx;
        Some(self.parse_attr_list_text(&line.text[open_idx..=close_idx], base_offset))
    }

    fn is_target_line_text(&self, text: &str) -> bool {
        let trimmed = text.trim();
        trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 2
    }

    fn parse_fence_info(
        &mut self,
        line: &Line,
        _fence_len: usize,
        info: String,
    ) -> (Option<String>, AttrList) {
        if let Some(brace_idx) = info.find('{') {
            let lang_part = info[..brace_idx].trim();
            let lang = if lang_part.is_empty() {
                None
            } else {
                Some(lang_part.to_string())
            };
            let open_idx = line.text.find('{').unwrap_or(line.text.len());
            let close_idx = line
                .text
                .rfind('}')
                .unwrap_or(line.text.len().saturating_sub(1));
            let base_offset = line.start + open_idx;
            let attrs = self.parse_attr_list_text(&line.text[open_idx..=close_idx], base_offset);
            (lang, attrs)
        } else {
            let lang = if info.is_empty() { None } else { Some(info) };
            (lang, AttrList::default())
        }
    }

    fn parse_attr_list_text(&mut self, text: &str, base_offset: usize) -> AttrList {
        let mut attrs = AttrList::default();
        let span = Span {
            start: base_offset,
            end: base_offset + text.len(),
        };
        attrs.span = Some(span);
        let inner = text.trim();
        if !inner.starts_with('{') || !inner.ends_with('}') {
            self.push_diag(
                span,
                DiagnosticSeverity::Error,
                E_ATTR_SYNTAX,
                "invalid attribute list",
            );
            return attrs;
        }
        let inner = &inner[1..inner.len().saturating_sub(1)];
        let mut tokens = Vec::new();
        let mut in_quotes = false;
        let mut token_start = None;
        for (idx, ch) in inner.char_indices() {
            if ch == '"' {
                in_quotes = !in_quotes;
            }
            if ch.is_whitespace() && !in_quotes {
                if let Some(start) = token_start {
                    tokens.push((start, idx));
                    token_start = None;
                }
            } else if token_start.is_none() {
                token_start = Some(idx);
            }
        }
        if let Some(start) = token_start {
            tokens.push((start, inner.len()));
        }

        for (start, end) in tokens {
            let token = &inner[start..end];
            if let Some(name) = token.strip_prefix('#') {
                if attrs.label.is_some() {
                    let span = Span {
                        start: base_offset + 1 + start,
                        end: base_offset + 1 + end,
                    };
                    self.push_diag(
                        span,
                        DiagnosticSeverity::Error,
                        E_ATTR_SYNTAX,
                        "duplicate label",
                    );
                    continue;
                }
                let name = name.to_string();
                if name.is_empty() || !is_valid_label(&name) {
                    let span = Span {
                        start: base_offset + 1 + start,
                        end: base_offset + 1 + end,
                    };
                    self.push_diag(
                        span,
                        DiagnosticSeverity::Error,
                        E_ATTR_SYNTAX,
                        "invalid label syntax",
                    );
                    continue;
                }
                let span = Span {
                    start: base_offset + 1 + start + 1,
                    end: base_offset + 1 + end,
                };
                attrs.label = Some(Label { name, span });
                continue;
            }
            let mut iter = token.splitn(2, '=');
            let key = iter.next().unwrap_or("");
            let value = iter.next();
            if key.is_empty() || value.is_none() {
                let span = Span {
                    start: base_offset + 1 + start,
                    end: base_offset + 1 + end,
                };
                self.push_diag(
                    span,
                    DiagnosticSeverity::Error,
                    E_ATTR_SYNTAX,
                    "invalid attribute item",
                );
                continue;
            }
            let value = value.unwrap_or("");
            let (raw, quoted, value_span) =
                if value.starts_with('"') && value.ends_with('"') && value.len() >= 2 {
                    let unquoted = &value[1..value.len() - 1];
                    (
                        unquoted.to_string(),
                        true,
                        Span {
                            start: base_offset + 1 + start + key.len() + 2,
                            end: base_offset + 1 + end - 1,
                        },
                    )
                } else {
                    (
                        value.to_string(),
                        false,
                        Span {
                            start: base_offset + 1 + start + key.len() + 1,
                            end: base_offset + 1 + end,
                        },
                    )
                };
            attrs.items.push(AttrItem {
                key: key.to_string(),
                value: AttrValue {
                    raw,
                    span: value_span,
                    quoted,
                },
            });
        }

        attrs
    }

    fn validate_box_styles(&mut self, attrs: &AttrList) {
        for item in &attrs.items {
            let value = item.value.raw.trim();
            let invalid = match item.key.as_str() {
                "bg" | "title-bg" | "border-color" => !is_hex_color(value),
                "border-style" => !is_border_style(value),
                "border-width" => !is_border_width(value),
                _ => false,
            };
            if invalid {
                self.push_diag(
                    item.value.span,
                    DiagnosticSeverity::Warning,
                    W_BOX_STYLE_INVALID,
                    "invalid box style value",
                );
            }
        }
    }

    fn parse_code_meta(
        &mut self,
        attrs: &AttrList,
        code_text: &str,
        line_start: usize,
        line_end: usize,
    ) -> CodeMeta {
        // Line metadata is parsed before overlap validation and OOB warnings.
        let total_lines = count_lines(code_text);
        let mut meta = CodeMeta {
            hl: Vec::new(),
            diff_add: Vec::new(),
            diff_del: Vec::new(),
            line_labels: Vec::new(),
        };
        for item in &attrs.items {
            match item.key.as_str() {
                "hl" => {
                    let (ranges, labels, oob) = self.parse_line_ranges(item, total_lines, true);
                    meta.hl = ranges;
                    meta.line_labels = labels;
                    if oob {
                        self.push_diag(
                            item.value.span,
                            DiagnosticSeverity::Warning,
                            W_CODE_RANGE_OOB,
                            "code line range out of bounds",
                        );
                    }
                }
                "diff_add" => {
                    let (ranges, _, oob) = self.parse_line_ranges(item, total_lines, false);
                    meta.diff_add = ranges;
                    if oob {
                        self.push_diag(
                            item.value.span,
                            DiagnosticSeverity::Warning,
                            W_CODE_RANGE_OOB,
                            "code line range out of bounds",
                        );
                    }
                }
                "diff_del" => {
                    let (ranges, _, oob) = self.parse_line_ranges(item, total_lines, false);
                    meta.diff_del = ranges;
                    if oob {
                        self.push_diag(
                            item.value.span,
                            DiagnosticSeverity::Warning,
                            W_CODE_RANGE_OOB,
                            "code line range out of bounds",
                        );
                    }
                }
                _ => {}
            }
        }

        if self.ranges_overlap(&meta.hl, &meta.diff_add)
            || self.ranges_overlap(&meta.hl, &meta.diff_del)
            || self.ranges_overlap(&meta.diff_add, &meta.diff_del)
        {
            let span = attrs.span.unwrap_or(Span {
                start: line_start,
                end: line_end,
            });
            self.push_diag(
                span,
                DiagnosticSeverity::Error,
                E_CODE_CONFLICT,
                "code line meta conflicts",
            );
        }

        self.clamp_ranges(&mut meta.hl, total_lines);
        self.clamp_ranges(&mut meta.diff_add, total_lines);
        self.clamp_ranges(&mut meta.diff_del, total_lines);

        meta
    }

    fn parse_line_ranges(
        &mut self,
        item: &AttrItem,
        max_lines: u32,
        allow_labels: bool,
    ) -> (Vec<LineRange>, Vec<LineLabel>, bool) {
        let mut ranges = Vec::new();
        let mut labels = Vec::new();
        let mut out_of_bounds = false;
        for entry in item.value.raw.split(',') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }
            if !allow_labels && entry.contains(':') {
                self.push_diag(
                    item.value.span,
                    DiagnosticSeverity::Error,
                    E_ATTR_SYNTAX,
                    "unexpected label in code range",
                );
                continue;
            }
            if allow_labels && let Some((range_part, label_part)) = entry.split_once(':') {
                if let Ok(line) = range_part.parse::<u32>() {
                    if line == 0 {
                        self.push_diag(
                            item.value.span,
                            DiagnosticSeverity::Error,
                            E_ATTR_SYNTAX,
                            "invalid line number",
                        );
                        continue;
                    }
                    if max_lines == 0 || line > max_lines {
                        out_of_bounds = true;
                        continue;
                    }
                    if !is_valid_label(label_part) {
                        self.push_diag(
                            item.value.span,
                            DiagnosticSeverity::Error,
                            E_ATTR_SYNTAX,
                            "invalid label syntax",
                        );
                        continue;
                    }
                    ranges.push(LineRange {
                        start: line,
                        end: line,
                    });
                    labels.push(LineLabel {
                        line,
                        label: Label {
                            name: label_part.to_string(),
                            span: item.value.span,
                        },
                    });
                } else {
                    self.push_diag(
                        item.value.span,
                        DiagnosticSeverity::Error,
                        E_ATTR_SYNTAX,
                        "invalid line range",
                    );
                }
                continue;
            }
            if let Some((start, end)) = entry.split_once('-') {
                if let (Ok(start), Ok(end)) = (start.parse::<u32>(), end.parse::<u32>()) {
                    if start == 0 || end == 0 || end < start {
                        self.push_diag(
                            item.value.span,
                            DiagnosticSeverity::Error,
                            E_ATTR_SYNTAX,
                            "invalid line range",
                        );
                        continue;
                    }
                    if max_lines == 0 || start > max_lines || end > max_lines {
                        out_of_bounds = true;
                        continue;
                    }
                    ranges.push(LineRange { start, end });
                } else {
                    self.push_diag(
                        item.value.span,
                        DiagnosticSeverity::Error,
                        E_ATTR_SYNTAX,
                        "invalid line range",
                    );
                }
                continue;
            }
            if let Ok(line) = entry.parse::<u32>() {
                if line == 0 {
                    self.push_diag(
                        item.value.span,
                        DiagnosticSeverity::Error,
                        E_ATTR_SYNTAX,
                        "invalid line number",
                    );
                    continue;
                }
                if max_lines == 0 || line > max_lines {
                    out_of_bounds = true;
                    continue;
                }
                ranges.push(LineRange {
                    start: line,
                    end: line,
                });
            } else {
                self.push_diag(
                    item.value.span,
                    DiagnosticSeverity::Error,
                    E_ATTR_SYNTAX,
                    "invalid line range",
                );
            }
        }
        (ranges, labels, out_of_bounds)
    }

    fn ranges_overlap(&self, left: &[LineRange], right: &[LineRange]) -> bool {
        for a in left {
            for b in right {
                if a.start <= b.end && b.start <= a.end {
                    return true;
                }
            }
        }
        false
    }

    fn clamp_ranges(&self, ranges: &mut Vec<LineRange>, max_lines: u32) {
        ranges.retain(|range| {
            range.start >= 1
                && range.end >= range.start
                && range.start <= max_lines
                && range.end <= max_lines
        });
    }

    fn push_diag(
        &mut self,
        span: Span,
        severity: DiagnosticSeverity,
        code: &'static str,
        message: &str,
    ) {
        let range = self.source_map.range(span);
        self.diagnostics
            .push(Diagnostic::new(range, severity, code, message));
    }

    fn is_heading_line(&self, text: &str) -> bool {
        parse_atx_heading(text).is_some()
    }

    fn is_box_open(&self, text: &str) -> bool {
        if !text.starts_with(":::") {
            return false;
        }
        let fence_len = text.chars().take_while(|c| *c == ':').count();
        let rest = text[fence_len..].trim_start();
        rest.starts_with("box")
    }

    fn is_code_fence_line(&self, text: &str) -> bool {
        parse_fence_open(text).is_some()
    }
}

fn split_lines(source: &str) -> Vec<Line> {
    let mut lines = Vec::new();
    let mut start = 0;
    for (idx, byte) in source.bytes().enumerate() {
        if byte == b'\n' {
            let text = source[start..idx].to_string();
            lines.push(Line {
                text,
                start,
                end: idx,
                has_newline: true,
                lazy_continuation: false,
            });
            start = idx + 1;
        }
    }
    if start <= source.len() {
        let text = source[start..].to_string();
        lines.push(Line {
            text,
            start,
            end: source.len(),
            has_newline: false,
            lazy_continuation: false,
        });
    }
    lines
}

#[derive(Clone, Copy, Debug)]
enum HtmlBlockKind {
    Type1(&'static str),
    Type2,
    Type3,
    Type4,
    Type5,
    Type6,
    Type7,
}

struct HtmlTag<'a> {
    name: &'a str,
    after: usize,
    closing: bool,
}

const HTML_BLOCK_TAGS: &[&str] = &[
    "address",
    "article",
    "aside",
    "base",
    "basefont",
    "blockquote",
    "body",
    "caption",
    "center",
    "col",
    "colgroup",
    "dd",
    "details",
    "dialog",
    "dir",
    "div",
    "dl",
    "dt",
    "fieldset",
    "figcaption",
    "figure",
    "footer",
    "form",
    "frame",
    "frameset",
    "h1",
    "h2",
    "h3",
    "h4",
    "h5",
    "h6",
    "head",
    "header",
    "hr",
    "html",
    "iframe",
    "legend",
    "li",
    "link",
    "main",
    "menu",
    "menuitem",
    "nav",
    "noframes",
    "ol",
    "optgroup",
    "option",
    "p",
    "param",
    "search",
    "section",
    "source",
    "summary",
    "table",
    "tbody",
    "td",
    "tfoot",
    "th",
    "thead",
    "title",
    "tr",
    "track",
    "ul",
];

fn strip_indent_up_to(text: &str, max_cols: usize) -> Option<&str> {
    let bytes = text.as_bytes();
    let mut cols = 0;
    let mut idx = 0;
    for (pos, byte) in bytes.iter().enumerate() {
        let next_cols = match advance_column(cols, *byte) {
            Some(next) => next,
            None => {
                idx = pos;
                return Some(&text[idx..]);
            }
        };
        cols = next_cols;
        idx = pos + 1;
        if cols > max_cols {
            return None;
        }
    }
    Some(&text[idx..])
}

fn parse_fence_open(text: &str) -> Option<(usize, usize, u8, String)> {
    let bytes = text.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && idx < 3 && bytes[idx] == b' ' {
        idx += 1;
    }
    if idx < bytes.len() && bytes[idx] == b' ' {
        return None;
    }
    let rest = &text[idx..];
    let fence_char = if rest.starts_with("```") {
        b'`'
    } else if rest.starts_with("~~~") {
        b'~'
    } else {
        return None;
    };
    let fence_len = rest
        .as_bytes()
        .iter()
        .take_while(|b| **b == fence_char)
        .count();
    if fence_len < 3 {
        return None;
    }
    let info = rest[fence_len..].trim_matches(|ch| ch == ' ' || ch == '\t');
    if fence_char == b'`' && info.contains('`') {
        return None;
    }
    // Process backslash escapes and entities in info string
    let info_decoded = unescape_and_decode(info);
    Some((idx, fence_len, fence_char, info_decoded))
}

fn unescape_and_decode(text: &str) -> String {
    // Combined unescape + entity decode that respects backslash-escaping.
    //
    // Behavior:
    // - If a backslash precedes an ASCII punctuation character (CommonMark rule),
    //   the backslash is consumed and the punctuation is emitted literally.
    //   In particular, a backslash before '&' produces a literal '&' which
    //   will NOT be treated as the start of a character entity.
    // - Otherwise, an unescaped '&' is interpreted as the start of an entity
    //   and decode_entity is used to convert it to UTF-8 bytes.
    // - All other UTF-8 characters are copied through unchanged.
    let bytes = text.as_bytes();
    let mut result = String::new();
    let mut i = 0usize;
    while i < bytes.len() {
        // Backslash escapes
        if bytes[i] == b'\\' {
            if i + 1 < bytes.len() {
                let next = bytes[i + 1];
                if next.is_ascii_punctuation() {
                    // Emit the punctuation literally and skip the backslash.
                    result.push(next as char);
                    i += 2;
                    continue;
                } else {
                    // Not an escapable punctuation: keep the backslash verbatim.
                    result.push('\\');
                    i += 1;
                    continue;
                }
            } else {
                // Trailing backslash: keep it.
                result.push('\\');
                i += 1;
                continue;
            }
        }

        // Entities: only when unescaped '&' encountered
        if bytes[i] == b'&' {
            if let Some((decoded, next)) = decode_entity(bytes, i, bytes.len()) {
                // decoded is a Vec<u8> representing UTF-8 bytes
                let s = match std::str::from_utf8(&decoded) {
                    Ok(v) => v.to_string(),
                    Err(_) => String::from_utf8_lossy(&decoded).to_string(),
                };
                result.push_str(&s);
                i = next;
                continue;
            } else {
                // Not a valid entity, emit '&' literally.
                result.push('&');
                i += 1;
                continue;
            }
        }

        // Normal UTF-8 character
        if let Some(ch) = std::str::from_utf8(&bytes[i..])
            .ok()
            .and_then(|s| s.chars().next())
        {
            result.push(ch);
            i += ch.len_utf8();
        } else {
            // Invalid UTF-8 byte, skip it.
            i += 1;
        }
    }

    result
}

fn is_fence_close(text: &str, fence_len: usize, fence_char: u8) -> bool {
    let bytes = text.as_bytes();
    let mut idx = 0;
    while idx < bytes.len() && idx < 3 && bytes[idx] == b' ' {
        idx += 1;
    }
    if idx < bytes.len() && bytes[idx] == b' ' {
        return false;
    }
    let rest = &text[idx..];
    let rest_bytes = rest.as_bytes();
    let mut count = 0;
    while count < rest_bytes.len() && rest_bytes[count] == fence_char {
        count += 1;
    }
    if count < fence_len {
        return false;
    }
    rest_bytes[count..]
        .iter()
        .all(|b| *b == b' ' || *b == b'\t')
}

fn strip_leading_spaces(text: &str, max: usize) -> &str {
    if max == 0 {
        return text;
    }
    let bytes = text.as_bytes();
    let mut idx = 0;
    let mut count = 0;
    while idx < bytes.len() && count < max && bytes[idx] == b' ' {
        idx += 1;
        count += 1;
    }
    &text[idx..]
}

fn setext_underline_level(text: &str) -> Option<u8> {
    let trimmed = strip_indent_up_to(text, 3)?;
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let ch = bytes[0];
    if ch != b'=' && ch != b'-' {
        return None;
    }
    let mut i = 0;
    while i < bytes.len() && bytes[i] == ch {
        i += 1;
    }
    if i == 0 {
        return None;
    }
    if bytes[i..].iter().any(|b| *b != b' ' && *b != b'\t') {
        return None;
    }
    Some(if ch == b'=' { 1 } else { 2 })
}

fn parse_atx_heading(text: &str) -> Option<(u8, usize, usize)> {
    let trimmed = strip_indent_up_to(text, 3)?;
    let indent_len = text.len() - trimmed.len();
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut level = 0;
    while level < bytes.len() && bytes[level] == b'#' {
        level += 1;
    }
    if level == 0 || level > 6 {
        return None;
    }
    if level < bytes.len() && !is_space_or_tab(bytes[level]) {
        return None;
    }
    let mut content_start = level;
    while content_start < bytes.len() && is_space_or_tab(bytes[content_start]) {
        content_start += 1;
    }
    let mut content_end = bytes.len();
    while content_end > content_start && is_space_or_tab(bytes[content_end - 1]) {
        content_end -= 1;
    }
    if content_end > content_start {
        let mut hash_start = content_end;
        while hash_start > content_start && bytes[hash_start - 1] == b'#' {
            hash_start -= 1;
        }
        if hash_start < content_end
            && hash_start >= content_start
            && (hash_start == content_start || is_space_or_tab(bytes[hash_start - 1]))
        {
            let mut pre = hash_start;
            if hash_start > content_start {
                pre = hash_start - 1;
                while pre > content_start && is_space_or_tab(bytes[pre - 1]) {
                    pre -= 1;
                }
            }
            content_end = pre;
        }
    }
    while content_end > content_start && is_space_or_tab(bytes[content_end - 1]) {
        content_end -= 1;
    }
    Some((
        level as u8,
        indent_len + content_start,
        indent_len + content_end,
    ))
}

fn is_thematic_break_line(text: &str) -> bool {
    let trimmed = match strip_indent_up_to(text, 3) {
        Some(value) => value,
        None => return false,
    };
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() {
        return false;
    }
    let mut marker: Option<u8> = None;
    let mut count = 0;
    for b in bytes {
        if *b == b' ' || *b == b'\t' {
            continue;
        }
        if marker.is_none() {
            if !matches!(*b, b'-' | b'*' | b'_') {
                return false;
            }
            marker = Some(*b);
            count += 1;
            continue;
        }
        if marker == Some(*b) {
            count += 1;
            continue;
        }
        return false;
    }
    count >= 3
}

fn is_space_or_tab(byte: u8) -> bool {
    byte == b' ' || byte == b'\t'
}

/// Returns (prefix_bytes, partially_consumed_tab, remaining_tab_cols, current_col)
/// where prefix_bytes is the number of bytes to skip,
/// partially_consumed_tab indicates if a tab was partially consumed,
/// remaining_tab_cols is the number of columns left in that tab,
/// and current_col is the column position after the marker and optional space/tab.
fn blockquote_prefix_info(text: &str) -> Option<(usize, bool, usize, usize)> {
    let bytes = text.as_bytes();
    let mut idx = 0;
    let mut col = 0;

    // Up to 3 spaces of indentation before '>'
    let mut spaces = 0;
    while idx < bytes.len() && spaces < 3 && bytes[idx] == b' ' {
        idx += 1;
        col += 1;
        spaces += 1;
    }

    // Cannot have 4+ spaces (that would be code block)
    if idx < bytes.len() && bytes[idx] == b' ' {
        return None;
    }

    // Must have '>' marker
    if idx >= bytes.len() || bytes[idx] != b'>' {
        return None;
    }
    idx += 1;
    col += 1;

    // Optional: consume one space or tab after '>'
    // But if it's a tab, we might only partially consume it
    let mut partially_consumed_tab = false;
    let mut remaining_tab_cols = 0;

    if idx < bytes.len() {
        match bytes[idx] {
            b' ' => {
                idx += 1;
                col += 1;
            }
            b'\t' => {
                let chars_to_tab = 4 - (col % 4);
                if chars_to_tab > 1 {
                    // Partially consume the tab: advance 1 column, don't advance byte
                    partially_consumed_tab = true;
                    remaining_tab_cols = chars_to_tab - 1;
                    col += 1;
                } else {
                    // Consume the entire tab (only 1 column to next tab stop)
                    idx += 1;
                    col += 1;
                }
            }
            _ => {
                // No space/tab after '>', that's fine
            }
        }
    }

    Some((idx, partially_consumed_tab, remaining_tab_cols, col))
}

fn blockquote_prefix_len(text: &str) -> Option<usize> {
    blockquote_prefix_info(text).map(|(prefix_bytes, _, _, _)| prefix_bytes)
}

fn parse_html_tag_name(text: &str) -> Option<HtmlTag<'_>> {
    let bytes = text.as_bytes();
    if bytes.first()? != &b'<' {
        return None;
    }
    let mut idx = 1;
    let mut closing = false;
    if idx < bytes.len() && bytes[idx] == b'/' {
        closing = true;
        idx += 1;
    }
    if idx >= bytes.len() || !bytes[idx].is_ascii_alphabetic() {
        return None;
    }
    let start = idx;
    idx += 1;
    while idx < bytes.len() {
        let b = bytes[idx];
        if b.is_ascii_alphanumeric() || b == b'-' {
            idx += 1;
            continue;
        }
        break;
    }
    let name = &text[start..idx];
    Some(HtmlTag {
        name,
        after: idx,
        closing,
    })
}

fn is_html_tag_boundary(bytes: &[u8], idx: usize) -> bool {
    match bytes.get(idx) {
        None => true,
        Some(b) => b.is_ascii_whitespace() || *b == b'>' || *b == b'/',
    }
}

fn match_html_block_start(text: &str) -> Option<HtmlBlockKind> {
    let trimmed = strip_indent_up_to(text, 3)?;
    if trimmed.is_empty() {
        return None;
    }

    if let Some(tag) = match_html_type1(trimmed) {
        return Some(HtmlBlockKind::Type1(tag));
    }
    if trimmed.starts_with("<!--") {
        return Some(HtmlBlockKind::Type2);
    }
    if trimmed.starts_with("<?") {
        return Some(HtmlBlockKind::Type3);
    }
    if trimmed.starts_with("<![CDATA[") {
        return Some(HtmlBlockKind::Type5);
    }
    if trimmed.starts_with("<!") {
        let third = trimmed.as_bytes().get(2).copied();
        if matches!(third, Some(b) if b.is_ascii_alphabetic()) {
            return Some(HtmlBlockKind::Type4);
        }
    }
    if match_html_block_tag(trimmed) {
        return Some(HtmlBlockKind::Type6);
    }
    if match_html_any_tag(trimmed) {
        return Some(HtmlBlockKind::Type7);
    }
    None
}

fn match_html_type1(text: &str) -> Option<&'static str> {
    let tag = parse_html_tag_name(text)?;
    if tag.closing {
        return None;
    }
    if !is_html_tag_boundary(text.as_bytes(), tag.after) {
        return None;
    }
    type1_tag_name(tag.name)
}

fn type1_tag_name(name: &str) -> Option<&'static str> {
    if name.eq_ignore_ascii_case("pre") {
        Some("pre")
    } else if name.eq_ignore_ascii_case("script") {
        Some("script")
    } else if name.eq_ignore_ascii_case("style") {
        Some("style")
    } else if name.eq_ignore_ascii_case("textarea") {
        Some("textarea")
    } else {
        None
    }
}

fn is_type1_tag_name(name: &str) -> bool {
    type1_tag_name(name).is_some()
}

fn match_html_block_tag(text: &str) -> bool {
    let tag = match parse_html_tag_name(text) {
        Some(tag) => tag,
        None => return false,
    };
    if !is_html_tag_boundary(text.as_bytes(), tag.after) {
        return false;
    }
    HTML_BLOCK_TAGS
        .iter()
        .any(|name| tag.name.eq_ignore_ascii_case(name))
}

fn match_html_any_tag(text: &str) -> bool {
    let bytes = text.as_bytes();
    let end = match parse_html_tag_end(bytes) {
        Some(end) => end,
        None => return false,
    };
    if let Some(tag) = parse_html_tag_name(text)
        && is_type1_tag_name(tag.name)
    {
        return false;
    }
    for b in &bytes[end + 1..] {
        if *b != b' ' && *b != b'\t' {
            return false;
        }
    }
    true
}

fn parse_html_tag_end(bytes: &[u8]) -> Option<usize> {
    if bytes.len() < 2 || bytes[0] != b'<' {
        return None;
    }
    let mut i = 1;
    let mut closing = false;
    if bytes.get(i) == Some(&b'/') {
        closing = true;
        i += 1;
    }
    if i >= bytes.len() || !bytes[i].is_ascii_alphabetic() {
        return None;
    }
    i += 1;
    while i < bytes.len() {
        let b = bytes[i];
        if b.is_ascii_alphanumeric() || b == b'-' {
            i += 1;
            continue;
        }
        break;
    }
    if i >= bytes.len() {
        return None;
    }
    if !bytes[i].is_ascii_whitespace()
        && bytes[i] != b'>'
        && !(bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'>')
    {
        return None;
    }
    if closing {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        return if i < bytes.len() && bytes[i] == b'>' {
            Some(i)
        } else {
            None
        };
    }
    loop {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            return None;
        }
        if bytes[i] == b'>' {
            return Some(i);
        }
        if bytes[i] == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'>' {
            return Some(i + 1);
        }
        if !is_attr_name_start(bytes[i]) {
            return None;
        }
        i += 1;
        while i < bytes.len() && is_attr_name_continue(bytes[i]) {
            i += 1;
        }
        let after_name = i;
        let mut ws = i;
        while ws < bytes.len() && bytes[ws].is_ascii_whitespace() {
            ws += 1;
        }
        if ws < bytes.len() && bytes[ws] == b'=' {
            i = ws + 1;
            while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                i += 1;
            }
            if i >= bytes.len() {
                return None;
            }
            let quote = bytes[i];
            if quote == b'"' || quote == b'\'' {
                i += 1;
                while i < bytes.len() && bytes[i] != quote {
                    i += 1;
                }
                if i >= bytes.len() {
                    return None;
                }
                i += 1;
            } else {
                let mut consumed = false;
                while i < bytes.len() {
                    let b = bytes[i];
                    if b.is_ascii_whitespace() || b == b'>' {
                        break;
                    }
                    if b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'>' {
                        break;
                    }
                    if matches!(b, b'"' | b'\'' | b'=' | b'<' | b'>' | b'`') {
                        return None;
                    }
                    consumed = true;
                    i += 1;
                }
                if !consumed {
                    return None;
                }
            }
        } else {
            i = after_name;
        }
        if i < bytes.len() {
            let b = bytes[i];
            if !(b.is_ascii_whitespace()
                || b == b'>'
                || (b == b'/' && i + 1 < bytes.len() && bytes[i + 1] == b'>'))
            {
                return None;
            }
        }
    }
}

fn html_block_end(kind: HtmlBlockKind, line: &str) -> bool {
    match kind {
        HtmlBlockKind::Type1(tag) => contains_html_closing_tag(line, tag),
        HtmlBlockKind::Type2 => line.contains("-->"),
        HtmlBlockKind::Type3 => line.contains("?>"),
        HtmlBlockKind::Type4 => line.contains('>'),
        HtmlBlockKind::Type5 => line.contains("]]>"),
        HtmlBlockKind::Type6 | HtmlBlockKind::Type7 => false,
    }
}

fn table_line_view(text: &str) -> Option<(usize, &str)> {
    let bytes = text.as_bytes();
    let mut idx = 0;
    let mut spaces = 0;
    while idx < bytes.len() && spaces < 3 && bytes[idx] == b' ' {
        idx += 1;
        spaces += 1;
    }
    if idx < bytes.len() && bytes[idx] == b' ' {
        return None;
    }
    Some((idx, &text[idx..]))
}

fn split_table_cells(text: &str, base_offset: usize) -> (Vec<TableCellRaw>, bool) {
    let bytes = text.as_bytes();
    let mut cells = Vec::new();
    let mut buf = String::new();
    let mut cell_start = 0usize;
    let mut i = 0usize;
    let mut had_pipe = false;

    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\\' && i + 1 < bytes.len() && bytes[i + 1] == b'|' {
            buf.push('\\');
            buf.push('|');
            i += 2;
            continue;
        }
        if b == b'`' {
            let run_len = count_run(bytes, i, bytes.len(), b'`');
            for _ in 0..run_len {
                buf.push('`');
            }
            i += run_len;
            while i < bytes.len() {
                if bytes[i] == b'`' && count_run(bytes, i, bytes.len(), b'`') == run_len {
                    for _ in 0..run_len {
                        buf.push('`');
                    }
                    i += run_len;
                    break;
                }
                let ch = text[i..].chars().next().unwrap();
                buf.push(ch);
                i += ch.len_utf8();
            }
            continue;
        }
        if b == b'|' {
            had_pipe = true;
            let cell = finalize_table_cell(&buf, base_offset + cell_start);
            cells.push(cell);
            buf.clear();
            i += 1;
            cell_start = i;
            continue;
        }
        let ch = text[i..].chars().next().unwrap();
        buf.push(ch);
        i += ch.len_utf8();
    }
    let cell = finalize_table_cell(&buf, base_offset + cell_start);
    cells.push(cell);

    if had_pipe && cells.len() > 1 {
        if cells.first().is_some_and(|cell| cell.text.is_empty()) {
            cells.remove(0);
        }
        if cells.last().is_some_and(|cell| cell.text.is_empty()) {
            cells.pop();
        }
    }

    (cells, had_pipe)
}

fn finalize_table_cell(text: &str, start: usize) -> TableCellRaw {
    let bytes = text.as_bytes();
    let mut leading = 0usize;
    while leading < bytes.len() && (bytes[leading] == b' ' || bytes[leading] == b'\t') {
        leading += 1;
    }
    let mut trailing = bytes.len();
    while trailing > leading && (bytes[trailing - 1] == b' ' || bytes[trailing - 1] == b'\t') {
        trailing -= 1;
    }
    TableCellRaw {
        text: text[leading..trailing].to_string(),
        start: start + leading,
    }
}

fn parse_table_separator(text: &str, base_offset: usize) -> Option<Vec<TableAlign>> {
    let (cells, had_pipe) = split_table_cells(text, base_offset);
    if !had_pipe {
        return None;
    }
    let mut aligns = Vec::new();
    for cell in cells {
        let trimmed = cell.text.trim();
        if trimmed.is_empty() {
            return None;
        }
        let left = trimmed.starts_with(':');
        let right = trimmed.ends_with(':');
        let core = trimmed.trim_matches(':');
        if core.len() < 3 || !core.chars().all(|ch| ch == '-') {
            return None;
        }
        let align = match (left, right) {
            (true, true) => TableAlign::Center,
            (true, false) => TableAlign::Left,
            (false, true) => TableAlign::Right,
            (false, false) => TableAlign::None,
        };
        aligns.push(align);
    }
    Some(aligns)
}

fn parse_table_cells(
    parser: &mut Parser,
    line_start: usize,
    cells: &[TableCellRaw],
    expected: usize,
    parse_inlines: bool,
) -> Vec<InlineSeq> {
    let mut out = Vec::new();
    for cell in cells.iter().take(expected) {
        let inlines = if parse_inlines {
            parser.parse_inline(&cell.text, line_start + cell.start)
        } else {
            Vec::new()
        };
        out.push(inlines);
    }
    while out.len() < expected {
        out.push(Vec::new());
    }
    out
}

fn detect_task_marker(blocks: &mut [Block]) -> Option<bool> {
    let first = blocks.first_mut()?;
    let BlockKind::Paragraph { content } = &mut first.kind else {
        return None;
    };
    take_task_marker(content)
}

fn take_task_marker(content: &mut InlineSeq) -> Option<bool> {
    let mut prefix = [0u8; 4];
    let mut filled = 0;

    for inline in content.iter() {
        let InlineKind::Text(text) = &inline.kind else {
            return None;
        };
        if text.is_empty() {
            continue;
        }
        for byte in text.as_bytes() {
            prefix[filled] = *byte;
            filled += 1;
            if filled == prefix.len() {
                break;
            }
        }
        if filled == prefix.len() {
            break;
        }
    }

    if filled < prefix.len() || prefix[0] != b'[' || prefix[2] != b']' {
        return None;
    }
    if prefix[3] != b' ' && prefix[3] != b'\t' {
        return None;
    }
    let checked = match prefix[1] {
        b' ' => false,
        b'x' | b'X' => true,
        _ => return None,
    };

    let mut remaining = prefix.len();
    let mut idx = 0;
    while idx < content.len() && remaining > 0 {
        let InlineKind::Text(text) = &mut content[idx].kind else {
            return None;
        };
        if text.is_empty() {
            content.remove(idx);
            continue;
        }
        let remove_len = remaining.min(text.len());
        let new_text = text[remove_len..].to_string();
        remaining -= remove_len;
        if new_text.is_empty() {
            content.remove(idx);
        } else {
            *text = new_text;
            idx += 1;
        }
    }

    if remaining == 0 { Some(checked) } else { None }
}

fn autolink_inlines(inlines: &mut InlineSeq) {
    let mut out = Vec::new();
    for inline in inlines.drain(..) {
        match inline.kind {
            InlineKind::Text(text) => {
                out.extend(split_autolinks(&text, inline.span));
            }
            InlineKind::Emph(children) => {
                let mut children = children;
                autolink_inlines(&mut children);
                out.push(Inline {
                    span: inline.span,
                    kind: InlineKind::Emph(children),
                });
            }
            InlineKind::Strong(children) => {
                let mut children = children;
                autolink_inlines(&mut children);
                out.push(Inline {
                    span: inline.span,
                    kind: InlineKind::Strong(children),
                });
            }
            InlineKind::Strikethrough(children) => {
                let mut children = children;
                autolink_inlines(&mut children);
                out.push(Inline {
                    span: inline.span,
                    kind: InlineKind::Strikethrough(children),
                });
            }
            InlineKind::Link { .. }
            | InlineKind::LinkRef { .. }
            | InlineKind::Image { .. }
            | InlineKind::ImageRef { .. }
            | InlineKind::CodeSpan(_)
            | InlineKind::HtmlSpan { .. }
            | InlineKind::MathInline { .. }
            | InlineKind::Ref { .. }
            | InlineKind::SoftBreak
            | InlineKind::HardBreak => {
                out.push(inline);
            }
        }
    }
    *inlines = out;
}

fn split_autolinks(text: &str, span: Span) -> InlineSeq {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    let mut last = 0usize;
    let base_start = span.start;
    let base_end = span.end;
    let clamp = |offset: usize| {
        let pos = base_start.saturating_add(offset);
        if pos > base_end { base_end } else { pos }
    };
    while i < bytes.len() {
        if !text.is_char_boundary(i) {
            i += 1;
            continue;
        }
        if let Some(link) = match_autolink_literal(text, i) {
            if link.start > last {
                let span = Span {
                    start: clamp(last),
                    end: clamp(link.start),
                };
                out.push(Inline {
                    span,
                    kind: InlineKind::Text(text[last..link.start].to_string()),
                });
            }
            let link_span = Span {
                start: clamp(link.start),
                end: clamp(link.end),
            };
            let child_span = Span {
                start: clamp(link.start),
                end: clamp(link.end),
            };
            out.push(Inline {
                span: link_span,
                kind: InlineKind::Link {
                    url: link.url,
                    title: None,
                    children: vec![Inline {
                        span: child_span,
                        kind: InlineKind::Text(link.display),
                    }],
                },
            });
            i = link.end;
            last = link.end;
            continue;
        }
        i += 1;
    }
    if last < bytes.len() {
        out.push(Inline {
            span: Span {
                start: clamp(last),
                end: clamp(bytes.len()),
            },
            kind: InlineKind::Text(text[last..].to_string()),
        });
    }
    out
}

struct AutolinkLiteral {
    start: usize,
    end: usize,
    url: String,
    display: String,
}

fn match_autolink_literal(text: &str, start: usize) -> Option<AutolinkLiteral> {
    let bytes = text.as_bytes();
    let prev = if start == 0 {
        None
    } else {
        bytes.get(start - 1).copied()
    };
    if !is_autolink_boundary(prev) {
        return None;
    }
    if text[start..].starts_with("http://") || text[start..].starts_with("https://") {
        let end = scan_autolink_end(text, start);
        return build_autolink(text, start, end, false);
    }
    if text[start..].starts_with("www.") {
        let end = scan_autolink_end(text, start);
        if end <= start + 4 {
            return None;
        }
        let rest = &text[start + 4..end];
        if !rest.contains('.') {
            return None;
        }
        return build_autolink(text, start, end, true);
    }
    let end = scan_email_end(text, start)?;
    let candidate = &text[start..end];
    if is_autolink_email(candidate) {
        return Some(AutolinkLiteral {
            start,
            end,
            url: format!("mailto:{}", candidate),
            display: candidate.to_string(),
        });
    }
    None
}

fn is_autolink_boundary(prev: Option<u8>) -> bool {
    match prev {
        None => true,
        Some(b) => b.is_ascii_whitespace() || matches!(b, b'(' | b'[' | b'{' | b'"' | b'\''),
    }
}

fn scan_autolink_end(text: &str, start: usize) -> usize {
    let bytes = text.as_bytes();
    let mut end = start;
    while end < bytes.len() {
        let b = bytes[end];
        if b.is_ascii_whitespace() || matches!(b, b'<' | b'>' | b'"' | b'\'') {
            break;
        }
        end += 1;
    }
    while end > start && !text.is_char_boundary(end) {
        end -= 1;
    }
    trim_autolink_punct(text, start, end)
}

fn scan_email_end(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut end = start;
    while end < bytes.len() {
        let b = bytes[end];
        if b.is_ascii_whitespace() || matches!(b, b'<' | b'>' | b'"' | b'\'') {
            break;
        }
        end += 1;
    }
    if end == start {
        None
    } else {
        Some(trim_autolink_punct(text, start, end))
    }
}

fn trim_autolink_punct(text: &str, start: usize, mut end: usize) -> usize {
    let bytes = text.as_bytes();
    while end > start {
        let b = bytes[end - 1];
        if matches!(b, b'.' | b',' | b';' | b':' | b'!' | b'?') {
            end -= 1;
            continue;
        }
        break;
    }
    if end > start && bytes[end - 1] == b')' {
        end = trim_autolink_brackets(bytes, start, end, b'(', b')');
    }
    if end > start && bytes[end - 1] == b']' {
        end = trim_autolink_brackets(bytes, start, end, b'[', b']');
    }
    if end > start && bytes[end - 1] == b'}' {
        end = trim_autolink_brackets(bytes, start, end, b'{', b'}');
    }
    end
}

fn trim_autolink_brackets(
    bytes: &[u8],
    start: usize,
    mut end: usize,
    open: u8,
    close: u8,
) -> usize {
    let mut open_count = 0usize;
    let mut close_count = 0usize;
    for b in &bytes[start..end] {
        if *b == open {
            open_count += 1;
        } else if *b == close {
            close_count += 1;
        }
    }
    while end > start && bytes[end - 1] == close && close_count > open_count {
        end -= 1;
        close_count = close_count.saturating_sub(1);
    }
    end
}

fn build_autolink(
    text: &str,
    start: usize,
    end: usize,
    add_scheme: bool,
) -> Option<AutolinkLiteral> {
    if end <= start {
        return None;
    }
    let display = text[start..end].to_string();
    let url = if add_scheme {
        format!("http://{}", display)
    } else {
        display.clone()
    };
    Some(AutolinkLiteral {
        start,
        end,
        url,
        display,
    })
}

fn contains_html_closing_tag(line: &str, tag: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    let needle = format!("</{}", tag);
    let mut search = 0usize;
    while let Some(pos) = lower[search..].find(&needle) {
        let idx = search + pos;
        let rest = &lower[idx + needle.len()..];
        if rest.is_empty() {
            return true;
        }
        let b = rest.as_bytes()[0];
        if b == b'>' || b.is_ascii_whitespace() {
            return true;
        }
        search = idx + needle.len();
    }
    false
}

struct ListMarker {
    ordered: bool,
    start: Option<u64>,
    marker_len: usize,
    content_indent: usize,
    empty: bool,
    marker: u8,
}

#[derive(Clone, Debug)]
struct TableCellRaw {
    text: String,
    start: usize,
}

fn indent_prefix_len(text: &str, required: usize) -> Option<usize> {
    if required == 0 {
        return Some(0);
    }
    let bytes = text.as_bytes();
    let mut columns = 0;
    for (idx, byte) in bytes.iter().enumerate() {
        let next_cols = match advance_column(columns, *byte) {
            Some(next) => next,
            None => break,
        };
        columns = next_cols;
        if columns >= required {
            return Some(idx + 1);
        }
    }
    None
}

/// Remove up to `columns` columns of indentation from the start of a line,
/// properly handling tabs. Returns the remaining text with tabs expanded to spaces.
fn remove_indent_columns(text: &str, columns: usize) -> String {
    let bytes = text.as_bytes();
    let mut col = 0; // Column position in the INPUT
    let mut byte_pos = 0;

    // Find the byte position where we've consumed `columns` columns
    while byte_pos < bytes.len() && col < columns {
        match bytes[byte_pos] {
            b' ' => {
                col += 1;
                byte_pos += 1;
            }
            b'\t' => {
                let next_col = col + (4 - (col % 4));
                if next_col > columns {
                    // Tab extends past the indent boundary
                    break;
                }
                col = next_col;
                byte_pos += 1;
            }
            _ => break,
        }
    }

    // If we stopped in the middle of a tab, emit spaces for the remaining columns
    let mut result = String::new();
    if col < columns && byte_pos < bytes.len() && bytes[byte_pos] == b'\t' {
        // Partial tab - emit the spaces that come after removing the indent
        let tab_start = col;
        let tab_end = tab_start + (4 - (tab_start % 4));
        let spaces_after_indent = tab_end - columns;
        for _ in 0..spaces_after_indent {
            result.push(' ');
        }
        // Update col to the position after the full tab in the input
        col = tab_end;
        byte_pos += 1;
    }

    // Append the rest of the line, expanding any remaining tabs
    // Tabs expand based on their position in the ORIGINAL input, not the output
    let rest = &text[byte_pos..];

    for ch in rest.chars() {
        if ch == '\t' {
            // This tab is at column `col` in the original input
            // It should expand to the next multiple of 4 from that position
            let next_tab_stop = col + (4 - (col % 4));
            let spaces = next_tab_stop - col;
            for _ in 0..spaces {
                result.push(' ');
            }
            col = next_tab_stop;
        } else {
            result.push(ch);
            if ch != '\r' && ch != '\n' {
                col += 1;
            }
        }
    }

    result
}

fn remove_list_indent(text: &str, _marker_len: usize, content_indent: usize) -> String {
    // For list items, we need to remove content_indent columns from the entire line.
    // Key insight from comrak: when we partially consume a tab, we output the remaining
    // columns as spaces, then expand remaining tabs based on their position in the original input.

    let bytes = text.as_bytes();
    let mut result = String::new();
    let mut col = 0; // Column position in the INPUT
    let mut byte_pos = 0;

    // First pass: skip content_indent columns
    while byte_pos < bytes.len() && col < content_indent {
        match bytes[byte_pos] {
            b' ' => {
                col += 1;
                byte_pos += 1;
            }
            b'\t' => {
                let next_col = col + (4 - (col % 4));
                if next_col > content_indent {
                    // Partial tab - will handle below
                    break;
                }
                col = next_col;
                byte_pos += 1;
            }
            _ => {
                // Non-whitespace character (e.g., list marker)
                col += 1;
                byte_pos += 1;
            }
        }
    }

    // Handle partial tab: output remaining columns as spaces, skip the tab byte
    if col < content_indent && byte_pos < bytes.len() && bytes[byte_pos] == b'\t' {
        let tab_end = col + (4 - (col % 4));
        let spaces_after_indent = tab_end - content_indent;

        for _ in 0..spaces_after_indent {
            result.push(' ');
        }
        // Update col to the position after the full tab in the input
        col = tab_end;
        byte_pos += 1;
    }

    // Second pass: append the rest of the line, expanding tabs based on their position
    // in the ORIGINAL input, not the output
    if byte_pos < bytes.len() {
        for ch in text[byte_pos..].chars() {
            if ch == '\t' {
                // This tab is at column `col` in the original input
                // It should expand to the next multiple of 4 from that position
                let next_tab_stop = col + (4 - (col % 4));
                let spaces = next_tab_stop - col;
                for _ in 0..spaces {
                    result.push(' ');
                }
                col = next_tab_stop;
            } else {
                result.push(ch);
                if ch != '\r' && ch != '\n' {
                    col += 1;
                }
            }
        }
    }

    result
}

fn parse_list_marker(text: &str) -> Option<ListMarker> {
    // Minimal list detection with up to 3 leading spaces.
    if is_thematic_break_line(text) {
        return None;
    }
    let bytes = text.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let mut idx = 0;
    let mut indent_cols = 0;
    while idx < bytes.len() && idx < 3 && bytes[idx] == b' ' {
        idx += 1;
        indent_cols += 1;
    }
    if idx < bytes.len() && bytes[idx] == b' ' {
        return None;
    }

    if idx < bytes.len() {
        let ch = bytes[idx];
        if matches!(ch, b'-' | b'+' | b'*') {
            let marker_pos = idx;
            let marker_width = 1;
            idx += 1;
            let start_col = indent_cols + marker_width;
            let (post_cols, post_bytes, content_ws_bytes, content_cols, has_nonspace) =
                scan_post_marker(bytes, idx, start_col);
            if post_cols == 0 && has_nonspace {
                return None;
            }
            let empty_item = !has_nonspace;

            // If content_cols == 0 and not empty, use fallback: padding = marker + 1
            let (content_indent, marker_len) = if empty_item {
                (
                    indent_cols + marker_width + 1,
                    marker_pos + marker_width + post_bytes,
                )
            } else if content_cols == 0 {
                // Fallback: too much whitespace (>4 cols)
                (indent_cols + marker_width + 1, marker_pos + marker_width)
            } else {
                (
                    indent_cols + marker_width + content_cols,
                    marker_pos + marker_width + content_ws_bytes,
                )
            };
            return Some(ListMarker {
                ordered: false,
                start: None,
                marker_len,
                content_indent,
                empty: empty_item,
                marker: ch,
            });
        }
    }

    let digit_start = idx;
    while idx < bytes.len() && bytes[idx].is_ascii_digit() {
        idx += 1;
    }
    let digits_len = idx.saturating_sub(digit_start);
    if digits_len == 0 || digits_len > 9 || idx >= bytes.len() {
        return None;
    }
    let delimiter = bytes[idx];
    if delimiter != b'.' && delimiter != b')' {
        return None;
    }
    let marker_end = idx + 1;
    let marker_width = marker_end - digit_start;
    idx = marker_end;
    let start_col = indent_cols + marker_width;
    let (post_cols, post_bytes, content_ws_bytes, content_cols, has_nonspace) =
        scan_post_marker(bytes, idx, start_col);
    if post_cols == 0 && has_nonspace {
        return None;
    }
    let empty_item = !has_nonspace;

    // If content_cols == 0 and not empty, use fallback: padding = marker + 1
    let (content_indent, marker_len) = if empty_item {
        (indent_cols + marker_width + 1, marker_end + post_bytes)
    } else if content_cols == 0 {
        // Fallback: too much whitespace (>4 cols)
        (indent_cols + marker_width + 1, marker_end)
    } else {
        (
            indent_cols + marker_width + content_cols,
            marker_end + content_ws_bytes,
        )
    };
    let start_num = text[digit_start..digit_start + digits_len]
        .parse::<u64>()
        .ok();
    Some(ListMarker {
        ordered: true,
        start: start_num,
        marker_len,
        content_indent,
        empty: empty_item,
        marker: delimiter,
    })
}

fn scan_post_marker(
    bytes: &[u8],
    start: usize,
    start_col: usize,
) -> (usize, usize, usize, usize, bool) {
    let mut idx = start;
    let mut col = start_col;
    let mut tab_remainder = 0; // Remaining columns in a partially consumed tab

    // Advance column by column, up to 5 columns
    // The key insight: we track tab remainder to avoid re-processing the same tab byte
    while col - start_col < 5 && idx < bytes.len() {
        if tab_remainder > 0 {
            // We're in the middle of a tab - consume one more column from it
            tab_remainder -= 1;
            col += 1;
            if tab_remainder == 0 {
                // Tab fully consumed, advance to next byte
                idx += 1;
            }
        } else {
            match bytes[idx] {
                b' ' => {
                    col += 1;
                    idx += 1;
                }
                b'\t' => {
                    let chars_to_tab = 4 - (col % 4);
                    // Consume 1 column from this tab
                    col += 1;
                    if chars_to_tab > 1 {
                        // Tab not fully consumed - set remainder
                        tab_remainder = chars_to_tab - 1;
                    } else {
                        // Tab fully consumed in one column
                        idx += 1;
                    }
                }
                _ => break, // Non-whitespace
            }
        }
    }

    let i = col - start_col;
    let total_cols = i;
    let total_bytes = idx - start;

    // Scan ahead to check if there's non-whitespace content
    let mut has_nonspace = false;
    let mut scan_idx = idx;
    let mut scan_tab_remainder = tab_remainder;

    while scan_idx < bytes.len() {
        if scan_tab_remainder > 0 {
            scan_tab_remainder -= 1;
            if scan_tab_remainder == 0 {
                scan_idx += 1;
            }
        } else {
            match bytes[scan_idx] {
                b' ' => scan_idx += 1,
                b'\t' => {
                    // Just skip the entire tab for this scan
                    scan_idx += 1;
                }
                _ => {
                    has_nonspace = true;
                    break;
                }
            }
        }
    }

    // Check if i (columns advanced) is in valid range [1, 4]
    if i == 0 || i > 4 {
        return (total_cols, total_bytes, 0, 0, has_nonspace);
    }

    // Valid range: calculate content_bytes for the i columns
    // This is the number of bytes that represent exactly i columns
    let content_cols = i;
    let mut temp_col = start_col;
    let mut temp_idx = start;

    while temp_col < start_col + content_cols && temp_idx < bytes.len() {
        match bytes[temp_idx] {
            b' ' => {
                temp_col += 1;
                temp_idx += 1;
            }
            b'\t' => {
                let next_col = temp_col + (4 - (temp_col % 4));
                if next_col <= start_col + content_cols {
                    temp_col = next_col;
                    temp_idx += 1;
                } else {
                    // Partial tab - this byte is NOT fully consumed
                    break;
                }
            }
            _ => break,
        }
    }

    let content_bytes = temp_idx - start;

    (
        total_cols,
        total_bytes,
        content_bytes,
        content_cols,
        has_nonspace,
    )
}

fn advance_column(columns: usize, byte: u8) -> Option<usize> {
    match byte {
        b' ' => Some(columns + 1),
        b'\t' => Some(columns + (4 - (columns % 4))),
        _ => None,
    }
}

fn block_line_range(lines: &[Line], span: Span) -> Option<(usize, usize)> {
    let mut start_idx = None;
    let mut end_idx = None;
    for (idx, line) in lines.iter().enumerate() {
        if start_idx.is_none() && span.start <= line.end {
            start_idx = Some(idx);
        }
        if span.end <= line.end {
            end_idx = Some(idx);
            break;
        }
    }
    match (start_idx, end_idx) {
        (Some(start), Some(end)) => Some((start, end)),
        _ => None,
    }
}

fn item_has_blank_between_blocks(lines: &[Line], blocks: &[Block]) -> bool {
    if blocks.is_empty() {
        return false;
    }
    let mut covered = vec![false; lines.len()];
    for block in blocks {
        let (start_idx, end_idx) = match block_line_range(lines, block.span) {
            Some(value) => value,
            None => return true,
        };
        for idx in start_idx..=end_idx {
            if let Some(slot) = covered.get_mut(idx) {
                *slot = true;
            }
        }
    }
    let first_covered = covered.iter().position(|v| *v);
    let last_covered = covered.iter().rposition(|v| *v);
    let last_relevant = lines.iter().rposition(|line| !line.text.trim().is_empty());
    let (start_idx, end_idx) = match (first_covered, last_covered) {
        (Some(start), Some(end)) => {
            let end = match last_relevant {
                Some(last) if last > end => last,
                _ => end,
            };
            (start, end)
        }
        _ => return false,
    };
    for (idx, line) in lines
        .iter()
        .enumerate()
        .skip(start_idx)
        .take(end_idx - start_idx + 1)
    {
        if covered.get(idx).copied().unwrap_or(false) {
            continue;
        }
        let trimmed = line.text.trim();
        if trimmed.is_empty()
            || (trimmed.starts_with('{') && trimmed.ends_with('}') && trimmed.len() >= 2)
        {
            return true;
        }
    }
    false
}

fn count_lines(text: &str) -> u32 {
    if text.is_empty() {
        return 0;
    }
    let mut count = 1;
    for byte in text.bytes() {
        if byte == b'\n' {
            count += 1;
        }
    }
    count
}

fn is_valid_label(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    name.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn is_hex_color(value: &str) -> bool {
    let value = value.trim();
    let hex = match value.strip_prefix('#') {
        Some(hex) => hex,
        None => return false,
    };
    let len = hex.len();
    if len != 3 && len != 6 {
        return false;
    }
    hex.chars().all(|c| c.is_ascii_hexdigit())
}

fn is_border_style(value: &str) -> bool {
    matches!(
        value.trim(),
        "solid" | "dashed" | "dotted" | "double" | "none"
    )
}

fn is_border_width(value: &str) -> bool {
    let value = value.trim();
    if value.is_empty() {
        return false;
    }
    let digits = value.strip_suffix("px").unwrap_or(value);
    !digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())
}

fn is_ascii_punctuation(byte: u8) -> bool {
    byte.is_ascii_punctuation()
}

fn count_run(bytes: &[u8], start: usize, end: usize, needle: u8) -> usize {
    let mut i = start;
    while i < end {
        if bytes[i] != needle {
            break;
        }
        i += 1;
    }
    i.saturating_sub(start)
}

fn delimiter_properties(
    buffer: &str,
    start: usize,
    end: usize,
    pos: usize,
    run_len: usize,
    delim: u8,
) -> (bool, bool) {
    let before = if pos > start {
        buffer[..pos].chars().next_back()
    } else {
        None
    };
    let after_pos = pos + run_len;
    let after = if after_pos < end {
        buffer[after_pos..end].chars().next()
    } else {
        None
    };

    let before_is_whitespace = match before {
        Some(ch) => ch.is_whitespace(),
        None => true,
    };
    let after_is_whitespace = match after {
        Some(ch) => ch.is_whitespace(),
        None => true,
    };
    let before_is_punctuation = before.is_some_and(is_unicode_punctuation);
    let after_is_punctuation = after.is_some_and(is_unicode_punctuation);

    let left_flanking = !after_is_whitespace
        && (!after_is_punctuation || before_is_whitespace || before_is_punctuation);
    let right_flanking = !before_is_whitespace
        && (!before_is_punctuation || after_is_whitespace || after_is_punctuation);

    if delim == b'_' {
        let can_open = left_flanking && (!right_flanking || before_is_punctuation);
        let can_close = right_flanking && (!left_flanking || after_is_punctuation);
        (can_open, can_close)
    } else {
        (left_flanking, right_flanking)
    }
}

fn is_unicode_punctuation(ch: char) -> bool {
    !ch.is_whitespace() && !ch.is_alphanumeric()
}

fn is_attr_name_start(b: u8) -> bool {
    b.is_ascii_alphabetic() || b == b'_' || b == b':'
}

fn is_attr_name_continue(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b':' | b'.' | b'-')
}

fn delimiter_blocked(opener: &Delimiter, closer: &Delimiter) -> bool {
    if opener.ch != closer.ch {
        return false;
    }
    let opener_both = opener.orig_can_open && opener.orig_can_close;
    let closer_both = closer.orig_can_open && closer.orig_can_close;
    if !opener_both && !closer_both {
        return false;
    }
    if (opener.len + closer.len) % 3 != 0 {
        return false;
    }
    opener.len % 3 != 0 || closer.len % 3 != 0
}

fn parse_label(bytes: &[u8], start: usize, end: usize) -> Option<(String, usize)> {
    if start >= end {
        return None;
    }
    let mut i = start;
    while i < end {
        let b = bytes[i];
        let ok = b.is_ascii_alphanumeric() || b == b'_' || b == b'-';
        if !ok {
            break;
        }
        i += 1;
    }
    if i == start {
        None
    } else {
        let label = String::from_utf8_lossy(&bytes[start..i]).to_string();
        Some((label, i))
    }
}

fn find_bracket_end(bytes: &[u8], start: usize, end: usize) -> Option<(usize, bool)> {
    let mut i = start;
    let mut depth = 0usize;
    let mut escaped = false;
    let mut had_newline = false;
    while i < end {
        let b = bytes[i];
        if b == b'\n' {
            had_newline = true;
        }
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if b == b'\\' {
            escaped = true;
            i += 1;
            continue;
        }
        if b == b'[' {
            depth += 1;
        } else if b == b']' {
            if depth == 0 {
                return Some((i, had_newline));
            }
            depth -= 1;
        }
        i += 1;
    }
    None
}

fn parse_link_title(bytes: &[u8], start: usize, end: usize) -> Option<(String, usize)> {
    if start >= end {
        return None;
    }
    let open = bytes[start];
    let close = match open {
        b'"' => b'"',
        b'\'' => b'\'',
        b'(' => b')',
        _ => return None,
    };
    let mut i = start + 1;
    let mut out = Vec::new();
    let mut escaped = false;
    while i < end {
        let b = bytes[i];
        if b == b'\n' {
            return None;
        }
        if escaped {
            out.push(b);
            escaped = false;
            i += 1;
            continue;
        }
        if b == b'\\' {
            if i + 1 < end && is_ascii_punctuation(bytes[i + 1]) {
                escaped = true;
                i += 1;
                continue;
            }
            out.push(b'\\');
            i += 1;
            continue;
        }
        if b == close {
            let title = match String::from_utf8(out) {
                Ok(value) => value,
                Err(err) => String::from_utf8_lossy(&err.into_bytes()).to_string(),
            };
            // Unescape backslashes and decode entities in title
            let title = unescape_and_decode(&title);
            return Some((title, i + 1));
        }
        out.push(b);
        i += 1;
    }
    None
}

fn parse_inline_link_destination(
    buffer: &str,
    start: usize,
    end: usize,
) -> Option<(String, Option<String>, usize)> {
    let bytes = buffer.as_bytes();
    let mut i = start;
    if i >= end || bytes[i] != b'(' {
        return None;
    }
    i += 1;
    while i < end && bytes[i].is_ascii_whitespace() {
        if bytes[i] == b'\n' {
            return None;
        }
        i += 1;
    }
    if i >= end {
        return None;
    }

    let mut url_bytes = Vec::new();
    if bytes[i] == b'<' {
        i += 1;
        let mut closed = false;
        while i < end {
            let b = bytes[i];
            if b == b'\n' {
                return None;
            }
            if b == b'\\' {
                if i + 1 < end && is_ascii_punctuation(bytes[i + 1]) {
                    url_bytes.push(bytes[i + 1]);
                    i += 2;
                    continue;
                }
                url_bytes.push(b'\\');
                i += 1;
                continue;
            }
            if b == b'>' {
                closed = true;
                i += 1;
                break;
            }
            url_bytes.push(b);
            i += 1;
        }
        if !closed {
            return None;
        }
    } else {
        let mut depth = 0usize;
        while i < end {
            let b = bytes[i];
            if b.is_ascii_whitespace() {
                break;
            }
            if b == b'\\' {
                if i + 1 < end && is_ascii_punctuation(bytes[i + 1]) {
                    url_bytes.push(bytes[i + 1]);
                    i += 2;
                    continue;
                }
                url_bytes.push(b'\\');
                i += 1;
                continue;
            }
            if b == b'(' {
                depth += 1;
                url_bytes.push(b);
                i += 1;
                continue;
            }
            if b == b')' {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                url_bytes.push(b);
                i += 1;
                continue;
            }
            url_bytes.push(b);
            i += 1;
        }
        if depth > 0 {
            return None;
        }
    }

    let url = match String::from_utf8(url_bytes) {
        Ok(value) => value,
        Err(err) => String::from_utf8_lossy(&err.into_bytes()).to_string(),
    };
    // Unescape backslashes and decode entities in URL, then percent-encode
    let url = unescape_and_decode(&url);
    let url = percent_encode_url(&url);

    let mut had_space = false;
    while i < end && bytes[i].is_ascii_whitespace() {
        had_space = true;
        i += 1;
    }
    if i >= end {
        return None;
    }
    if bytes[i] == b')' {
        return Some((url, None, i));
    }
    if !had_space {
        return None;
    }

    let (title, next) = parse_link_title(bytes, i, end)?;
    i = next;
    while i < end && bytes[i].is_ascii_whitespace() {
        i += 1;
    }
    if i < end && bytes[i] == b')' {
        return Some((url, Some(title), i));
    }
    None
}

fn parse_link_reference_definition_lines(
    lines: &[Line],
    start: usize,
) -> Option<(String, LinkDefinition, usize)> {
    let line = lines.get(start)?;
    let bytes = line.text.as_bytes();
    let mut i = 0;
    let mut spaces = 0;
    while i < bytes.len() && bytes[i] == b' ' && spaces < 4 {
        i += 1;
        spaces += 1;
    }
    if spaces > 3 {
        return None;
    }
    if i >= bytes.len() || bytes[i] != b'[' {
        return None;
    }

    let (label_bytes, label_end_line, label_end_pos) =
        if let Some((label_end, had_newline)) = find_bracket_end(bytes, i + 1, bytes.len()) {
            if had_newline {
                return None;
            }
            (bytes[i + 1..label_end].to_vec(), start, label_end)
        } else {
            parse_link_label_multiline(lines, start, i + 1)?
        };
    let label = normalize_link_label(&label_bytes);
    if label.is_empty() {
        return None;
    }
    if has_unescaped_brackets(&label_bytes) {
        return None;
    }
    let end_line = lines.get(label_end_line)?;
    let end_bytes = end_line.text.as_bytes();
    let mut pos = label_end_pos + 1;
    if pos >= end_bytes.len() || end_bytes[pos] != b':' {
        return None;
    }
    pos += 1;

    let mut line_idx = label_end_line;
    while pos < end_bytes.len() && is_space_or_tab(end_bytes[pos]) {
        pos += 1;
    }
    let mut dest_on_new_line = false;
    if pos >= end_bytes.len() {
        line_idx += 1;
        if line_idx >= lines.len() {
            return None;
        }
        pos = skip_spaces_tabs(&lines[line_idx].text, 0);
        dest_on_new_line = true;
    }
    if pos >= lines[line_idx].text.len() {
        return None;
    }

    let dest_bytes = lines[line_idx].text.as_bytes();
    let (url, next_pos) = parse_reference_destination(dest_bytes, pos, dest_bytes.len())?;
    pos = next_pos;

    let mut had_space_after_dest = false;
    while pos < dest_bytes.len() && is_space_or_tab(dest_bytes[pos]) {
        had_space_after_dest = true;
        pos += 1;
    }
    let mut title = None;
    let mut end_line_idx = line_idx;

    // 1. Title on the same line.
    if pos < lines[line_idx].text.len() {
        let first = dest_bytes[pos];
        if is_title_delim(first) {
            if !had_space_after_dest {
                // Title delimiter must be preceded by a space or tab.
                return None;
            }
            let (parsed, title_end_line, title_end_pos) =
                parse_link_title_multiline(lines, line_idx, pos)?;
            // Title is accepted when only spaces/tabs or end-of-line follow.
            if trailing_spaces_tabs_only(&lines[title_end_line].text, title_end_pos) {
                title = Some(unescape_and_decode(&parsed));
                end_line_idx = title_end_line;
            } else {
                // Non-whitespace after the title invalidates the definition.
                return None;
            }
        } else {
            // Not a title delimiter: no title.
        }
    } else {
        // 2. Title on the next line.
        let peek_idx = line_idx + 1;
        if peek_idx < lines.len() {
            let peek_pos = skip_spaces_tabs(&lines[peek_idx].text, 0);
            if peek_pos < lines[peek_idx].text.len() {
                let first = lines[peek_idx].text.as_bytes()[peek_pos];
                if is_title_delim(first) {
                    // If destination is on the same line, next-line titles must be indented.
                    if !dest_on_new_line && peek_pos == 0 {
                        // Treat as destination-only definition (no title).
                        return Some((
                            label,
                            LinkDefinition { url, title: None },
                            end_line_idx + 1,
                        ));
                    }
                    let (parsed, title_end_line, title_end_pos) =
                        parse_link_title_multiline(lines, peek_idx, peek_pos)?;
                    // Title is accepted when only spaces/tabs or end-of-line follow.
                    if trailing_spaces_tabs_only(&lines[title_end_line].text, title_end_pos) {
                        title = Some(unescape_and_decode(&parsed));
                        end_line_idx = title_end_line;
                    } else {
                        // Non-whitespace after the title invalidates the definition.
                        return None;
                    }
                }
            }
        }
    }

    Some((label, LinkDefinition { url, title }, end_line_idx + 1))
}

fn skip_spaces_tabs(text: &str, mut pos: usize) -> usize {
    let bytes = text.as_bytes();
    while pos < bytes.len() && (bytes[pos] == b' ' || bytes[pos] == b'\t') {
        pos += 1;
    }
    pos
}

fn trailing_spaces_tabs_only(text: &str, pos: usize) -> bool {
    text.as_bytes()[pos..]
        .iter()
        .all(|b| *b == b' ' || *b == b'\t')
}

fn is_title_delim(byte: u8) -> bool {
    byte == b'"' || byte == b'\'' || byte == b'('
}

fn parse_link_title_multiline(
    lines: &[Line],
    mut line_idx: usize,
    mut pos: usize,
) -> Option<(String, usize, usize)> {
    let line = lines.get(line_idx)?;
    let bytes = line.text.as_bytes();
    if pos >= bytes.len() {
        return None;
    }
    let open = bytes[pos];
    let close = match open {
        b'"' => b'"',
        b'\'' => b'\'',
        b'(' => b')',
        _ => return None,
    };
    pos += 1;
    let mut out = Vec::new();
    let mut escaped = false;

    loop {
        let line_text = &lines[line_idx].text;
        let bytes = line_text.as_bytes();
        while pos < bytes.len() {
            let b = bytes[pos];
            if escaped {
                out.push(b);
                escaped = false;
                pos += 1;
                continue;
            }
            if b == b'\\' {
                if pos + 1 < bytes.len() && is_ascii_punctuation(bytes[pos + 1]) {
                    escaped = true;
                    pos += 1;
                    continue;
                }
                out.push(b'\\');
                pos += 1;
                continue;
            }
            if b == close {
                let title = match String::from_utf8(out) {
                    Ok(value) => value,
                    Err(err) => String::from_utf8_lossy(&err.into_bytes()).to_string(),
                };
                return Some((title, line_idx, pos + 1));
            }
            out.push(b);
            pos += 1;
        }
        line_idx += 1;
        if line_idx >= lines.len() {
            return None;
        }
        if lines[line_idx].text.trim().is_empty() {
            return None;
        }
        out.push(b'\n');
        pos = 0;
    }
}

fn parse_link_label_multiline(
    lines: &[Line],
    mut line_idx: usize,
    mut pos: usize,
) -> Option<(Vec<u8>, usize, usize)> {
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut escaped = false;

    loop {
        let line = lines.get(line_idx)?;
        let bytes = line.text.as_bytes();
        while pos < bytes.len() {
            let b = bytes[pos];
            if escaped {
                out.push(b);
                escaped = false;
                pos += 1;
                continue;
            }
            if b == b'\\' {
                if pos + 1 < bytes.len() && is_ascii_punctuation(bytes[pos + 1]) {
                    escaped = true;
                    pos += 1;
                    continue;
                }
                out.push(b'\\');
                pos += 1;
                continue;
            }
            if b == b'[' {
                depth += 1;
                out.push(b);
                pos += 1;
                continue;
            }
            if b == b']' {
                if depth == 0 {
                    return Some((out, line_idx, pos));
                }
                depth = depth.saturating_sub(1);
                out.push(b);
                pos += 1;
                continue;
            }
            out.push(b);
            pos += 1;
        }

        line_idx += 1;
        if line_idx >= lines.len() {
            return None;
        }
        if lines[line_idx].text.trim().is_empty() {
            return None;
        }
        out.push(b'\n');
        pos = 0;
    }
}

fn parse_reference_destination(bytes: &[u8], start: usize, end: usize) -> Option<(String, usize)> {
    let mut i = start;
    let mut url_bytes = Vec::new();
    let mut angle = false;
    if i < end && bytes[i] == b'<' {
        angle = true;
        i += 1;
        let mut closed = false;
        while i < end {
            let b = bytes[i];
            if b == b'\n' {
                return None;
            }
            if b == b'\\' {
                if i + 1 < end && is_ascii_punctuation(bytes[i + 1]) {
                    url_bytes.push(bytes[i + 1]);
                    i += 2;
                    continue;
                }
                url_bytes.push(b'\\');
                i += 1;
                continue;
            }
            if b == b'>' {
                closed = true;
                i += 1;
                break;
            }
            url_bytes.push(b);
            i += 1;
        }
        if !closed {
            return None;
        }
    } else {
        while i < end {
            let b = bytes[i];
            if b.is_ascii_whitespace() {
                break;
            }
            if b == b'\\' {
                if i + 1 < end && is_ascii_punctuation(bytes[i + 1]) {
                    url_bytes.push(bytes[i + 1]);
                    i += 2;
                    continue;
                }
                url_bytes.push(b'\\');
                i += 1;
                continue;
            }
            url_bytes.push(b);
            i += 1;
        }
    }
    if url_bytes.is_empty() && !angle {
        return None;
    }
    let url = match String::from_utf8(url_bytes) {
        Ok(value) => value,
        Err(err) => String::from_utf8_lossy(&err.into_bytes()).to_string(),
    };
    // Unescape backslashes and decode entities in URL, then percent-encode
    let url = unescape_and_decode(&url);
    let url = percent_encode_url(&url);
    Some((url, i))
}

fn has_unescaped_brackets(bytes: &[u8]) -> bool {
    let mut escaped = false;
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if escaped {
            escaped = false;
            i += 1;
            continue;
        }
        if b == b'\\' && i + 1 < bytes.len() && is_label_escape(bytes[i + 1]) {
            escaped = true;
            i += 1;
            continue;
        }
        if b == b'[' || b == b']' {
            return true;
        }
        i += 1;
    }
    false
}

fn decode_entity(bytes: &[u8], start: usize, end: usize) -> Option<(Vec<u8>, usize)> {
    if start + 2 >= end {
        return None;
    }
    if bytes[start] != b'&' {
        return None;
    }
    let mut i = start + 1;
    if bytes[i] == b'#' {
        i += 1;
        let mut radix = 10;
        if i < end && (bytes[i] == b'x' || bytes[i] == b'X') {
            radix = 16;
            i += 1;
        }
        let num_start = i;
        while i < end && bytes[i].is_ascii_hexdigit() {
            i += 1;
        }
        if i == num_start || i >= end || bytes[i] != b';' {
            return None;
        }
        let number_str = match std::str::from_utf8(&bytes[num_start..i]) {
            Ok(value) => value,
            Err(_) => return None,
        };
        // CommonMark: reject numeric references with too many digits
        // Max 7 digits for decimal, 6 for hex (to stay within valid Unicode range)
        let max_digits = if radix == 16 { 6 } else { 7 };
        if number_str.len() > max_digits {
            return None;
        }
        let value = u32::from_str_radix(number_str, radix).ok()?;
        // CommonMark: invalid codepoints (0, surrogates, > 0x10FFFF) -> U+FFFD (replacement char)
        let ch = if value == 0 || (0xD800..=0xDFFF).contains(&value) || value > 0x10FFFF {
            '\u{FFFD}'
        } else {
            std::char::from_u32(value).unwrap_or('\u{FFFD}')
        };
        let mut out = [0u8; 4];
        let encoded = ch.encode_utf8(&mut out);
        return Some((encoded.as_bytes().to_vec(), i + 1));
    }
    let name_start = i;
    while i < end && bytes[i].is_ascii_alphanumeric() {
        i += 1;
    }
    if i == name_start || i >= end || bytes[i] != b';' {
        return None;
    }
    let name = &bytes[name_start..i];
    let name_str = std::str::from_utf8(name).ok()?;
    let decoded = lookup_named_entity(name_str)?;
    Some((decoded.as_bytes().to_vec(), i + 1))
}

/// Percent-encode non-ASCII characters in URL (CommonMark requirement)
fn percent_encode_url(url: &str) -> String {
    // Encode only non-ASCII characters and spaces.
    // Keep all other ASCII characters verbatim.
    let mut result = String::new();
    for ch in url.chars() {
        if ch == ' ' {
            result.push_str("%20");
        } else if ch.is_ascii() {
            result.push(ch);
        } else {
            let mut buf = [0u8; 4];
            let encoded = ch.encode_utf8(&mut buf);
            for &byte in encoded.as_bytes() {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

fn percent_encode_autolink_url(url: &str) -> String {
    let encoded = percent_encode_url(url);
    let encoded = encoded.replace('\\', "%5C");
    let encoded = encoded.replace('[', "%5B");
    encoded.replace(']', "%5D")
}

fn is_autolink_scheme(value: &str) -> bool {
    let bytes = value.as_bytes();
    let mut i = 0;
    if bytes.is_empty() || !bytes[0].is_ascii_alphabetic() {
        return false;
    }
    while i < bytes.len() {
        let b = bytes[i];
        if b == b':' {
            return i >= 2 && i + 1 < bytes.len();
        }
        let ok = b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.');
        if !ok {
            return false;
        }
        i += 1;
    }
    false
}

fn is_autolink_email(value: &str) -> bool {
    let mut parts = value.split('@');
    let local = match parts.next() {
        Some(part) if !part.is_empty() => part,
        _ => return false,
    };
    let domain = match parts.next() {
        Some(part) if !part.is_empty() => part,
        _ => return false,
    };
    if parts.next().is_some() {
        return false;
    }
    for b in local.bytes() {
        let ok = b.is_ascii_alphanumeric()
            || matches!(
                b,
                b'!' | b'#'
                    | b'$'
                    | b'%'
                    | b'&'
                    | b'\''
                    | b'*'
                    | b'+'
                    | b'-'
                    | b'/'
                    | b'='
                    | b'?'
                    | b'^'
                    | b'_'
                    | b'`'
                    | b'{'
                    | b'|'
                    | b'}'
                    | b'~'
                    | b'.'
            );
        if !ok {
            return false;
        }
    }
    let mut dot = false;
    for (idx, b) in domain.bytes().enumerate() {
        let ok = b.is_ascii_alphanumeric() || b == b'.' || b == b'-';
        if !ok {
            return false;
        }
        if b == b'.' {
            if idx == 0 {
                return false;
            }
            dot = true;
        }
    }
    dot && !domain.ends_with('.')
}

#[cfg(test)]
mod link_def_tests {
    use super::{parse_link_reference_definition_lines, split_lines};
    use crate::label::normalize_link_label;

    #[test]
    fn link_definition_with_backslashes_parses() {
        let input = "[foo]: /url\\bar\\*baz \"foo\\\"bar\\baz\"\n";
        let lines = split_lines(input);
        let parsed = parse_link_reference_definition_lines(&lines, 0);
        assert!(parsed.is_some(), "expected link definition to parse");
    }

    #[test]
    fn normalize_label_eszett() {
        assert_eq!(normalize_link_label("ẞ".as_bytes()), "ss");
    }
}
