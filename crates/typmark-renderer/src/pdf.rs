use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::Renderer;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfBackend {
    Auto,
    Chromium,
    Wkhtmltopdf,
}

#[derive(Debug, Clone)]
pub struct PdfMargin {
    pub top: String,
    pub right: String,
    pub bottom: String,
    pub left: String,
}

impl PdfMargin {
    pub fn new(
        top: impl Into<String>,
        right: impl Into<String>,
        bottom: impl Into<String>,
        left: impl Into<String>,
    ) -> Self {
        Self {
            top: top.into(),
            right: right.into(),
            bottom: bottom.into(),
            left: left.into(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PdfOptions {
    pub backend: PdfBackend,
    pub page: Option<String>,
    pub margin: Option<PdfMargin>,
    pub scale: Option<String>,
    pub base_url: Option<String>,
}

impl PdfOptions {
    pub fn new(backend: PdfBackend) -> Self {
        Self {
            backend,
            page: None,
            margin: None,
            scale: None,
            base_url: None,
        }
    }

    pub fn with_page(mut self, page: impl Into<String>) -> Self {
        self.page = Some(page.into());
        self
    }

    pub fn with_margin(mut self, margin: PdfMargin) -> Self {
        self.margin = Some(margin);
        self
    }

    pub fn with_scale(mut self, scale: impl Into<String>) -> Self {
        self.scale = Some(scale.into());
        self
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }
}

#[derive(Debug, Clone)]
enum ResolvedBackend {
    Chromium(PathBuf),
    Wkhtmltopdf(PathBuf),
}

pub fn export_pdf(
    renderer: &Renderer,
    html: &str,
    options: &PdfOptions,
    output_path: &Path,
) -> Result<(), String> {
    let highlighted = renderer.highlight_html(html);
    let extra_css = pdf_extra_css(options.margin.as_ref());
    let wrapped = renderer.embed_html_with_base_and_css(
        &highlighted,
        true,
        false,
        options.base_url.as_deref(),
        Some(&extra_css),
    );
    let temp = TempFile::new("typmark_pdf", "html")
        .map_err(|err| format!("failed to create temp file: {}", err))?;
    fs::write(&temp.path, wrapped).map_err(|err| format!("failed to write temp html: {}", err))?;

    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create output directory: {}", err))?;
        }
    }

    let backend = resolve_backend(options.backend)?;
    match backend {
        ResolvedBackend::Chromium(path) => {
            export_with_chromium(&path, &temp.path, output_path, options)?
        }
        ResolvedBackend::Wkhtmltopdf(path) => {
            export_with_wkhtmltopdf(&path, &temp.path, output_path, options)?
        }
    }
    Ok(())
}

fn pdf_extra_css(margin: Option<&PdfMargin>) -> String {
    let page_margin = margin
        .map(|value| {
            format!(
                "{} {} {} {}",
                value.top, value.right, value.bottom, value.left
            )
        })
        .unwrap_or_else(|| "0".to_string());
    format!(
        ":root {{\n\
  --typmark-bg: #ffffff;\n\
  --typmark-fg: #111111;\n\
  --typmark-muted: #5f5f5f;\n\
  --typmark-border: #d6d6d6;\n\
  --typmark-accent: #1f5da8;\n\
  --typmark-code-bg: #f5f5f5;\n\
  --typmark-code-fg: #111111;\n\
  --typmark-box-bg: #f7f7f7;\n\
  --typmark-box-border: #d0d0d0;\n\
}}\n\
@page {{ margin: {page_margin}; }}\n\
@media print {{\n\
  html,\n\
  body {{\n\
    margin: 0;\n\
    width: 100%;\n\
    height: 100%;\n\
  }}\n\
  body {{\n\
    box-sizing: border-box;\n\
    max-width: none;\n\
    padding: 0;\n\
  }}\n\
  .TypMark-math-block {{\n\
    overflow: visible;\n\
  }}\n\
  .TypMark-math-block .typst-doc {{\n\
    display: block;\n\
    max-width: 100% !important;\n\
    width: auto !important;\n\
    height: auto !important;\n\
    margin: 0 auto;\n\
  }}\n\
  * {{\n\
    box-sizing: border-box;\n\
  }}\n\
}}\n"
    )
}

fn resolve_backend(backend: PdfBackend) -> Result<ResolvedBackend, String> {
    let chromium = resolve_executable(&[
        "chromium",
        "chromium-browser",
        "google-chrome",
        "google-chrome-stable",
        "chrome",
        "msedge",
        "microsoft-edge",
    ]);
    let wkhtml = resolve_executable(&["wkhtmltopdf"]);

    match backend {
        PdfBackend::Chromium => chromium
            .map(ResolvedBackend::Chromium)
            .ok_or_else(|| "chromium backend not found in PATH".to_string()),
        PdfBackend::Wkhtmltopdf => wkhtml
            .map(ResolvedBackend::Wkhtmltopdf)
            .ok_or_else(|| "wkhtmltopdf backend not found in PATH".to_string()),
        PdfBackend::Auto => {
            if let Some(path) = chromium {
                Ok(ResolvedBackend::Chromium(path))
            } else if let Some(path) = wkhtml {
                Ok(ResolvedBackend::Wkhtmltopdf(path))
            } else {
                Err(
                    "no PDF backend found in PATH (chromium or wkhtmltopdf). Install one and retry."
                        .to_string(),
                )
            }
        }
    }
}

fn export_with_chromium(
    chromium: &Path,
    html_path: &Path,
    output_path: &Path,
    options: &PdfOptions,
) -> Result<(), String> {
    if options.page.is_some() || options.margin.is_some() || options.scale.is_some() {
        eprintln!("note: chromium backend ignores pdf-page, pdf-margin, and pdf-scale");
    }

    let html_url = path_to_file_url(html_path)?;
    let mut cmd = Command::new(chromium);
    cmd.arg("--headless");
    cmd.arg("--disable-gpu");
    cmd.arg("--allow-file-access-from-files");
    cmd.arg("--print-to-pdf-no-header");
    cmd.arg("--no-pdf-header-footer");
    cmd.arg(format!("--print-to-pdf={}", output_path.display()));
    cmd.arg(html_url);
    run_command(cmd, "chromium")
}

fn export_with_wkhtmltopdf(
    wkhtmltopdf: &Path,
    html_path: &Path,
    output_path: &Path,
    options: &PdfOptions,
) -> Result<(), String> {
    let mut cmd = Command::new(wkhtmltopdf);
    cmd.arg("--quiet");
    cmd.arg("--enable-local-file-access");

    if let Some(page) = &options.page {
        cmd.arg("--page-size").arg(page);
    }
    cmd.arg("--margin-top").arg("0");
    cmd.arg("--margin-right").arg("0");
    cmd.arg("--margin-bottom").arg("0");
    cmd.arg("--margin-left").arg("0");
    if let Some(scale) = &options.scale {
        cmd.arg("--zoom").arg(scale);
    }

    cmd.arg(html_path);
    cmd.arg(output_path);
    run_command(cmd, "wkhtmltopdf")
}

fn run_command(mut cmd: Command, label: &str) -> Result<(), String> {
    let output = cmd
        .output()
        .map_err(|err| format!("failed to run {}: {}", label, err))?;
    if output.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut message = format!("{} failed", label);
    let stderr = stderr.trim();
    let stdout = stdout.trim();
    if !stderr.is_empty() {
        message.push_str(&format!(": {}", stderr));
    } else if !stdout.is_empty() {
        message.push_str(&format!(": {}", stdout));
    }
    Err(message)
}

fn resolve_executable(candidates: &[&str]) -> Option<PathBuf> {
    let path_var = env::var_os("PATH")?;
    for dir in env::split_paths(&path_var) {
        for candidate in candidates {
            let full = dir.join(candidate);
            if is_executable(&full) {
                return Some(full);
            }
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    let metadata = match fs::metadata(path) {
        Ok(value) => value,
        Err(_) => return false,
    };
    metadata.is_file() && metadata.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
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

struct TempFile {
    path: PathBuf,
}

impl TempFile {
    fn new(prefix: &str, extension: &str) -> std::io::Result<Self> {
        let mut attempts = 0;
        let pid = std::process::id();
        loop {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default();
            let filename = format!("{}_{}_{}.{}", prefix, pid, now.as_nanos(), extension);
            let path = env::temp_dir().join(filename);
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(_) => return Ok(Self { path }),
                Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => {
                    attempts += 1;
                    if attempts > 10 {
                        return Err(err);
                    }
                }
                Err(err) => return Err(err),
            }
        }
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}
