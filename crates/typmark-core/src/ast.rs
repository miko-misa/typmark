use crate::span::Span;

pub type InlineSeq = Vec<Inline>;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct NodeId(pub u32);

#[derive(Clone, Debug, PartialEq)]
pub struct Document {
    pub span: Span,
    pub settings: Option<AttrList>,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub span: Span,
    pub attrs: AttrList,
    pub kind: BlockKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BlockKind {
    Paragraph {
        content: InlineSeq,
    },
    Heading {
        level: u8,
        title: InlineSeq,
    },
    Section {
        level: u8,
        title: InlineSeq,
        label: Option<Label>,
        children: Vec<Block>,
    },
    List(List),
    BlockQuote {
        blocks: Vec<Block>,
    },
    CodeBlock(CodeBlock),
    Box(BoxBlock),
    MathBlock {
        typst_src: String,
    },
    ThematicBreak,
    HtmlBlock {
        raw: String,
    },
    Table(Table),
}

#[derive(Clone, Debug, PartialEq)]
pub struct List {
    pub ordered: bool,
    pub start: Option<u64>,
    pub tight: bool,
    pub items: Vec<ListItem>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    pub span: Span,
    pub blocks: Vec<Block>,
    pub task: Option<bool>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub headers: Vec<InlineSeq>,
    pub aligns: Vec<TableAlign>,
    pub rows: Vec<Vec<InlineSeq>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TableAlign {
    None,
    Left,
    Center,
    Right,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodeBlock {
    pub kind: CodeBlockKind,
    pub lang: Option<String>,
    pub info_attrs: AttrList,
    pub meta: CodeMeta,
    pub text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeBlockKind {
    Fenced,
    Indented,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CodeMeta {
    // Line numbers are 1-based and include blank lines.
    pub hl: Vec<LineRange>,
    pub diff_add: Vec<LineRange>,
    pub diff_del: Vec<LineRange>,
    pub line_labels: Vec<LineLabel>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LineRange {
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineLabel {
    pub line: u32,
    pub label: Label,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoxBlock {
    pub title: Option<InlineSeq>,
    pub blocks: Vec<Block>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Inline {
    pub span: Span,
    pub kind: InlineKind,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InlineKind {
    Text(String),
    Emph(InlineSeq),
    Strong(InlineSeq),
    Strikethrough(InlineSeq),
    CodeSpan(String),
    SoftBreak,
    HardBreak,
    Link {
        url: String,
        title: Option<String>,
        children: InlineSeq,
    },
    Image {
        url: String,
        title: Option<String>,
        alt: InlineSeq,
    },
    LinkRef {
        label: String,
        children: InlineSeq,
        meta: LinkRefMeta,
    },
    ImageRef {
        label: String,
        alt: InlineSeq,
        meta: LinkRefMeta,
    },
    Ref {
        label: Label,
        bracket: Option<InlineSeq>,
        resolved: Option<ResolvedRef>,
    },
    MathInline {
        typst_src: String,
    },
    HtmlSpan {
        raw: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinkDefinition {
    pub url: String,
    pub title: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LinkRefMeta {
    pub opener_span: Span,
    pub closer_span: Span,
    pub label_open_span: Option<Span>,
    pub label_span: Option<Span>,
    pub label_close_span: Option<Span>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ResolvedRef {
    Block {
        label: String,
        display: Option<InlineSeq>,
    },
    CodeLine {
        label: String,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttrList {
    pub span: Option<Span>,
    pub label: Option<Label>,
    pub items: Vec<AttrItem>,
}

impl AttrList {
    pub fn empty() -> Self {
        Self {
            span: None,
            label: None,
            items: Vec::new(),
        }
    }
}

impl Default for AttrList {
    fn default() -> Self {
        Self::empty()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttrItem {
    pub key: String,
    pub value: AttrValue,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AttrValue {
    pub raw: String,
    pub span: Span,
    pub quoted: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Label {
    pub name: String,
    pub span: Span,
}
