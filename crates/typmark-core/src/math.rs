use std::sync::{Arc, Mutex};

use crate::ast::{Block, BlockKind, BoxBlock, List, TypstPreamble};
use lru::LruCache;
use once_cell::sync::Lazy;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

/// The state for a single Typst compilation.
struct MathWorld {
    library: &'static LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: Vec<Font>,
    source: Source,
    main_id: FileId,
    external_asset_requested: Mutex<bool>,
}

impl World for MathWorld {
    fn library(&self) -> &LazyHash<Library> {
        self.library
    }

    fn book(&self) -> &LazyHash<FontBook> {
        &self.book
    }

    fn main(&self) -> FileId {
        self.main_id
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main_id {
            Ok(self.source.clone())
        } else if id.package().is_some() {
            #[cfg(not(target_arch = "wasm32"))]
            {
                let bytes = read_package_file(id)?;
                let text = std::str::from_utf8(&bytes).map_err(FileError::from)?;
                Ok(Source::new(id, text.to_string()))
            }
            #[cfg(target_arch = "wasm32")]
            {
                Err(FileError::NotFound(id.vpath().as_rooted_path().into()))
            }
        } else {
            *self.external_asset_requested.lock().unwrap() = true;
            Err(FileError::NotFound(id.vpath().as_rooted_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
        #[cfg(not(target_arch = "wasm32"))]
        if id.package().is_some() {
            return read_package_file(id).map(Bytes::new);
        }
        if id.package().is_none() {
            *self.external_asset_requested.lock().unwrap() = true;
        }
        Err(FileError::NotFound(id.vpath().as_rooted_path().into()))
    }

    fn font(&self, index: usize) -> Option<Font> {
        self.fonts.get(index).cloned()
    }

    fn today(&self, _offset: Option<i64>) -> Option<Datetime> {
        None
    }
}

struct FontSlot {
    book: FontBook,
    fonts: Vec<Font>,
}

fn push_font_bytes<T>(book: &mut FontBook, fonts: &mut Vec<Font>, bytes: T)
where
    T: AsRef<[u8]> + Send + Sync + 'static,
{
    let buffer = Bytes::new(bytes);
    for font in Font::iter(buffer) {
        book.push(font.info().clone());
        fonts.push(font);
    }
}

fn load_fonts() -> FontSlot {
    let mut book = FontBook::new();
    let mut fonts = Vec::new();

    for font_bytes in typst_assets::fonts() {
        push_font_bytes(&mut book, &mut fonts, font_bytes);
    }

    #[cfg(not(target_arch = "wasm32"))]
    let mut paths = Vec::new();
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(value) = std::env::var("TYPMARK_FONT_PATHS") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        paths.extend(
            value
                .split(separator)
                .filter(|entry| !entry.is_empty())
                .map(PathBuf::from),
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    if paths.is_empty() {
        let default_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/NotoSans-Regular.ttf");
        paths.push(default_path);
    }

    #[cfg(not(target_arch = "wasm32"))]
    for path in expand_font_paths(&paths) {
        if let Ok(font_bytes) = std::fs::read(&path) {
            push_font_bytes(&mut book, &mut fonts, font_bytes);
        }
    }

    FontSlot { book, fonts }
}

#[cfg(not(target_arch = "wasm32"))]
fn expand_font_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for path in paths {
        if path.is_dir() {
            if let Ok(entries) = std::fs::read_dir(path) {
                let mut files = Vec::new();
                for entry in entries.flatten() {
                    let entry_path = entry.path();
                    if is_font_file(&entry_path) {
                        files.push(entry_path);
                    }
                }
                files.sort();
                out.extend(files);
            }
        } else if is_font_file(path) {
            out.push(path.clone());
        }
    }
    out
}

#[cfg(not(target_arch = "wasm32"))]
fn is_font_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    matches!(ext, "ttf" | "otf" | "ttc" | "otc")
}

#[cfg(not(target_arch = "wasm32"))]
fn read_package_file(id: FileId) -> FileResult<Vec<u8>> {
    let Some(path) = resolve_package_file(id) else {
        return Err(FileError::NotFound(package_search_path(id)));
    };
    if path.is_dir() {
        return Err(FileError::IsDirectory);
    }
    std::fs::read(&path).map_err(|err| FileError::from_io(err, &path))
}

#[cfg(not(target_arch = "wasm32"))]
fn resolve_package_file(id: FileId) -> Option<PathBuf> {
    for root in package_roots() {
        let path = package_file_path(&root, id)?;
        if path.exists() {
            return Some(path);
        }
    }
    None
}

#[cfg(not(target_arch = "wasm32"))]
fn package_file_path(root: &Path, id: FileId) -> Option<PathBuf> {
    let spec = id.package()?;
    let package_root = root
        .join(spec.namespace.as_str())
        .join(spec.name.as_str())
        .join(spec.version.to_string());
    id.vpath().resolve(&package_root)
}

#[cfg(not(target_arch = "wasm32"))]
fn package_search_path(id: FileId) -> PathBuf {
    package_roots()
        .into_iter()
        .find_map(|root| package_file_path(&root, id))
        .unwrap_or_else(|| id.vpath().as_rooted_path().into())
}

#[cfg(not(target_arch = "wasm32"))]
fn package_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Ok(value) = std::env::var("TYPMARK_TYPST_PACKAGE_PATHS") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        roots.extend(
            value
                .split(separator)
                .filter(|entry| !entry.is_empty())
                .map(PathBuf::from),
        );
    }
    if let Some(path) = typst_data_dir() {
        roots.push(path.join("typst").join("packages"));
    }
    if let Some(path) = typst_cache_dir() {
        roots.push(path.join("typst").join("packages"));
    }
    roots
}

