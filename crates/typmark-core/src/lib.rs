mod ast;
mod diagnostic;
mod emit;
mod entities;
mod math;
mod parser;
mod resolver;
mod section;
mod source_map;
mod span;

pub use ast::{
    AttrItem, AttrList, AttrValue, Block, BlockKind, BoxBlock, CodeBlock, CodeMeta, Document,
    Inline, InlineKind, InlineSeq, Label, LineLabel, LineRange, List, ListItem, NodeId,
    ResolvedRef,
};
pub use diagnostic::{
    Diagnostic, DiagnosticSeverity, E_ATTR_SYNTAX, E_CODE_CONFLICT, E_LABEL_DUP, E_MATH_INLINE_NL,
    E_REF_BRACKET_NL, E_REF_DEPTH, E_REF_OMIT, E_REF_SELF_TITLE, E_TARGET_ORPHAN,
    RelatedDiagnostic, W_BOX_STYLE_INVALID, W_CODE_RANGE_OOB, W_REF_MISSING,
};
pub use emit::{emit_html, emit_html_sanitized};
pub use parser::{ParseResult, parse};
pub use resolver::{ResolveResult, resolve};
pub use source_map::{Position, Range, SourceMap};
pub use span::{Span, SpanError};
