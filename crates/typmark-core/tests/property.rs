use std::panic;

use typmark_core::{
    AttrList, Block, BlockKind, CodeBlock, Document, Inline, InlineKind, Label, List, Span, parse,
    resolve,
};

const CASES: usize = 200;
const MAX_LEN: usize = 512;
const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 \
\n\t#@*`$[](){}!<>:+-_=./\\\\\"";

#[test]
fn parser_never_panics_on_random_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = Lcg::new(0x7f4a_2d91_13b4_55a1);
    for case in 0..CASES {
        let len = rng.gen_range(0, MAX_LEN + 1);
        let source = random_string(&mut rng, len);
        let result = panic::catch_unwind(|| parse(&source));
        if result.is_err() {
            return Err(format!("parse panicked for case {}: {:?}", case, source).into());
        }
    }
    Ok(())
}

#[test]
fn spans_are_in_bounds_on_random_input() -> Result<(), Box<dyn std::error::Error>> {
    let mut rng = Lcg::new(0x91d4_2f8e_c1a3_044f);
    for case in 0..CASES {
        let len = rng.gen_range(0, MAX_LEN + 1);
        let source = random_string(&mut rng, len);
        let parsed = parse(&source);
        let resolved = resolve(
            parsed.document,
            &source,
            &parsed.source_map,
            parsed.diagnostics,
            &parsed.link_defs,
        );
        if let Err(message) = check_document_spans(&resolved.document, source.len()) {
            return Err(format!(
                "span check failed for case {}: {}\nSource:\n---\n{}\n---",
                case, message, source
            )
            .into());
        }
    }
    Ok(())
}

fn check_document_spans(document: &Document, source_len: usize) -> Result<(), String> {
    check_span(document.span, source_len, "document")?;
    check_block_seq(
        &document.blocks,
        document.span,
        source_len,
        "document.blocks",
    )?;
    Ok(())
}

fn check_block_seq(
    blocks: &[Block],
    parent: Span,
    source_len: usize,
    context: &str,
) -> Result<(), String> {
    let mut prev_end = parent.start;
    for (idx, block) in blocks.iter().enumerate() {
        let label = format!("{}[{}]", context, idx);
        check_span(block.span, source_len, &label)?;
        if block.span.start < parent.start || block.span.end > parent.end {
            return Err(format!(
                "{} span {:?} not within parent {:?}",
                label, block.span, parent
            ));
        }
        if block.span.start < prev_end {
            return Err(format!(
                "{} span {:?} overlaps previous end {}",
                label, block.span, prev_end
            ));
        }
        prev_end = block.span.end;
        check_block(block, source_len, &label)?;
    }
    Ok(())
}

fn check_block(block: &Block, source_len: usize, context: &str) -> Result<(), String> {
    check_attr_list(&block.attrs, source_len, &format!("{}.attrs", context))?;
    match &block.kind {
        BlockKind::Paragraph { content } => check_inline_seq(
            content,
            block.span,
            source_len,
            &format!("{}.paragraph", context),
        )?,
        BlockKind::Heading { title, .. } => check_inline_seq(
            title,
            block.span,
            source_len,
            &format!("{}.heading", context),
        )?,
        BlockKind::Section {
            title, children, ..
        } => {
            check_inline_seq(
                title,
                block.span,
                source_len,
                &format!("{}.section.title", context),
            )?;
            check_block_seq(
                children,
                block.span,
                source_len,
                &format!("{}.section.children", context),
            )?;
        }
        BlockKind::List(List { items, .. }) => {
            for (idx, item) in items.iter().enumerate() {
                let label = format!("{}.list.items[{}]", context, idx);
                check_span(item.span, source_len, &label)?;
                if item.span.start < block.span.start || item.span.end > block.span.end {
                    return Err(format!(
                        "{} span {:?} not within list {:?}",
                        label, item.span, block.span
                    ));
                }
                check_block_seq(
                    &item.blocks,
                    item.span,
                    source_len,
                    &format!("{}.blocks", label),
                )?;
            }
        }
        BlockKind::BlockQuote { blocks } => {
            check_block_seq(
                blocks,
                block.span,
                source_len,
                &format!("{}.blockquote", context),
            )?;
        }
        BlockKind::Box(box_block) => {
            if let Some(title) = &box_block.title {
                check_inline_seq(
                    title,
                    block.span,
                    source_len,
                    &format!("{}.box.title", context),
                )?;
            }
            check_block_seq(
                &box_block.blocks,
                block.span,
                source_len,
                &format!("{}.box.blocks", context),
            )?;
        }
        BlockKind::CodeBlock(CodeBlock {
            info_attrs, meta, ..
        }) => {
            check_attr_list(
                info_attrs,
                source_len,
                &format!("{}.code.info_attrs", context),
            )?;
            for (idx, line_label) in meta.line_labels.iter().enumerate() {
                check_label(
                    &line_label.label,
                    source_len,
                    &format!("{}.code.line_labels[{}]", context, idx),
                )?;
            }
        }
        BlockKind::Table(table) => {
            for (idx, header) in table.headers.iter().enumerate() {
                check_inline_seq(
                    header,
                    block.span,
                    source_len,
                    &format!("{}.table.headers[{}]", context, idx),
                )?;
            }
            for (row_idx, row) in table.rows.iter().enumerate() {
                for (col_idx, cell) in row.iter().enumerate() {
                    check_inline_seq(
                        cell,
                        block.span,
                        source_len,
                        &format!("{}.table.rows[{}][{}]", context, row_idx, col_idx),
                    )?;
                }
            }
        }
        BlockKind::MathBlock { .. } | BlockKind::ThematicBreak | BlockKind::HtmlBlock { .. } => {}
    }
    Ok(())
}

