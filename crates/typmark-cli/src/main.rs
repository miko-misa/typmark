use std::env;
use std::fs;
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::process;

use typmark_core::{
    AttrList, Diagnostic, DiagnosticSeverity, HtmlEmitOptions,
    emit_html_document_sanitized_with_options, emit_html_document_with_options, parse, resolve,
};
use typmark_renderer::{PdfBackend, PdfMargin, PdfOptions, Renderer, Theme};

fn main() {
    let mut input: Option<String> = None;
    let mut sanitized = false;
    let mut simple_code_blocks = false;
    let mut wrap_sections = true;
    let mut diagnostics_mode: Option<DiagnosticsMode> = None;
    let mut render = true;
    let mut render_js = false;
    let mut theme = Theme::Dark;
    let mut pdf_output: Option<String> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                return;
            }
            "--version" => {
                println!("{}", env!("CARGO_PKG_VERSION"));
                return;
            }
            "--sanitized" => sanitized = true,
            "--simple-code" => simple_code_blocks = true,
            "--no-section-wrap" => wrap_sections = false,
            "--render" => render = true,
            "--render-js" => {
                render = true;
                render_js = true;
            }
            "--raw" => render = false,
            "--pdf" => {
                let value = match args.next() {
                    Some(value) => value,
                    None => {
                        eprintln!("--pdf expects an output file path");
                        print_usage();
                        process::exit(2);
                    }
                };
                pdf_output = Some(value);
            }
            "--theme" => {
                let value = args.next().unwrap_or_else(|| {
                    eprintln!("--theme expects: auto | light | dark");
                    print_usage();
                    process::exit(2);
                });
                theme = match value.as_str() {
                    "auto" => Theme::Auto,
                    "light" => Theme::Light,
                    "dark" => Theme::Dark,
                    _ => {
                        eprintln!("--theme expects: auto | light | dark");
                        print_usage();
                        process::exit(2);
                    }
                };
            }
            "--diagnostics" => {
                let mode = match args.next().as_deref() {
                    Some("json") => DiagnosticsMode::Json,
                    Some("pretty") => DiagnosticsMode::Pretty,
                    _ => {
                        eprintln!("--diagnostics expects: json | pretty");
                        print_usage();
                        process::exit(2);
                    }
                };
                diagnostics_mode = Some(mode);
            }
            _ => {
                if input.is_none() {
                    input = Some(arg);
                } else {
                    eprintln!("unexpected argument: {}", arg);
                    print_usage();
                    process::exit(2);
                }
            }
        }
    }

    let source = match input.as_deref() {
        Some(path) => fs::read_to_string(path).unwrap_or_else(|err| {
            eprintln!("failed to read {}: {}", path, err);
            process::exit(1);
        }),
        None => {
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .unwrap_or_else(|err| {
                    eprintln!("failed to read stdin: {}", err);
                    process::exit(1);
                });
            buffer
        }
    };

    let parsed = parse(&source);
    let resolved = resolve(
        parsed.document,
        &source,
        &parsed.source_map,
        parsed.diagnostics,
        &parsed.link_defs,
    );

    let options = HtmlEmitOptions {
        simple_code_blocks,
        wrap_sections,
    };

    if let Some(mode) = diagnostics_mode {
        emit_diagnostics(&resolved.diagnostics, mode);
    }

    let html = if sanitized {
        emit_html_document_sanitized_with_options(&resolved.document, &options)
    } else {
        emit_html_document_with_options(&resolved.document, &options)
    };

    if let Some(pdf_path) = pdf_output {
        let input_path = input.as_deref().map(Path::new);
        let output_path = Path::new(&pdf_path);
        let pdf_settings = match parse_pdf_settings(resolved.document.settings.as_ref()) {
            Ok(settings) => settings,
            Err(err) => {
                eprintln!("pdf settings error: {}", err);
                process::exit(1);
            }
        };
        let base_url = match resolve_pdf_base_url(&pdf_settings, input_path) {
            Ok(base_url) => base_url,
            Err(err) => {
                eprintln!("pdf settings error: {}", err);
                process::exit(1);
            }
        };
        let renderer = apply_renderer_settings(
            Renderer::new(Theme::Light),
            resolved.document.settings.as_ref(),
        );
        let mut options = PdfOptions::new(pdf_settings.backend);
        if let Some(page) = pdf_settings.page {
            options = options.with_page(page);
        }
        if let Some(margin) = pdf_settings.margin {
            options = options.with_margin(margin);
        }
        if let Some(scale) = pdf_settings.scale {
            options = options.with_scale(scale);
        }
        if let Some(base_url) = base_url {
            options = options.with_base_url(base_url);
        }
        if let Err(err) = renderer.export_pdf(&html, &options, output_path) {
            eprintln!("pdf export failed: {}", err);
            process::exit(1);
        }
    } else if render {
        let renderer =
            apply_renderer_settings(Renderer::new(theme), resolved.document.settings.as_ref());
        let highlighted = renderer.highlight_html(&html);
        let wrapped = renderer.embed_html(&highlighted, true, render_js);
        print!("{}", wrapped);
    } else {
        print!("{}", html);
    }

    if resolved
        .diagnostics
        .iter()
        .any(|diag| diag.severity == DiagnosticSeverity::Error)
    {
        process::exit(1);
    }
}