#[cfg(not(target_arch = "wasm32"))]
fn typst_data_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        home_dir().map(|home| home.join("Library").join("Application Support"))
    } else if cfg!(windows) {
        std::env::var_os("APPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".local").join("share")))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn typst_cache_dir() -> Option<PathBuf> {
    if cfg!(target_os = "macos") {
        home_dir().map(|home| home.join("Library").join("Caches"))
    } else if cfg!(windows) {
        std::env::var_os("LOCALAPPDATA").map(PathBuf::from)
    } else {
        std::env::var_os("XDG_CACHE_HOME")
            .map(PathBuf::from)
            .or_else(|| home_dir().map(|home| home.join(".cache")))
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

type CacheKey = (String, bool, Option<String>, Option<String>, Option<String>); // (source, is_display_mode, inline_size, block_size, font)
type Cache = Mutex<LruCache<CacheKey, String>>;
type EmbedCache = Mutex<LruCache<String, Arc<TypstRenderOutcome>>>;

const MAX_TYPST_SOURCE_BYTES: usize = 256 * 1024;
const MAX_TYPST_PAGES: usize = 16;
const MAX_TYPST_SVG_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TypstResourceLimit {
    SourceBytes,
    Pages,
    SvgBytes,
}

#[derive(Clone, Debug)]
pub(crate) struct TypstRenderOutcome {
    pub svg: Option<String>,
    pub page_count: usize,
    pub external_asset_requested: bool,
    pub resource_limit: Option<TypstResourceLimit>,
}

static FONT_SLOT: Lazy<Mutex<FontSlot>> = Lazy::new(|| Mutex::new(load_fonts()));
static TYPST_LIBRARY: Lazy<LazyHash<Library>> = Lazy::new(|| LazyHash::new(Library::default()));
static RENDER_CACHE: Lazy<Cache> = Lazy::new(|| Mutex::new(LruCache::new(100.try_into().unwrap())));
static EMBED_RENDER_CACHE: Lazy<EmbedCache> =
    Lazy::new(|| Mutex::new(LruCache::new(100.try_into().unwrap())));

#[derive(Clone, Debug, Default)]
pub struct MathSettings {
    pub inline_size: Option<String>,
    pub block_size: Option<String>,
    pub font: Option<String>,
}

fn normalize_svg_with_limit(svg: String) -> Result<String, TypstResourceLimit> {
    if svg.len() > MAX_TYPST_SVG_BYTES {
        return Err(TypstResourceLimit::SvgBytes);
    }
    Ok(normalize_svg_ids(&svg))
}

fn render_first_page_with_limits(
    document: &PagedDocument,
) -> Result<Option<String>, TypstResourceLimit> {
    if document.pages.len() > MAX_TYPST_PAGES {
        return Err(TypstResourceLimit::Pages);
    }
    document
        .pages
        .first()
        .map(|page| normalize_svg_with_limit(typst_svg::svg(page)))
        .transpose()
}

/// Renders a Typst math snippet to an SVG string.
/// Returns Ok(svg_string) on success, or Err(raw_source) on failure.
pub fn render_math(source: &str, display: bool, settings: &MathSettings) -> Result<String, String> {
    let size_setting = if display {
        settings.block_size.as_deref()
    } else {
        settings.inline_size.as_deref()
    };
    let input_bytes = source
        .len()
        .saturating_add(settings.font.as_deref().map_or(0, str::len))
        .saturating_add(size_setting.map_or(0, str::len));
    if input_bytes > MAX_TYPST_SOURCE_BYTES {
        return Err(source.to_string());
    }

    let cache_key = (
        source.to_string(),
        display,
        settings.inline_size.clone(),
        settings.block_size.clone(),
        settings.font.clone(),
    );

    // Check cache first

    if let Some(cached) = RENDER_CACHE.lock().unwrap().get(&cache_key) {
        return Ok(cached.clone());
    }

    // Create a Typst world for this compilation

    let mut preamble = String::from(
        "#show math.equation: set text(top-edge: \"bounds\", bottom-edge: \"bounds\")\n",
    );
    if let Some(font) = &settings.font {
        preamble.push_str(&format!("#set text(font: \"{}\")\n", font));
    }
    if display {
        preamble.push_str("#set page(width: auto, height: auto, margin: 0.5em)\n");
        preamble.push_str("#set block(spacing: 0.5em)\n");
        let size = settings.block_size.as_deref().unwrap_or("14.5pt");
        preamble.push_str(&format!("#set text(size: {})\n", size));
    } else {
        preamble.push_str("#set page(width: auto, height: auto, margin: (top: 0.35em, bottom: 0.35em, left: 0.2em, right: 0.2em))\n");
        let size = settings.inline_size.as_deref().unwrap_or("13pt");
        preamble.push_str(&format!("#set text(size: {})\n", size));
    };

    let wrapped_source = format!(
        "{}#math.equation(block: {}, $ {} $)",
        preamble, display, source
    );
    if wrapped_source.len() > MAX_TYPST_SOURCE_BYTES {
        return Err(source.to_string());
    }

    let main_file_id = FileId::new(None, VirtualPath::new("main.typ"));

    let (book, fonts) = {
        let slot = FONT_SLOT.lock().unwrap();
        (slot.book.clone(), slot.fonts.clone())
    };

    let world = MathWorld {
        library: &TYPST_LIBRARY,
        book: LazyHash::new(book),
        fonts,
        source: Source::new(main_file_id, wrapped_source),
        main_id: main_file_id,
        external_asset_requested: Mutex::new(false),
    };

    // Compile and render

    let result = {
        let warned = typst::compile::<PagedDocument>(&world);
        #[cfg(not(target_arch = "wasm32"))]
        if std::env::var("TYPMARK_DEBUG_MATH").is_ok() {
            for warning in &warned.warnings {
                eprintln!(
                    "typst math warning: {:?}: {}",
                    warning.severity, warning.message
                );
            }
        }
        warned
            .output
            .ok()
            .and_then(|doc| render_first_page_with_limits(&doc).ok().flatten())
    };

    match result {
        Some(svg) => {
            RENDER_CACHE.lock().unwrap().put(cache_key, svg.clone());

            Ok(svg)
        }

        None => {
            if std::env::var("TYPMARK_DEBUG_MATH").is_ok() {
                let warned = typst::compile::<PagedDocument>(&world);
                if let Err(errors) = warned.output {
                    for error in errors {
                        eprintln!("typst math error: {:?}: {}", error.severity, error.message);
                    }
                }
            }
            Err(source.to_string())
        }
    }
}

fn cache_typst_outcome(source: &str, outcome: TypstRenderOutcome) -> Arc<TypstRenderOutcome> {
    let outcome = Arc::new(outcome);
    EMBED_RENDER_CACHE
        .lock()
        .unwrap()
        .put(source.to_string(), Arc::clone(&outcome));
    outcome
}

/// Compiles a full Typst snippet for HTML embedding.
///
/// The snippet is compiled as normal Typst content, unlike `render_math`, which
/// wraps its source in a Typst math equation. Successful and failed outcomes are
/// both cached so validation and emission share one compilation.
pub(crate) fn render_typst_svg(source: &str) -> Arc<TypstRenderOutcome> {
    if let Some(cached) = EMBED_RENDER_CACHE.lock().unwrap().get(source) {
        return Arc::clone(cached);
    }
    if source.len() > MAX_TYPST_SOURCE_BYTES {
        return Arc::new(TypstRenderOutcome {
            svg: None,
            page_count: 0,
            external_asset_requested: false,
            resource_limit: Some(TypstResourceLimit::SourceBytes),
        });
    }

    let wrapped_source = format!(
        "#set page(width: auto, height: auto, margin: 0pt, fill: none)\n#set text(size: 11pt)\n{}",
        source
    );
    let main_file_id = FileId::new(None, VirtualPath::new("main.typ"));

    let (book, fonts) = {
        let slot = FONT_SLOT.lock().unwrap();
        (slot.book.clone(), slot.fonts.clone())
    };

    let world = MathWorld {
        library: &TYPST_LIBRARY,
        book: LazyHash::new(book),
        fonts,
        source: Source::new(main_file_id, wrapped_source),
        main_id: main_file_id,
        external_asset_requested: Mutex::new(false),
    };

    let warned = typst::compile::<PagedDocument>(&world);
    #[cfg(not(target_arch = "wasm32"))]
    if std::env::var("TYPMARK_DEBUG_TYPST").is_ok() {
        for warning in &warned.warnings {
            eprintln!(
                "typst embed warning: {:?}: {}",
                warning.severity, warning.message
            );
        }
    }

    let (svg, page_count, resource_limit) = match warned.output {
        Ok(doc) => {
            let page_count = doc.pages.len();
            match render_first_page_with_limits(&doc) {
                Ok(svg) => (svg, page_count, None),
                Err(limit) => (None, page_count, Some(limit)),
            }
        }
        Err(errors) => {
            #[cfg(not(target_arch = "wasm32"))]
            if std::env::var("TYPMARK_DEBUG_TYPST").is_ok() {
                for error in errors {
                    eprintln!("typst embed error: {:?}: {}", error.severity, error.message);
                }
            }
            (None, 0, None)
        }
    };

    cache_typst_outcome(
        source,
        TypstRenderOutcome {
            svg,
            page_count,
            external_asset_requested: *world.external_asset_requested.lock().unwrap(),
            resource_limit,
        },
    )
}

pub fn typst_source_with_preamble(source: &str, preamble: &str) -> String {
    if preamble.trim().is_empty() {
        return source.to_string();
    }

    let preamble = preamble.trim_end();
    let mut out = String::with_capacity(preamble.len() + 1 + source.len());
    out.push_str(preamble);
    out.push('\n');
    out.push_str(source);
    out
}

pub(crate) fn collect_typst_preamble(blocks: &[Block]) -> String {
    let mut parts = Vec::new();
    collect_typst_preamble_parts(blocks, &mut parts);
    parts.join("\n")
}

fn collect_typst_preamble_parts<'a>(blocks: &'a [Block], parts: &mut Vec<&'a str>) {
    for block in blocks {
        match &block.kind {
            BlockKind::TypstPreamble(TypstPreamble { typst_src, .. }) => {
                parts.push(typst_src.as_str());
            }
            BlockKind::List(List { items, .. }) => {
                for item in items {
                    collect_typst_preamble_parts(&item.blocks, parts);
                }
            }
            BlockKind::BlockQuote { blocks } => collect_typst_preamble_parts(blocks, parts),
            BlockKind::Box(BoxBlock { blocks, .. }) => collect_typst_preamble_parts(blocks, parts),
            BlockKind::Section { children, .. } => collect_typst_preamble_parts(children, parts),
            _ => {}
        }
    }
}

/// Adds a font from raw bytes to the Typst font book.
pub fn add_font_bytes(bytes: Vec<u8>) {
    let mut slot = FONT_SLOT.lock().unwrap();
    let FontSlot { book, fonts } = &mut *slot;
    push_font_bytes(book, fonts, bytes);
    RENDER_CACHE.lock().unwrap().clear();
    EMBED_RENDER_CACHE.lock().unwrap().clear();
}

pub fn prefix_svg_ids(svg: &str, prefix: &str) -> String {
    let mut ids = Vec::new();
    let mut search = 0;
    while let Some(symbol_pos) = svg[search..].find("<symbol") {
        let symbol_start = search + symbol_pos;
        let Some(id_pos) = svg[symbol_start..].find(" id=\"") else {
            search = symbol_start + 7;
            continue;
        };
        let id_start = symbol_start + id_pos + 5;
        let Some(relative_end) = svg[id_start..].find('"') else {
            break;
        };
        let id_end = id_start + relative_end;
        ids.push(svg[id_start..id_end].to_string());
        search = id_end + 1;
    }
    rewrite_svg_ids(svg, prefix, ids)
}

pub fn prefix_typst_svg_ids(svg: &str, prefix: &str) -> String {
    let mut ids = Vec::new();
    let mut search = 0;
    while let Some(relative_start) = svg[search..].find(" id=\"") {
        let id_start = search + relative_start + 5;
        let Some(relative_end) = svg[id_start..].find('"') else {
            break;
        };
        let id_end = id_start + relative_end;
        let id = svg[id_start..id_end].to_string();
        if !ids.contains(&id) {
            ids.push(id);
        }
        search = id_end + 1;
    }
    rewrite_svg_ids(svg, prefix, ids)
}

fn rewrite_svg_ids(svg: &str, prefix: &str, ids: Vec<String>) -> String {
    let mut out = svg.to_string();
    for id in ids {
        let new_id = format!("{}-{}", prefix, id);
        out = out.replace(&format!("id=\"{}\"", id), &format!("id=\"{}\"", new_id));
        out = out.replace(
            &format!("xlink:href=\"#{}\"", id),
            &format!("xlink:href=\"#{}\"", new_id),
        );
        out = out.replace(
            &format!("href=\"#{}\"", id),
            &format!("href=\"#{}\"", new_id),
        );
        out = out.replace(&format!("url(#{})", id), &format!("url(#{})", new_id));
    }
    out
}

fn normalize_svg_ids(svg: &str) -> String {
    let mut ids = Vec::new();
    let mut search = 0;
    while let Some(symbol_pos) = svg[search..].find("<symbol") {
        let symbol_start = search + symbol_pos;
        let id_attr_pos = match svg[symbol_start..].find("id=\"") {
            Some(pos) => symbol_start + pos + 4,
            None => {
                search = symbol_start + 7;
                continue;
            }
        };
        let id_end = match svg[id_attr_pos..].find('"') {
            Some(pos) => id_attr_pos + pos,
            None => break,
        };
        ids.push(svg[id_attr_pos..id_end].to_string());
        search = id_end;
    }

    if ids.is_empty() {
        return svg.to_string();
    }

    let mut out = svg.to_string();
    for (index, id) in ids.iter().enumerate() {
        let new_id = format!("g{}", index + 1);
        out = out.replace(&format!("id=\"{}\"", id), &format!("id=\"{}\"", new_id));
        out = out.replace(
            &format!("xlink:href=\"#{}\"", id),
            &format!("xlink:href=\"#{}\"", new_id),
        );
        out = out.replace(
            &format!("href=\"#{}\"", id),
            &format!("href=\"#{}\"", new_id),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::{
        MAX_TYPST_PAGES, MAX_TYPST_SOURCE_BYTES, MAX_TYPST_SVG_BYTES, MathSettings,
        TypstResourceLimit, normalize_svg_with_limit, prefix_typst_svg_ids, render_math,
        render_typst_svg,
    };

    #[test]
    fn prefix_typst_svg_ids_rewrites_definitions_and_references() {
        let svg = r##"<svg><defs id="glyph"><linearGradient id="paint"/></defs><path fill="url(#paint)"/><use href="#glyph" xlink:href="#glyph"/></svg>"##;
        let prefixed = prefix_typst_svg_ids(svg, "tm-t1");

        assert!(prefixed.contains("id=\"tm-t1-glyph\""));
        assert!(prefixed.contains("id=\"tm-t1-paint\""));
        assert!(prefixed.contains("url(#tm-t1-paint)"));
        assert!(prefixed.contains("href=\"#tm-t1-glyph\""));
        assert!(prefixed.contains("xlink:href=\"#tm-t1-glyph\""));
    }

    #[test]
    fn failed_typst_render_outcomes_are_cached() {
        let source = "#let = typmark-cache-test";
        let first = render_typst_svg(source);
        let second = render_typst_svg(source);

        assert!(first.svg.is_none());
        assert!(Arc::ptr_eq(&first, &second));
    }

    #[test]
    fn oversized_typst_sources_are_rejected_without_caching_the_payload() {
        let source = "x".repeat(MAX_TYPST_SOURCE_BYTES + 1);
        let first = render_typst_svg(&source);
        let second = render_typst_svg(&source);

        assert_eq!(first.resource_limit, Some(TypstResourceLimit::SourceBytes));
        assert!(!Arc::ptr_eq(&first, &second));
        assert!(render_math(&source, false, &MathSettings::default()).is_err());
    }

    #[test]
    fn excessive_page_counts_are_rejected_before_svg_rendering() {
        let source = "[Page]\n#pagebreak()\n".repeat(MAX_TYPST_PAGES + 1);
        let outcome = render_typst_svg(&source);

        assert!(outcome.page_count > MAX_TYPST_PAGES);
        assert!(outcome.svg.is_none());
        assert_eq!(outcome.resource_limit, Some(TypstResourceLimit::Pages));
    }

    #[test]
    fn oversized_svg_output_is_rejected() {
        let svg = "x".repeat(MAX_TYPST_SVG_BYTES + 1);
        assert_eq!(
            normalize_svg_with_limit(svg),
            Err(TypstResourceLimit::SvgBytes)
        );
    }
}