fn check_inline_seq(
    inlines: &[Inline],
    parent: Span,
    source_len: usize,
    context: &str,
) -> Result<(), String> {
    let mut prev_end = parent.start;
    for (idx, inline) in inlines.iter().enumerate() {
        let label = format!("{}[{}]", context, idx);
        check_span(inline.span, source_len, &label)?;
        if inline.span.start < parent.start || inline.span.end > parent.end {
            return Err(format!(
                "{} span {:?} not within parent {:?}",
                label, inline.span, parent
            ));
        }
        if inline.span.start < prev_end {
            return Err(format!(
                "{} span {:?} overlaps previous end {}",
                label, inline.span, prev_end
            ));
        }
        prev_end = inline.span.end;
        check_inline(inline, source_len, &label)?;
    }
    Ok(())
}

fn check_inline(inline: &Inline, source_len: usize, context: &str) -> Result<(), String> {
    match &inline.kind {
        InlineKind::Emph(children)
        | InlineKind::Strong(children)
        | InlineKind::Strikethrough(children) => check_inline_seq(
            children,
            inline.span,
            source_len,
            &format!("{}.children", context),
        )?,
        InlineKind::Link { children, .. } | InlineKind::LinkRef { children, .. } => {
            check_inline_seq(
                children,
                inline.span,
                source_len,
                &format!("{}.link.children", context),
            )?
        }
        InlineKind::Image { alt, .. } | InlineKind::ImageRef { alt, .. } => check_inline_seq(
            alt,
            inline.span,
            source_len,
            &format!("{}.image.alt", context),
        )?,
        InlineKind::Ref { label, bracket, .. } => {
            check_label(label, source_len, &format!("{}.ref.label", context))?;
            if let Some(bracket) = bracket {
                check_inline_seq(
                    bracket,
                    inline.span,
                    source_len,
                    &format!("{}.ref.bracket", context),
                )?;
            }
        }
        InlineKind::Text(_)
        | InlineKind::CodeSpan(_)
        | InlineKind::SoftBreak
        | InlineKind::HardBreak
        | InlineKind::MathInline { .. }
        | InlineKind::HtmlSpan { .. } => {}
    }
    Ok(())
}

fn check_attr_list(attr: &AttrList, source_len: usize, context: &str) -> Result<(), String> {
    if let Some(span) = attr.span {
        check_span(span, source_len, &format!("{}.span", context))?;
    }
    if let Some(label) = &attr.label {
        check_label(label, source_len, &format!("{}.label", context))?;
    }
    for (idx, item) in attr.items.iter().enumerate() {
        let label = format!("{}.items[{}]", context, idx);
        check_span(item.value.span, source_len, &format!("{}.value", label))?;
    }
    Ok(())
}

fn check_label(label: &Label, source_len: usize, context: &str) -> Result<(), String> {
    check_span(label.span, source_len, context)
}

fn check_span(span: Span, source_len: usize, context: &str) -> Result<(), String> {
    if span.start > span.end {
        return Err(format!("{} inverted span {:?}", context, span));
    }
    if span.end > source_len {
        return Err(format!(
            "{} span {:?} out of bounds (len={})",
            context, span, source_len
        ));
    }
    Ok(())
}

fn random_string(rng: &mut Lcg, len: usize) -> String {
    let mut out = String::with_capacity(len);
    for _ in 0..len {
        let idx = rng.gen_range(0, CHARSET.len());
        let byte = CHARSET.get(idx).copied().unwrap_or(b' ');
        out.push(byte as char);
    }
    out
}

struct Lcg {
    state: u64,
}

impl Lcg {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn gen_range(&mut self, min: usize, max: usize) -> usize {
        if max <= min {
            return min;
        }
        let span = max - min;
        let value = (self.next() >> 1) as usize;
        min + (value % span)
    }
}