fn print_usage() {
    eprintln!(
        "Usage: typmark-cli [--version] [--sanitized] [--simple-code] [--no-section-wrap] [--render|--render-js|--raw] [--pdf output.pdf] [--theme auto|light|dark] [--diagnostics json|pretty] [input]"
    );
}

#[derive(Clone, Copy)]
enum DiagnosticsMode {
    Json,
    Pretty,
}

fn emit_diagnostics(diagnostics: &[Diagnostic], mode: DiagnosticsMode) {
    if diagnostics.is_empty() {
        if let DiagnosticsMode::Json = mode {
            eprintln!("[]");
        }
        return;
    }
    match mode {
        DiagnosticsMode::Json => {
            eprintln!("{}", diagnostics_to_json(diagnostics));
        }
        DiagnosticsMode::Pretty => {
            for diagnostic in diagnostics {
                eprintln!("{}", diagnostic_to_pretty(diagnostic));
            }
        }
    }
}

fn diagnostic_to_pretty(diagnostic: &Diagnostic) -> String {
    let severity = match diagnostic.severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
    };
    let start_line = diagnostic.range.start.line + 1;
    let start_col = diagnostic.range.start.character + 1;
    format!(
        "{}:{}:{} {} {}",
        start_line, start_col, severity, diagnostic.code, diagnostic.message
    )
}

fn apply_renderer_settings(renderer: Renderer, settings: Option<&AttrList>) -> Renderer {
    let mut renderer = renderer;
    let Some(settings) = settings else {
        return renderer;
    };
    for item in &settings.items {
        let value = item.value.raw.trim();
        if value.is_empty() {
            continue;
        }
        match item.key.as_str() {
            "font-size" => renderer = renderer.with_var("--typmark-font-size", value),
            "line-height" => renderer = renderer.with_var("--typmark-line-height", value),
            "font" => renderer = renderer.with_var("--typmark-font", value),
            "code-font" => renderer = renderer.with_var("--typmark-code-font", value),
            "code-size" => renderer = renderer.with_var("--typmark-code-size", value),
            "paragraph-gap" => renderer = renderer.with_var("--typmark-paragraph-gap", value),
            "page-width" => {
                let normalized = if value == "auto" { "none" } else { value };
                renderer = renderer.with_var("--typmark-page-width", normalized);
            }
            "image-max-width" => renderer = renderer.with_var("--typmark-image-max-width", value),
            _ => {}
        }
    }
    renderer
}

