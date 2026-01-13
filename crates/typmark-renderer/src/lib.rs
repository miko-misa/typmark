use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::Path;
use syntect::easy::HighlightLines;
use syntect::highlighting::{Theme as SyntectTheme, ThemeSet};
use syntect::html::{IncludeBackground, styled_line_to_highlighted_html};
use syntect::parsing::{SyntaxReference, SyntaxSet};

const BASE_CSS: &str = include_str!("../assets/typmark.css");
const BASE_JS: &str = include_str!("../assets/typmark.js");

#[derive(Debug, Clone, Copy)]
pub enum Theme {
    Auto,
    Light,
    Dark,
}

#[derive(Debug, Clone)]
pub struct Renderer {
    theme: Theme,
    custom_vars: BTreeMap<String, String>,
}

impl Renderer {
    pub fn new(theme: Theme) -> Self {
        Self {
            theme,
            custom_vars: BTreeMap::new(),
        }
    }

    pub fn with_var(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.custom_vars.insert(key.into(), value.into());
        self
    }

    pub fn stylesheet(&self) -> String {
        let mut out = String::new();
        let (light_vars, dark_vars) = default_theme_vars();

        match self.theme {
            Theme::Auto => {
                out.push_str(&root_block(&light_vars, true));
                out.push_str("@media (prefers-color-scheme: dark) {\n");
                out.push_str(&indent_root_block(&dark_vars));
                out.push_str("}\n");
            }
            Theme::Light => {
                out.push_str(&root_block(&light_vars, true));
            }
            Theme::Dark => {
                out.push_str(&root_block(&dark_vars, true));
            }
        }

        if !self.custom_vars.is_empty() {
            out.push_str(&root_block(&self.custom_vars, false));
        }

        out.push_str(BASE_CSS);
        out
    }

    pub fn embed_html(&self, html: &str, with_inline_css: bool, with_inline_js: bool) -> String {
        let mut out = String::new();
        out.push_str("<!DOCTYPE html>\n");
        out.push_str("<html lang=\"en\">\n");
        out.push_str("<head>\n");
        out.push_str("  <meta charset=\"utf-8\" />\n");
        out.push_str("  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\" />\n");
        if with_inline_css {
            out.push_str("  <style>\n");
            out.push_str(&self.stylesheet());
            out.push_str("\n  </style>\n");
        }
        out.push_str("</head>\n");
        out.push_str("<body>\n");
        out.push_str(html);
        if !html.ends_with('\n') {
            out.push('\n');
        }
        if with_inline_js {
            out.push_str("  <script>\n");
            out.push_str(BASE_JS);
            out.push_str("\n  </script>\n");
        }
        out.push_str("</body>\n");
        out.push_str("</html>\n");
        out
    }

    pub fn generate_files(&self, out_dir: &Path) -> io::Result<()> {
        fs::create_dir_all(out_dir)?;
        fs::write(out_dir.join("typmark.css"), self.stylesheet())?;
        fs::write(out_dir.join("typmark.js"), BASE_JS)?;
        Ok(())
    }

    pub fn highlight_html(&self, html: &str) -> String {
        let syntax_set = SyntaxSet::load_defaults_newlines();
        let theme_set = ThemeSet::load_defaults();
        let theme = pick_theme(self.theme, &theme_set);
        highlight_html_inner(html, &syntax_set, theme)
    }
}

fn default_theme_vars() -> (BTreeMap<String, String>, BTreeMap<String, String>) {
    let light = BTreeMap::from([
        ("--typmark-bg".to_string(), "#fbfbf8".to_string()),
        ("--typmark-fg".to_string(), "#1f2328".to_string()),
        ("--typmark-muted".to_string(), "#5f6b76".to_string()),
        ("--typmark-border".to_string(), "#d8dee4".to_string()),
        ("--typmark-accent".to_string(), "#2b6cb0".to_string()),
        ("--typmark-code-bg".to_string(), "#f4f6f8".to_string()),
        ("--typmark-code-fg".to_string(), "#1f2328".to_string()),
        ("--typmark-box-bg".to_string(), "#f7f6f1".to_string()),
        ("--typmark-box-border".to_string(), "#c9c2b8".to_string()),
    ]);

    let dark = BTreeMap::from([
        ("--typmark-bg".to_string(), "#0e1116".to_string()),
        ("--typmark-fg".to_string(), "#e6edf3".to_string()),
        ("--typmark-muted".to_string(), "#9aa4af".to_string()),
        ("--typmark-border".to_string(), "#2a313b".to_string()),
        ("--typmark-accent".to_string(), "#63b3ed".to_string()),
        ("--typmark-code-bg".to_string(), "#202634".to_string()),
        ("--typmark-code-fg".to_string(), "#f0f6fc".to_string()),
        ("--typmark-box-bg".to_string(), "#1b212b".to_string()),
        ("--typmark-box-border".to_string(), "#2d3440".to_string()),
    ]);

    (light, dark)
}

