use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn bin_path() -> PathBuf {
    if let Some(path) = env::var_os("CARGO_BIN_EXE_typmark-cli") {
        return PathBuf::from(path);
    }
    if let Some(path) = env::var_os("CARGO_BIN_EXE_typmark_cli") {
        return PathBuf::from(path);
    }
    let exe = env::current_exe().expect("current exe");
    let mut debug_dir = exe.as_path();
    while let Some(parent) = debug_dir.parent() {
        if parent.file_name().and_then(|name| name.to_str()) == Some("debug") {
            let candidate = parent.join("typmark-cli");
            if candidate.exists() {
                return candidate;
            }
        }
        debug_dir = parent;
    }
    panic!("binary path missing");
}

fn temp_file(name: &str, contents: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let now = SystemTime::now().duration_since(UNIX_EPOCH).expect("time");
    let file_name = format!(
        "typmark_cli_{}_{}_{}.tmd",
        name,
        now.as_secs(),
        now.subsec_nanos()
    );
    path.push(file_name);
    fs::write(&path, contents).expect("write temp file");
    path
}

#[test]
fn diagnostics_pretty_reports_error_and_exit_code() {
    let input = temp_file("ref_omit", "{#p}\nParagraph.\n\n@p\n");
    let output = Command::new(bin_path())
        .args(["--diagnostics", "pretty", input.to_str().expect("path")])
        .output()
        .expect("run");

    assert!(!output.status.success(), "expected error exit code");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("E_REF_OMIT"),
        "expected E_REF_OMIT in stderr"
    );
}

#[test]
fn diagnostics_json_reports_warning_and_exit_code() {
    let input = temp_file("ref_missing", "@missing[link]\n");
    let output = Command::new(bin_path())
        .args(["--diagnostics", "json", input.to_str().expect("path")])
        .output()
        .expect("run");

    assert!(output.status.success(), "expected success exit code");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("\"code\": \"W_REF_MISSING\""),
        "expected W_REF_MISSING in stderr"
    );
}

#[test]
fn render_wraps_html_with_assets() {
    let input = temp_file("render", "Paragraph.\n");
    let output = Command::new(bin_path())
        .args([input.to_str().expect("path")])
        .output()
        .expect("run");

    assert!(output.status.success(), "expected success exit code");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<!DOCTYPE html>"), "expected HTML wrapper");
    assert!(stdout.contains("<style>"), "expected inline CSS");
    assert!(
        !stdout.contains("<script>"),
        "expected no inline JS by default"
    );
}

#[test]
fn render_allows_theme_selection() {
    let input = temp_file("render_theme", "Paragraph.\n");
    let output = Command::new(bin_path())
        .args(["--theme", "dark", input.to_str().expect("path")])
        .output()
        .expect("run");

    assert!(output.status.success(), "expected success exit code");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<!DOCTYPE html>"), "expected HTML wrapper");
}

#[test]
fn raw_outputs_fragment_html() {
    let input = temp_file("raw", "Paragraph.\n");
    let output = Command::new(bin_path())
        .args(["--raw", input.to_str().expect("path")])
        .output()
        .expect("run");

    assert!(output.status.success(), "expected success exit code");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(!stdout.contains("<!DOCTYPE html>"), "expected raw HTML");
    assert!(stdout.contains("<p>Paragraph.</p>"));
}
