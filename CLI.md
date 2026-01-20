# TypMark CLI Reference

## Overview
TypMark CLI reads TypMark text and writes HTML to stdout.
Diagnostics are written to stderr.
Exit code is 1 when any error diagnostic exists.

## Usage
```
typmark-cli [options] [input]
```

If input is omitted, TypMark CLI reads from stdin.

## Options
--diagnostics json
Write diagnostics in JSON format to stderr.

--diagnostics pretty
Write diagnostics in readable text to stderr.

--sanitized
Sanitize the HTML output.

--simple-code
Use simple code block output.

--source-map
Add data-tm-range attributes for source mapping. Ranges use zero-based line numbers and UTF-8 byte columns in the form startLine:startCol-endLine:endCol.

--no-section-wrap
Do not wrap sections with section tags.

--render
Wrap output in a full HTML document with inline CSS. This is the default output.

--render-js
Wrap output in a full HTML document with inline CSS and JS.

--raw
Output raw HTML without renderer wrapping.

--pdf output.pdf
Export a PDF to the given path. PDF options are read from the document settings line.

--theme auto|light|dark
Select the theme for rendered output. Default is dark.

--version
Print the CLI version.

--help
Print usage help.

## PDF settings
PDF export reads these keys from the document settings line.

- pdf-page
- pdf-margin default 1.5rem
- pdf-scale
- pdf-base
- pdf-backend

PDF export uses the renderer stylesheet and syntax highlighting.
JavaScript is not embedded in PDF output.
PDF output always uses a dedicated light theme with a white background.
Install wkhtmltopdf or a chromium-based browser before using PDF export.
For PDF output, pdf-margin is applied as page margin.

## Output
HTML is written to stdout.
Diagnostics are written to stderr.
When diagnostics are enabled, HTML is still written to stdout. Ignore stdout if you only need diagnostics.
When --pdf is used, PDF is written to the output path and no HTML is printed.

## Exit codes
0 when there are no error diagnostics.
1 when there is at least one error diagnostic.
2 when command line arguments are invalid.

## Examples
```
typmark-cli input.tmd > output.html
```

```
cat input.tmd | typmark-cli --diagnostics pretty
```

```
typmark-cli --render input.tmd > output.html
```

```
typmark-cli --render --theme dark input.tmd > output.html
```

```
typmark-cli --raw input.tmd > output.html
```

```
typmark-cli --pdf output.pdf input.tmd
```