fn format_vars(vars: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    for (key, value) in vars {
        out.push_str("  ");
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push_str(";\n");
    }
    out
}

fn root_block(vars: &BTreeMap<String, String>, include_color_scheme: bool) -> String {
    let mut out = String::new();
    out.push_str(":root {\n");
    if include_color_scheme {
        out.push_str("  color-scheme: light dark;\n");
    }
    out.push_str(&format_vars(vars));
    out.push_str("}\n");
    out
}

fn indent_root_block(vars: &BTreeMap<String, String>) -> String {
    let mut out = String::new();
    out.push_str("  :root {\n");
    out.push_str("    color-scheme: light dark;\n");
    for (key, value) in vars {
        out.push_str("    ");
        out.push_str(key);
        out.push_str(": ");
        out.push_str(value);
        out.push_str(";\n");
    }
    out.push_str("  }\n");
    out
}

fn pick_theme(theme: Theme, theme_set: &ThemeSet) -> &SyntectTheme {
    let candidates = match theme {
        Theme::Dark => ["Monokai Extended Bright", "Monokai Extended", "base16-ocean.dark"],
        Theme::Light => ["InspiredGitHub", "Solarized (light)", "base16-ocean.light"],
        Theme::Auto => ["InspiredGitHub", "Solarized (light)", "base16-ocean.light"],
    };
    for name in candidates {
        if let Some(found) = theme_set.themes.get(name) {
            return found;
        }
    }
    theme_set
        .themes
        .values()
        .next()
        .expect("theme set has at least one theme")
}

fn highlight_html_inner(html: &str, syntax_set: &SyntaxSet, theme: &SyntectTheme) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    let figure_tag = "<figure class=\"TypMark-codeblock\"";

    while let Some(start) = rest.find(figure_tag) {
        out.push_str(&rest[..start]);
        let after_start = &rest[start..];
        let end = match after_start.find("</figure>") {
            Some(index) => index + "</figure>".len(),
            None => {
                out.push_str(after_start);
                return out;
            }
        };
        let figure = &after_start[..end];
        out.push_str(&highlight_figure(figure, syntax_set, theme));
        rest = &after_start[end..];
    }

    out.push_str(rest);
    out
}

fn highlight_figure(figure: &str, syntax_set: &SyntaxSet, theme: &SyntectTheme) -> String {
    let code_start = match figure.find("<code") {
        Some(index) => index,
        None => return figure.to_string(),
    };
    let code_tag_end = match figure[code_start..].find('>') {
        Some(index) => code_start + index,
        None => return figure.to_string(),
    };
    let code_tag = &figure[code_start..=code_tag_end];
    let code_close = match figure[code_tag_end + 1..].find("</code>") {
        Some(index) => code_tag_end + 1 + index,
        None => return figure.to_string(),
    };
    let code_inner = &figure[code_tag_end + 1..code_close];

    let language = extract_language(code_tag);
    let syntax = language
        .as_deref()
        .and_then(|token| syntax_set.find_syntax_by_token(token))
        .unwrap_or_else(|| syntax_set.find_syntax_plain_text());
    let highlighted = highlight_code_lines(code_inner, syntax_set, syntax, theme);

    let mut out = String::with_capacity(figure.len() + highlighted.len());
    out.push_str(&figure[..code_tag_end + 1]);
    out.push_str(&highlighted);
    out.push_str(&figure[code_close..]);
    out
}

