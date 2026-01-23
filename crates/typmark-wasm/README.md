# TypMark WASM

This crate exposes TypMark rendering for browser use via `wasm-bindgen`.

## Build

```sh
wasm-pack build --target web --out-dir pkg
```

## JS usage (draft)

```js
import init, { render_html_with_options, add_font } from "./pkg/typmark_wasm.js";

await init();
add_font(fontBytes);

const result = render_html_with_options(source, {
  wrapSections: true,
  simpleCodeBlocks: false,
});
```

## Notes
- PDF export is disabled in wasm builds.
- Fonts must be provided via `add_font` when you need custom fonts.
