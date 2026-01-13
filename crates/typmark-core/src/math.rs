use std::path::{Path, PathBuf};
use std::sync::Mutex;

use lru::LruCache;
use once_cell::sync::Lazy;
use typst::diag::{FileError, FileResult};
use typst::foundations::{Bytes, Datetime};
use typst::layout::PagedDocument;
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::utils::LazyHash;
use typst::{Library, LibraryExt, World};

/// The state for a single Typst compilation.
struct MathWorld<'a> {
    library: &'a LazyHash<Library>,
    book: LazyHash<FontBook>,
    fonts: &'a [Font],
    source: Source,
    main_id: FileId,
}

impl World for MathWorld<'_> {
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
        } else {
            Err(FileError::NotFound(id.vpath().as_rooted_path().into()))
        }
    }

    fn file(&self, id: FileId) -> FileResult<Bytes> {
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

    let mut paths = Vec::new();
    if let Ok(value) = std::env::var("TYPMARK_FONT_PATHS") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        paths.extend(
            value
                .split(separator)
                .filter(|entry| !entry.is_empty())
                .map(PathBuf::from),
        );
    }
    if paths.is_empty() {
        let default_path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/NotoSans-Regular.ttf");
        paths.push(default_path);
    }

    for path in expand_font_paths(&paths) {
        if let Ok(font_bytes) = std::fs::read(&path) {
            push_font_bytes(&mut book, &mut fonts, font_bytes);
        }
    }

    FontSlot { book, fonts }
}

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

fn is_font_file(path: &Path) -> bool {
    let Some(ext) = path.extension().and_then(|ext| ext.to_str()) else {
        return false;
    };
    matches!(ext, "ttf" | "otf" | "ttc" | "otc")
}

type CacheKey = (String, bool); // (source, is_display_mode)
type Cache = Mutex<LruCache<CacheKey, String>>;

static FONT_SLOT: Lazy<FontSlot> = Lazy::new(load_fonts);
static TYPST_LIBRARY: Lazy<LazyHash<Library>> = Lazy::new(|| LazyHash::new(Library::default()));
static RENDER_CACHE: Lazy<Cache> = Lazy::new(|| Mutex::new(LruCache::new(100.try_into().unwrap())));

/// Renders a Typst math snippet to an SVG string.
/// Returns Ok(svg_string) on success, or Err(raw_source) on failure.
pub fn render_math(source: &str, display: bool) -> Result<String, String> {
    let cache_key = (source.to_string(), display);

    // Check cache first

    if let Some(cached) = RENDER_CACHE.lock().unwrap().get(&cache_key) {
        return Ok(cached.clone());
    }

    // Create a Typst world for this compilation

    let mut preamble = if display {
        String::from("#set page(width: auto, height: auto, margin: 0.5em)\n")
    } else {
        String::from("#set page(width: auto, height: auto, margin: 0.2em)\n")
    };
    if display {
        preamble.push_str("#set block(spacing: 0.5em)\n");
    }

    let wrapped_source = format!(
        "{}#math.equation(block: {}, $ {} $)",
        preamble, display, source
    );

    let main_file_id = FileId::new(None, VirtualPath::new("main.typ"));

    let world = MathWorld {
        library: &TYPST_LIBRARY,
        book: LazyHash::new(FONT_SLOT.book.clone()),
        fonts: &FONT_SLOT.fonts,
        source: Source::new(main_file_id, wrapped_source),
        main_id: main_file_id,
    };

    // Compile and render

    let result = {
        let warned = typst::compile::<PagedDocument>(&world);
        if std::env::var("TYPMARK_DEBUG_MATH").is_ok() {
            for warning in &warned.warnings {
                eprintln!(
                    "typst math warning: {:?}: {}",
                    warning.severity, warning.message
                );
            }
        }
        warned.output.ok().and_then(|doc| {
            if doc.pages.is_empty() {
                None
            } else {
                Some(typst_svg::svg(&doc.pages[0]))
            }
        })
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