fn highlight_code_lines(
    code_html: &str,
    syntax_set: &SyntaxSet,
    syntax: &SyntaxReference,
    theme: &SyntectTheme,
) -> String {
    let mut out = String::with_capacity(code_html.len());
    let mut rest = code_html;
    let mut highlighter = HighlightLines::new(syntax, theme);

    while let Some(span_start) = rest.find("<span ") {
        out.push_str(&rest[..span_start]);
        let span_open_end = match rest[span_start..].find('>') {
            Some(index) => span_start + index,
            None => {
                out.push_str(&rest[span_start..]);
                return out;
            }
        };
        let span_open = &rest[span_start..=span_open_end];
        let content_start = span_open_end + 1;
        let close_tag = "</span>";
        let content_end = match rest[content_start..].find(close_tag) {
            Some(index) => content_start + index,
            None => {
                out.push_str(&rest[span_start..]);
                return out;
            }
        };
        let content = &rest[content_start..content_end];
        let line = unescape_html_code(content);
        let highlighted = highlight_line(&line, syntax_set, &mut highlighter);

        out.push_str(span_open);
        out.push_str(&highlighted);
        out.push_str(close_tag);
        rest = &rest[content_end + close_tag.len()..];
    }

    out.push_str(rest);
    out
}

fn highlight_line(
    line: &str,
    syntax_set: &SyntaxSet,
    highlighter: &mut HighlightLines,
) -> String {
    match highlighter.highlight_line(line, syntax_set) {
        Ok(ranges) => match styled_line_to_highlighted_html(&ranges, IncludeBackground::No) {
            Ok(html) => strip_font_weight(&html),
            Err(_) => escape_html_code(line),
        },
        Err(_) => escape_html_code(line),
    }
}

fn extract_language(code_tag: &str) -> Option<String> {
    let class_attr = extract_attr(code_tag, "class")?;
    for class_name in class_attr.split_whitespace() {
        if let Some(lang) = class_name.strip_prefix("language-") {
            if !lang.is_empty() {
                return Some(lang.to_string());
            }
        }
    }
    None
}

fn extract_attr(tag: &str, name: &str) -> Option<String> {
    let needle = format!("{}=\"", name);
    let start = tag.find(&needle)? + needle.len();
    let end = tag[start..].find('"')?;
    Some(tag[start..start + end].to_string())
}

fn escape_html_code(text: &str) -> String {
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

fn unescape_html_code(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(pos) = rest.find('&') {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos..];
        if let Some(stripped) = tail.strip_prefix("&amp;") {
            out.push('&');
            rest = stripped;
        } else if let Some(stripped) = tail.strip_prefix("&lt;") {
            out.push('<');
            rest = stripped;
        } else if let Some(stripped) = tail.strip_prefix("&gt;") {
            out.push('>');
            rest = stripped;
        } else if let Some(stripped) = tail.strip_prefix("&quot;") {
            out.push('"');
            rest = stripped;
        } else {
            out.push('&');
            rest = &tail[1..];
        }
    }
    out.push_str(rest);
    out
}

fn strip_font_weight(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(pos) = rest.find("font-weight:") {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos + "font-weight:".len()..];
        let end = match tail.find(';') {
            Some(index) => index + 1,
            None => {
                rest = "";
                break;
            }
        };
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod tests {
    use super::{Renderer, Theme};

    #[test]
    fn embed_html_includes_css_and_js() {
        let renderer = Renderer::new(Theme::Light);
        let html = renderer.embed_html("<p>Hi</p>", true, true);
        assert!(html.contains("<style>"));
        assert!(html.contains("<script>"));
        assert!(html.contains("<p>Hi</p>"));
    }

    #[test]
    fn embed_html_can_skip_assets() {
        let renderer = Renderer::new(Theme::Light);
        let html = renderer.embed_html("<p>Hi</p>", false, false);
        assert!(!html.contains("<style>"));
        assert!(!html.contains("<script>"));
        assert!(html.contains("<p>Hi</p>"));
    }

    #[test]
    fn highlight_preserves_line_wrappers() {
        let renderer = Renderer::new(Theme::Light);
        let html = "<figure class=\"TypMark-codeblock\" data-typmark=\"codeblock\"><pre class=\"TypMark-pre\"><code class=\"language-rust\"><span class=\"line\" data-line=\"1\">let x = 1;</span></code></pre></figure>";
        let highlighted = renderer.highlight_html(html);
        assert!(highlighted.contains("class=\"line\""));
        assert!(highlighted.contains("style=\""));
    }
}