struct PdfSettings {
    page: Option<String>,
    margin: Option<PdfMargin>,
    scale: Option<String>,
    base: Option<String>,
    backend: PdfBackend,
}

fn parse_pdf_settings(settings: Option<&AttrList>) -> Result<PdfSettings, String> {
    let mut pdf = PdfSettings {
        page: None,
        margin: Some(PdfMargin::new("1.5rem", "1.5rem", "1.5rem", "1.5rem")),
        scale: None,
        base: None,
        backend: PdfBackend::Auto,
    };
    let Some(settings) = settings else {
        return Ok(pdf);
    };

    for item in &settings.items {
        let value = item.value.raw.trim();
        if value.is_empty() {
            continue;
        }
        match item.key.as_str() {
            "pdf-page" => pdf.page = Some(value.to_string()),
            "pdf-margin" => {
                pdf.margin = Some(parse_pdf_margin(value)?);
            }
            "pdf-scale" => {
                parse_pdf_scale(value)?;
                pdf.scale = Some(value.to_string());
            }
            "pdf-base" => pdf.base = Some(value.to_string()),
            "pdf-backend" => {
                pdf.backend = parse_pdf_backend(value)?;
            }
            _ => {}
        }
    }

    Ok(pdf)
}

fn parse_pdf_backend(value: &str) -> Result<PdfBackend, String> {
    match value {
        "auto" => Ok(PdfBackend::Auto),
        "chromium" | "chrome" => Ok(PdfBackend::Chromium),
        "wkhtmltopdf" | "wkhtml" => Ok(PdfBackend::Wkhtmltopdf),
        _ => Err(format!(
            "unsupported pdf-backend: {} (expected auto|chromium|wkhtmltopdf)",
            value
        )),
    }
}

fn parse_pdf_scale(value: &str) -> Result<f32, String> {
    let scale = value
        .parse::<f32>()
        .map_err(|_| format!("pdf-scale must be a positive number, got {}", value))?;
    if scale <= 0.0 {
        return Err(format!(
            "pdf-scale must be a positive number, got {}",
            value
        ));
    }
    Ok(scale)
}

fn parse_pdf_margin(value: &str) -> Result<PdfMargin, String> {
    let normalized = value.replace(',', " ");
    let parts: Vec<&str> = normalized.split_whitespace().collect();
    let values = match parts.len() {
        1 => (parts[0], parts[0], parts[0], parts[0]),
        2 => (parts[0], parts[1], parts[0], parts[1]),
        3 => (parts[0], parts[1], parts[2], parts[1]),
        4 => (parts[0], parts[1], parts[2], parts[3]),
        _ => {
            return Err(format!(
                "pdf-margin expects 1 to 4 values, got {}",
                parts.len()
            ));
        }
    };
    Ok(PdfMargin::new(values.0, values.1, values.2, values.3))
}

fn resolve_pdf_base_url(
    settings: &PdfSettings,
    input_path: Option<&Path>,
) -> Result<Option<String>, String> {
    if let Some(base) = settings.base.as_deref() {
        let trimmed = base.trim();
        if trimmed.is_empty() {
            return Ok(None);
        }
        if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
            return Err("pdf-base does not allow remote URLs".to_string());
        }
        if trimmed.starts_with("file://") {
            let mut url = trimmed.to_string();
            if !url.ends_with('/') {
                url.push('/');
            }
            return Ok(Some(url));
        }
        let base_dir = resolve_pdf_base_dir(Path::new(trimmed), input_path)?;
        return Ok(Some(path_to_file_url_dir(&base_dir)?));
    }

    let Some(default_dir) = default_pdf_base_dir(input_path)? else {
        return Ok(None);
    };
    Ok(Some(path_to_file_url_dir(&default_dir)?))
}

