use crate::source_map::Range;

pub const E_ATTR_SYNTAX: &str = "E_ATTR_SYNTAX";
pub const E_TARGET_ORPHAN: &str = "E_TARGET_ORPHAN";
pub const E_LABEL_DUP: &str = "E_LABEL_DUP";
pub const E_REF_OMIT: &str = "E_REF_OMIT";
pub const E_REF_BRACKET_NL: &str = "E_REF_BRACKET_NL";
pub const E_REF_SELF_TITLE: &str = "E_REF_SELF_TITLE";
pub const E_REF_DEPTH: &str = "E_REF_DEPTH";
pub const E_MATH_INLINE_NL: &str = "E_MATH_INLINE_NL";
pub const E_CODE_CONFLICT: &str = "E_CODE_CONFLICT";
pub const E_TYPST_RENDER: &str = "E_TYPST_RENDER";
pub const E_TYPST_RESOURCE_LIMIT: &str = "E_TYPST_RESOURCE_LIMIT";

pub const W_REF_MISSING: &str = "W_REF_MISSING";
pub const W_CODE_RANGE_OOB: &str = "W_CODE_RANGE_OOB";
pub const W_BOX_STYLE_INVALID: &str = "W_BOX_STYLE_INVALID";
pub const W_TYPST_ATTR_INVALID: &str = "W_TYPST_ATTR_INVALID";
pub const W_TYPST_EXTERNAL_ASSET: &str = "W_TYPST_EXTERNAL_ASSET";
pub const W_TYPST_MULTI_PAGE: &str = "W_TYPST_MULTI_PAGE";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub range: Range,
    pub severity: DiagnosticSeverity,
    pub code: &'static str,
    pub message: String,
    pub related: Vec<RelatedDiagnostic>,
}

impl Diagnostic {
    pub fn new(
        range: Range,
        severity: DiagnosticSeverity,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            range,
            severity,
            code,
            message: message.into(),
            related: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RelatedDiagnostic {
    pub range: Range,
    pub message: Option<String>,
}

impl RelatedDiagnostic {
    pub fn new(range: Range, message: Option<String>) -> Self {
        Self { range, message }
    }
}
