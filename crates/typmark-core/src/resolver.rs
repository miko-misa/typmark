use std::collections::{HashMap, HashSet};

use crate::ast::{
    Block, BlockKind, BoxBlock, Document, Inline, InlineKind, InlineSeq, Label, LinkDefinition,
    LinkRefMeta, List, ResolvedRef,
};
use crate::diagnostic::{
    Diagnostic, DiagnosticSeverity, E_LABEL_DUP, E_REF_DEPTH, E_REF_OMIT, E_REF_SELF_TITLE,
    W_REF_MISSING,
};
use crate::label::{normalize_link_label, unescape_backslash_punct};
use crate::section::build_sections;
use crate::source_map::SourceMap;
use crate::span::Span;

pub struct ResolveResult {
    pub document: Document,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Clone)]
struct LabelInfo {
    span: Span,
    kind: LabelKind,
    title: Option<Vec<Inline>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LabelKind {
    Title,
    Block,
    CodeLine,
}

pub fn resolve(
    document: Document,
    source: &str,
    source_map: &SourceMap,
    mut diagnostics: Vec<Diagnostic>,
    link_defs: &HashMap<String, LinkDefinition>,
) -> ResolveResult {
    let mut document = document;
    // First, resolve CommonMark-style link references like [text][label].
    resolve_link_refs(&mut document, source, source_map, link_defs);

    // Then, build the section tree for TypMark-style header/section linking.
    document.blocks = build_sections(document.blocks);

    let mut labels = HashMap::new();
    collect_labels(&document.blocks, &mut labels, &mut diagnostics, source_map);
    check_self_reference_titles(&document.blocks, &mut diagnostics, source_map);
    resolve_refs(&mut document.blocks, &labels, &mut diagnostics, source_map);

    ResolveResult {
        document,
        diagnostics,
    }
}

fn resolve_link_refs(
    document: &mut Document,
    source: &str,
    source_map: &SourceMap,
    link_defs: &HashMap<String, LinkDefinition>,
) {
    resolve_link_refs_in_blocks(&mut document.blocks, source, source_map, link_defs);
}

fn resolve_link_refs_in_blocks(
    blocks: &mut [Block],
    source: &str,
    source_map: &SourceMap,
    link_defs: &HashMap<String, LinkDefinition>,
) {
    for block in blocks {
        match &mut block.kind {
            BlockKind::Paragraph { content } => {
                resolve_link_refs_inlines(content, source, source_map, link_defs);
            }
            BlockKind::Heading { title, .. } => {
                resolve_link_refs_inlines(title, source, source_map, link_defs);
            }
            BlockKind::Section {
                title, children, ..
            } => {
                resolve_link_refs_inlines(title, source, source_map, link_defs);
                resolve_link_refs_in_blocks(children, source, source_map, link_defs);
            }
            BlockKind::BlockQuote { blocks } => {
                resolve_link_refs_in_blocks(blocks, source, source_map, link_defs);
            }
            BlockKind::List(List { items, .. }) => {
                for item in items {
                    resolve_link_refs_in_blocks(&mut item.blocks, source, source_map, link_defs);
                }
            }
            BlockKind::Box(BoxBlock { title, blocks }) => {
                if let Some(title) = title.as_mut() {
                    resolve_link_refs_inlines(title, source, source_map, link_defs);
                }
                resolve_link_refs_in_blocks(blocks, source, source_map, link_defs);
            }
            _ => {}
        }
    }
}

fn resolve_link_refs_inlines(
    inlines: &mut InlineSeq,
    source: &str,
    source_map: &SourceMap,
    link_defs: &HashMap<String, LinkDefinition>,
) {
    let mut idx = 0;
    while idx < inlines.len() {
        let mut replace = None;
        {
            let inline = &mut inlines[idx];
            match &mut inline.kind {
                InlineKind::LinkRef {
                    label,
                    children,
                    meta,
                } => {
                    resolve_link_refs_inlines(children, source, source_map, link_defs);
                    let normalized_label = normalize_link_label(label.as_bytes());
                    if let Some(def) = link_defs.get(&normalized_label) {
                        let children = std::mem::take(children);
                        let url = def.url.clone();
                        let title = def.title.clone();
                        inline.kind = InlineKind::Link {
                            url,
                            title,
                            children,
                        };
                    } else {
                        let children = std::mem::take(children);
                        replace = Some(build_link_ref_fallback(meta, children, false, source));
                    }
                }
                InlineKind::ImageRef { label, alt, meta } => {
                    resolve_link_refs_inlines(alt, source, source_map, link_defs);
                    let normalized_label = normalize_link_label(label.as_bytes());
                    if let Some(def) = link_defs.get(&normalized_label) {
                        let alt = std::mem::take(alt);
                        let url = def.url.clone();
                        let title = def.title.clone();
                        inline.kind = InlineKind::Image { url, title, alt };
                    } else {
                        let alt = std::mem::take(alt);
                        replace = Some(build_link_ref_fallback(meta, alt, true, source));
                    }
                }
                InlineKind::Emph(children) | InlineKind::Strong(children) => {
                    resolve_link_refs_inlines(children, source, source_map, link_defs);
                }
                InlineKind::Link { children, .. } => {
                    resolve_link_refs_inlines(children, source, source_map, link_defs);
                }
                InlineKind::Image { alt, .. } => {
                    resolve_link_refs_inlines(alt, source, source_map, link_defs);
                }
                InlineKind::Ref { bracket, .. } => {
                    if let Some(bracket) = bracket.as_mut() {
                        resolve_link_refs_inlines(bracket, source, source_map, link_defs);
                    }
                }
                _ => {}
            }
        }

        if let Some(replacement) = replace {
            inlines.splice(idx..idx + 1, replacement);
            continue;
        }
        idx += 1;
    }
}

fn build_link_ref_fallback(
    meta: &LinkRefMeta,
    mut children: InlineSeq,
    image: bool,
    source: &str,
) -> InlineSeq {
    let mut out = Vec::new();
    let opener_text = if image { "![" } else { "[" };
    out.push(Inline {
        span: meta.opener_span,
        kind: InlineKind::Text(opener_text.to_string()),
    });
    out.append(&mut children);
    out.push(Inline {
        span: meta.closer_span,
        kind: InlineKind::Text("]".to_string()),
    });
    if let (Some(open_span), Some(close_span)) = (meta.label_open_span, meta.label_close_span) {
        out.push(Inline {
            span: open_span,
            kind: InlineKind::Text("[".to_string()),
        });
        if let Some(label_span) = meta.label_span {
            let label_text = source[label_span.start..label_span.end].to_string();
            let label_text = unescape_backslash_punct(&label_text);
            out.push(Inline {
                span: label_span,
                kind: InlineKind::Text(label_text),
            });
        }
        out.push(Inline {
            span: close_span,
            kind: InlineKind::Text("]".to_string()),
        });
    }
    out
}

fn collect_labels(
    blocks: &[Block],
    labels: &mut HashMap<String, LabelInfo>,
    diagnostics: &mut Vec<Diagnostic>,
    source_map: &SourceMap,
) {
    // Build a label table for blocks and code-line labels, reporting duplicates.
    for block in blocks {
        if let Some(label) = block.attrs.label.as_ref() {
            let (kind, title) = match &block.kind {
                BlockKind::Section { title, .. } => (LabelKind::Title, Some(title.clone())),
                BlockKind::Box(BoxBlock { title, .. }) if title.is_some() => {
                    (LabelKind::Title, title.clone())
                }
                _ => (LabelKind::Block, None),
            };
            insert_label(labels, label, kind, title, diagnostics, source_map);
        }

        if let BlockKind::CodeBlock(code_block) = &block.kind {
            for line_label in &code_block.meta.line_labels {
                insert_label(
                    labels,
                    &line_label.label,
                    LabelKind::CodeLine,
                    None,
                    diagnostics,
                    source_map,
                );
            }
        }

        match &block.kind {
            BlockKind::List(List { items, .. }) => {
                for item in items {
                    collect_labels(&item.blocks, labels, diagnostics, source_map);
                }
            }
            BlockKind::BlockQuote { blocks } => {
                collect_labels(blocks, labels, diagnostics, source_map);
            }
            BlockKind::Box(BoxBlock { blocks, .. }) => {
                collect_labels(blocks, labels, diagnostics, source_map);
            }
            BlockKind::Section { children, .. } => {
                collect_labels(children, labels, diagnostics, source_map);
            }
            _ => {}
        }
    }
}

fn insert_label(
    labels: &mut HashMap<String, LabelInfo>,
    label: &Label,
    kind: LabelKind,
    title: Option<Vec<Inline>>,
    diagnostics: &mut Vec<Diagnostic>,
    source_map: &SourceMap,
) {
    if let Some(existing) = labels.get(&label.name) {
        let mut diag = Diagnostic::new(
            source_map.range(label.span),
            DiagnosticSeverity::Error,
            E_LABEL_DUP,
            "duplicate label",
        );
        diag.related.push(crate::diagnostic::RelatedDiagnostic::new(
            source_map.range(existing.span),
            None,
        ));
        diagnostics.push(diag);
        return;
    }
    labels.insert(
        label.name.clone(),
        LabelInfo {
            span: label.span,
            kind,
            title,
        },
    );
}

fn check_self_reference_titles(
    blocks: &[Block],
    diagnostics: &mut Vec<Diagnostic>,
    source_map: &SourceMap,
) {
    for block in blocks {
        match &block.kind {
            BlockKind::Section { title, label, .. } => {
                if let Some(label) = label {
                    if let Some(span) = find_self_ref(title, &label.name) {
                        diagnostics.push(Diagnostic::new(
                            source_map.range(span),
                            DiagnosticSeverity::Error,
                            E_REF_SELF_TITLE,
                            "self-reference in title",
                        ));
                    }
                }
            }
            BlockKind::Box(BoxBlock {
                title: Some(title), ..
            }) => {
                if let Some(label) = block.attrs.label.as_ref() {
                    if let Some(span) = find_self_ref(title, &label.name) {
                        diagnostics.push(Diagnostic::new(
                            source_map.range(span),
                            DiagnosticSeverity::Error,
                            E_REF_SELF_TITLE,
                            "self-reference in title",
                        ));
                    }
                }
            }
            _ => {}
        }

        match &block.kind {
            BlockKind::List(List { items, .. }) => {
                for item in items {
                    check_self_reference_titles(&item.blocks, diagnostics, source_map);
                }
            }
            BlockKind::BlockQuote { blocks } => {
                check_self_reference_titles(blocks, diagnostics, source_map);
            }
            BlockKind::Box(BoxBlock { blocks, .. }) => {
                check_self_reference_titles(blocks, diagnostics, source_map);
            }
            BlockKind::Section { children, .. } => {
                check_self_reference_titles(children, diagnostics, source_map);
            }
            _ => {}
        }
    }
}

fn find_self_ref(inlines: &[Inline], label: &str) -> Option<Span> {
    for inline in inlines {
        match &inline.kind {
            InlineKind::Ref {
                label: ref_label,
                bracket,
                ..
            } => {
                if ref_label.name == label {
                    return Some(inline.span);
                }
                if let Some(bracket) = bracket {
                    if let Some(span) = find_self_ref(bracket, label) {
                        return Some(span);
                    }
                }
            }
            InlineKind::Emph(children) | InlineKind::Strong(children) => {
                if let Some(span) = find_self_ref(children, label) {
                    return Some(span);
                }
            }
            InlineKind::Link { children, .. } | InlineKind::LinkRef { children, .. } => {
                if let Some(span) = find_self_ref(children, label) {
                    return Some(span);
                }
            }
            InlineKind::Image { alt, .. } | InlineKind::ImageRef { alt, .. } => {
                if let Some(span) = find_self_ref(alt, label) {
                    return Some(span);
                }
            }
            _ => {}
        }
    }
    None
}

fn resolve_refs(
    blocks: &mut [Block],
    labels: &HashMap<String, LabelInfo>,
    diagnostics: &mut Vec<Diagnostic>,
    source_map: &SourceMap,
) {
    for block in blocks {
        match &mut block.kind {
            BlockKind::Paragraph { content } => {
                resolve_inlines(content, labels, diagnostics, source_map);
            }
            BlockKind::Heading { title, .. } => {
                resolve_inlines(title, labels, diagnostics, source_map);
            }
            BlockKind::Section {
                title, children, ..
            } => {
                resolve_inlines(title, labels, diagnostics, source_map);
                resolve_refs(children, labels, diagnostics, source_map);
            }
            BlockKind::BlockQuote { blocks } => {
                resolve_refs(blocks, labels, diagnostics, source_map);
            }
            BlockKind::List(List { items, .. }) => {
                for item in items {
                    resolve_refs(&mut item.blocks, labels, diagnostics, source_map);
                }
            }
            BlockKind::Box(BoxBlock { title, blocks }) => {
                if let Some(title) = title.as_mut() {
                    resolve_inlines(title, labels, diagnostics, source_map);
                }
                resolve_refs(blocks, labels, diagnostics, source_map);
            }
            _ => {}
        }
    }
}

fn resolve_inlines(
    inlines: &mut [Inline],
    labels: &HashMap<String, LabelInfo>,
    diagnostics: &mut Vec<Diagnostic>,
    source_map: &SourceMap,
) {
    for inline in inlines {
        match &mut inline.kind {
            InlineKind::Ref {
                label,
                bracket,
                resolved,
            } => {
                let info = match labels.get(&label.name) {
                    Some(info) => info,
                    None => {
                        diagnostics.push(Diagnostic::new(
                            source_map.range(inline.span),
                            DiagnosticSeverity::Warning,
                            W_REF_MISSING,
                            "reference target not found",
                        ));
                        continue;
                    }
                };

                if bracket.is_none() && info.kind != LabelKind::Title {
                    diagnostics.push(Diagnostic::new(
                        source_map.range(inline.span),
                        DiagnosticSeverity::Error,
                        E_REF_OMIT,
                        "missing reference text for non-title target",
                    ));
                }

                let mut display = None;
                if bracket.is_none() && info.kind == LabelKind::Title {
                    let (text, exceeded) = build_reference_text(&label.name, labels, info.span);
                    display = Some(text);
                    if exceeded {
                        diagnostics.push(Diagnostic::new(
                            source_map.range(inline.span),
                            DiagnosticSeverity::Error,
                            E_REF_DEPTH,
                            "reference display text depth exceeded",
                        ));
                    }
                }

                *resolved = Some(match info.kind {
                    LabelKind::CodeLine => ResolvedRef::CodeLine {
                        label: label.name.clone(),
                    },
                    _ => ResolvedRef::Block {
                        label: label.name.clone(),
                        display,
                    },
                });
            }
            InlineKind::Emph(children)
            | InlineKind::Strong(children)
            | InlineKind::Strikethrough(children) => {
                resolve_inlines(children, labels, diagnostics, source_map);
            }
            // LinkRef is already resolved, so we only need to recurse.
            InlineKind::Link { children, .. } | InlineKind::LinkRef { children, .. } => {
                resolve_inlines(children, labels, diagnostics, source_map);
            }
            InlineKind::Image { alt, .. } | InlineKind::ImageRef { alt, .. } => {
                resolve_inlines(alt, labels, diagnostics, source_map);
            }
            _ => {}
        }
    }
}

fn build_reference_text(
    label: &str,
    labels: &HashMap<String, LabelInfo>,
    fallback_span: Span,
) -> (Vec<Inline>, bool) {
    let mut visited = HashSet::new();
    build_reference_text_inner(label, labels, 0, &mut visited, fallback_span)
}

fn build_reference_text_inner(
    label: &str,
    labels: &HashMap<String, LabelInfo>,
    depth: usize,
    visited: &mut HashSet<String>,
    fallback_span: Span,
) -> (Vec<Inline>, bool) {
    let span = labels
        .get(label)
        .map(|info| info.span)
        .unwrap_or(fallback_span);
    if depth > 100 {
        return (vec![text_inline(span, label)], true);
    }
    if !visited.insert(label.to_string()) {
        return (vec![text_inline(span, label)], true);
    }
    let title = match labels.get(label).and_then(|info| info.title.as_ref()) {
        Some(title) => title,
        None => {
            visited.remove(label);
            return (vec![text_inline(span, label)], false);
        }
    };
    let (result, exceeded) = build_reference_text_from_inlines(title, labels, depth + 1, visited);
    visited.remove(label);
    (result, exceeded)
}

fn build_reference_text_from_inlines(
    inlines: &[Inline],
    labels: &HashMap<String, LabelInfo>,
    depth: usize,
    visited: &mut HashSet<String>,
) -> (Vec<Inline>, bool) {
    let mut out = Vec::new();
    let mut exceeded = false;
    for inline in inlines {
        match &inline.kind {
            InlineKind::Text(_) | InlineKind::CodeSpan(_) | InlineKind::MathInline { .. } => {
                out.push(inline.clone());
            }
            InlineKind::SoftBreak | InlineKind::HardBreak => {
                out.push(inline.clone());
            }
            InlineKind::Emph(children) => {
                let (inner, inner_exceeded) =
                    build_reference_text_from_inlines(children, labels, depth, visited);
                exceeded |= inner_exceeded;
                out.push(Inline {
                    span: inline.span,
                    kind: InlineKind::Emph(inner),
                });
            }
            InlineKind::Strikethrough(children) => {
                let (inner, inner_exceeded) =
                    build_reference_text_from_inlines(children, labels, depth, visited);
                exceeded |= inner_exceeded;
                out.push(Inline {
                    span: inline.span,
                    kind: InlineKind::Strikethrough(inner),
                });
            }
            InlineKind::Strong(children) => {
                let (inner, inner_exceeded) =
                    build_reference_text_from_inlines(children, labels, depth, visited);
                exceeded |= inner_exceeded;
                out.push(Inline {
                    span: inline.span,
                    kind: InlineKind::Strong(inner),
                });
            }
            InlineKind::Link { children, .. } | InlineKind::LinkRef { children, .. } => {
                let (inner, inner_exceeded) =
                    build_reference_text_from_inlines(children, labels, depth, visited);
                exceeded |= inner_exceeded;
                out.extend(inner);
            }
            InlineKind::Image { alt, .. } | InlineKind::ImageRef { alt, .. } => {
                let (inner, inner_exceeded) =
                    build_reference_text_from_inlines(alt, labels, depth, visited);
                exceeded |= inner_exceeded;
                out.extend(inner);
            }
            InlineKind::Ref { label, bracket, .. } => {
                if let Some(bracket) = bracket {
                    let (inner, inner_exceeded) =
                        build_reference_text_from_inlines(bracket, labels, depth, visited);
                    exceeded |= inner_exceeded;
                    out.extend(inner);
                } else {
                    let (inner, inner_exceeded) = build_reference_text_inner(
                        &label.name,
                        labels,
                        depth + 1,
                        visited,
                        inline.span,
                    );
                    exceeded |= inner_exceeded;
                    out.extend(inner);
                }
            }
            InlineKind::HtmlSpan { raw } => {
                out.push(Inline {
                    span: inline.span,
                    kind: InlineKind::Text(raw.clone()),
                });
            }
        }
    }
    (out, exceeded)
}

fn text_inline(span: Span, text: &str) -> Inline {
    Inline {
        span,
        kind: InlineKind::Text(text.to_string()),
    }
}