fn resolve_pdf_base_dir(path: &Path, input_path: Option<&Path>) -> Result<PathBuf, String> {
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let base_dir = match input_path.and_then(|input| input.parent()) {
        Some(parent) if !parent.as_os_str().is_empty() => parent.to_path_buf(),
        _ => env::current_dir()
            .map_err(|err| format!("failed to resolve current directory: {}", err))?,
    };
    Ok(base_dir.join(path))
}

fn default_pdf_base_dir(input_path: Option<&Path>) -> Result<Option<PathBuf>, String> {
    let from_input = input_path.and_then(|input| input.parent()).map(|dir| {
        if dir.as_os_str().is_empty() {
            env::current_dir().ok()
        } else {
            Some(dir.to_path_buf())
        }
    });
    if let Some(Some(dir)) = from_input {
        return Ok(Some(dir));
    }
    env::current_dir()
        .map(Some)
        .map_err(|err| format!("failed to resolve current directory: {}", err))
}

fn path_to_file_url(path: &Path) -> Result<String, String> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        env::current_dir()
            .map_err(|err| format!("failed to resolve current directory: {}", err))?
            .join(path)
    };
    let mut value = absolute.to_string_lossy().replace('\\', "/");
    if !value.starts_with('/') {
        value = format!("/{}", value);
    }

    let mut out = String::from("file://");
    for byte in value.as_bytes() {
        let ch = *byte as char;
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '~' | '/') {
            out.push(ch);
        } else {
            out.push_str(&format!("%{:02X}", byte));
        }
    }
    Ok(out)
}

fn path_to_file_url_dir(path: &Path) -> Result<String, String> {
    let mut url = path_to_file_url(path)?;
    if !url.ends_with('/') {
        url.push('/');
    }
    Ok(url)
}

fn diagnostics_to_json(diagnostics: &[Diagnostic]) -> String {
    if diagnostics.is_empty() {
        return "[]".to_string();
    }

    let mut out = String::new();
    out.push_str("[\n");
    for (idx, diag) in diagnostics.iter().enumerate() {
        out.push_str("  {\n");
        out.push_str(&format!("    \"code\": \"{}\",\n", diag.code));
        out.push_str(&format!(
            "    \"severity\": \"{}\",\n",
            severity_label(diag.severity)
        ));
        out.push_str(&format!(
            "    \"message\": \"{}\",\n",
            escape_json(&diag.message)
        ));
        out.push_str("    \"range\": {\n");
        out.push_str(&format!(
            "      \"start\": {{ \"line\": {}, \"character\": {} }},\n",
            diag.range.start.line, diag.range.start.character
        ));
        out.push_str(&format!(
            "      \"end\": {{ \"line\": {}, \"character\": {} }}\n",
            diag.range.end.line, diag.range.end.character
        ));
        out.push_str("    }");

        if diag.related.is_empty() {
            out.push_str("\n  }");
        } else {
            out.push_str(",\n    \"related\": [\n");
            for (rel_idx, related) in diag.related.iter().enumerate() {
                out.push_str("      {\n");
                out.push_str("        \"range\": {\n");
                out.push_str(&format!(
                    "          \"start\": {{ \"line\": {}, \"character\": {} }},\n",
                    related.range.start.line, related.range.start.character
                ));
                out.push_str(&format!(
                    "          \"end\": {{ \"line\": {}, \"character\": {} }}\n",
                    related.range.end.line, related.range.end.character
                ));
                out.push_str("        }");
                if let Some(message) = &related.message {
                    out.push_str(&format!(
                        ",\n        \"message\": \"{}\"\n",
                        escape_json(message)
                    ));
                    out.push_str("      }");
                } else {
                    out.push_str("\n      }");
                }
                if rel_idx + 1 < diag.related.len() {
                    out.push_str(",\n");
                } else {
                    out.push('\n');
                }
            }
            out.push_str("    ]\n  }");
        }

        if idx + 1 < diagnostics.len() {
            out.push_str(",\n");
        } else {
            out.push('\n');
        }
    }
    out.push(']');
    out
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
    }
}

fn escape_json(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}
