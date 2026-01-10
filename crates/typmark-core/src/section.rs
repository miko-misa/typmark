use crate::ast::{Block, BlockKind, BoxBlock, List};
use crate::span::Span;

pub fn build_sections(blocks: Vec<Block>) -> Vec<Block> {
    let mut iter = blocks.into_iter().peekable();
    let mut out = Vec::new();

    while let Some(block) = iter.next() {
        if let BlockKind::Heading { level, title } = block.kind {
            // Group following blocks until the next heading of the same/higher level.
            let mut children = Vec::new();
            while let Some(next) = iter.peek() {
                if let Some(next_level) = heading_level(next) {
                    if next_level <= level {
                        break;
                    }
                }
                if let Some(child) = iter.next() {
                    children.push(child);
                }
            }
            let children = build_sections(children);
            let end = children
                .last()
                .map(|child| child.span.end)
                .unwrap_or(block.span.end);
            let span = Span {
                start: block.span.start,
                end,
            };
            let label = block.attrs.label.clone();
            out.push(Block {
                span,
                attrs: block.attrs,
                kind: BlockKind::Section {
                    level,
                    title,
                    label,
                    children,
                },
            });
            continue;
        }

        out.push(rewrite_block(block));
    }

    out
}

fn rewrite_block(mut block: Block) -> Block {
    match &mut block.kind {
        BlockKind::List(List { items, .. }) => {
            for item in items {
                item.blocks = build_sections(std::mem::take(&mut item.blocks));
            }
        }
        BlockKind::BlockQuote { blocks } => {
            *blocks = build_sections(std::mem::take(blocks));
        }
        BlockKind::Box(BoxBlock { blocks, .. }) => {
            *blocks = build_sections(std::mem::take(blocks));
        }
        BlockKind::Section { children, .. } => {
            *children = build_sections(std::mem::take(children));
        }
        _ => {}
    }
    block
}

fn heading_level(block: &Block) -> Option<u8> {
    if let BlockKind::Heading { level, .. } = block.kind {
        Some(level)
    } else {
        None
    }
}
