use std::fs;
use std::path::{Path, PathBuf};
use typmark_core::{
    Diagnostic, DiagnosticSeverity, RelatedDiagnostic, emit_html, emit_html_sanitized, parse,
    resolve,
};

#[test]
fn golden_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures_dir = root.join("tests/fixtures");
    let expect_dir = root.join("tests/expect");

    let mut fixtures = collect_fixtures(&fixtures_dir, false)?;
    fixtures.sort_by(|a, b| file_name(a).cmp(&file_name(b)));

    for fixture in fixtures {
        let name = file_stem(&fixture)?;
        let source = fs::read_to_string(&fixture)?;
        let parsed = parse(&source);
        let resolved = resolve(
            parsed.document,
            &source,
            &parsed.source_map,
            parsed.diagnostics,
            &parsed.link_defs,
        );
        let html = emit_html(&resolved.document.blocks);

        let html_path = expect_dir.join(format!("{}.html", name));
        if html_path.exists() {
            let expected = fs::read_to_string(&html_path)?;
            assert_eq!(
                html.trim_end(),
                expected.trim_end(),
                "HTML mismatch for fixture {}",
                name
            );
        }

        let diag_path = expect_dir.join(format!("{}.diag.json", name));
        if diag_path.exists() {
            let expected = fs::read_to_string(&diag_path)?;
            let actual = diagnostics_to_json(&resolved.diagnostics);
            assert_eq!(
                actual.trim_end(),
                expected.trim_end(),
                "Diagnostics mismatch for fixture {}",
                name
            );
        } else if !resolved.diagnostics.is_empty() {
            panic!(
                "Unexpected diagnostics for fixture {}: {}",
                name,
                diagnostics_to_json(&resolved.diagnostics)
            );
        }
    }

    Ok(())
}

#[test]
fn golden_sanitized_fixtures() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let fixtures_dir = root.join("tests/fixtures/sani");
    let expect_dir = root.join("tests/expect/sani");

    let mut fixtures = collect_fixtures(&fixtures_dir, true)?;
    fixtures.sort_by(|a, b| file_name(a).cmp(&file_name(b)));

    for fixture in fixtures {
        let name = file_stem(&fixture)?;
        let source = fs::read_to_string(&fixture)?;
        let parsed = parse(&source);
        let resolved = resolve(
            parsed.document,
            &source,
            &parsed.source_map,
            parsed.diagnostics,
            &parsed.link_defs,
        );
        let html = emit_html_sanitized(&resolved.document.blocks);

        let html_path = expect_dir.join(format!("{}.html", name));
        if html_path.exists() {
            let expected = fs::read_to_string(&html_path)?;
            assert_eq!(
                html.trim_end(),
                expected.trim_end(),
                "HTML mismatch for fixture {}",
                name
            );
        }
    }

    Ok(())
}

fn collect_fixtures(
    dir: &Path,
    recursive: bool,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut fixtures = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() && recursive {
            fixtures.extend(collect_fixtures(&path, recursive)?);
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("tmd") {
            fixtures.push(path);
        }
    }
    Ok(fixtures)
}

fn file_name(path: &Path) -> &str {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("")
}

fn file_stem(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .map(|value| value.to_string())
        .ok_or_else(|| "fixture name is not valid UTF-8".into())
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
                out.push_str(&related_to_json(related));
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

fn related_to_json(related: &RelatedDiagnostic) -> String {
    let mut out = String::new();
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
    out
}

fn severity_label(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
    }
}

fn escape_json(text: &str) -> String {
    let mut out = String::new();
    for ch in text.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(ch),
        }
    }
    out
}
