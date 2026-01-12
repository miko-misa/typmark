use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

use typmark_core::{
    Diagnostic, DiagnosticSeverity, HtmlEmitOptions, emit_html_sanitized_with_options,
    emit_html_with_options, parse, resolve,
};

fn main() {
    let mut input: Option<String> = None;
    let mut sanitized = false;
    let mut simple_code_blocks = false;
    let mut wrap_sections = true;
    let mut diagnostics_mode: Option<DiagnosticsMode> = None;

    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print_usage();
                return;
            }
            "--sanitized" => sanitized = true,
            "--simple-code" => simple_code_blocks = true,
            "--no-section-wrap" => wrap_sections = false,
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

    let source = match input {
        Some(path) => fs::read_to_string(&path).unwrap_or_else(|err| {
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
        emit_html_sanitized_with_options(&resolved.document.blocks, &options)
    } else {
        emit_html_with_options(&resolved.document.blocks, &options)
    };

    print!("{}", html);

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
        "Usage: typmark-cli [--sanitized] [--simple-code] [--no-section-wrap] [--diagnostics json|pretty] [input]"
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
