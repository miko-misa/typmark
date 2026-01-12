use std::path::{Path, PathBuf};
use std::sync::Mutex;

use comemo::Prehashed;
use lru::LruCache;
use once_cell::sync::Lazy;
use typst::diag::{FileError, FileResult};
use typst::eval::Tracer;
use typst::foundations::{Bytes, Datetime};
use typst::syntax::{FileId, Source, VirtualPath};
use typst::text::{Font, FontBook};
use typst::{Library, World};

/// The state for a single Typst compilation.
struct MathWorld<'a> {
    library: &'a Prehashed<Library>,
    book: &'a Prehashed<FontBook>,
    fonts: &'a [Font],
    source: Source,
}

impl World for MathWorld<'_> {
    fn library(&self) -> &Prehashed<Library> {
        self.library
    }

    fn book(&self) -> &Prehashed<FontBook> {
        self.book
    }

    fn main(&self) -> Source {
        self.source.clone()
    }

    fn source(&self, id: FileId) -> FileResult<Source> {
        if id == self.main().id() {
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

fn load_fonts() -> FontSlot {
    let mut book = FontBook::new();
    let mut fonts = Vec::new();

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
            let buffer = Bytes::from(font_bytes);
            for font in Font::iter(buffer) {
                book.push(font.info().clone());
                fonts.push(font);
            }
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
static TYPST_LIBRARY: Lazy<Prehashed<Library>> = Lazy::new(|| Prehashed::new(Library::default()));
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

    let preamble = if display {
        "#set page(width: auto, height: auto, margin: 0.5em)\n#set block(spacing: 0.5em)\n"
    } else {
        ""
    };

    let wrapped_source = format!(
        "{}#{{math.equation(block: {}, {repr})}}",
        preamble,
        display,
        repr = source
    );

    let main_file_id = FileId::new(None, VirtualPath::new("main.typ"));

    let world = MathWorld {
        library: &TYPST_LIBRARY,

        book: &Prehashed::new(FONT_SLOT.book.clone()),

        fonts: &FONT_SLOT.fonts,

        source: Source::new(main_file_id, wrapped_source),
    };

    // Compile and render

    let result = {
        let mut tracer = Tracer::new();

        typst::compile(&world, &mut tracer).ok().and_then(|doc| {
            if doc.pages.is_empty() {
                None
            } else {
                Some(typst_svg::svg(&doc.pages[0].frame))
            }
        })
    };

    match result {
        Some(svg) => {
            RENDER_CACHE.lock().unwrap().put(cache_key, svg.clone());

            Ok(svg)
        }

        None => Err(source.to_string()),
    }
}
