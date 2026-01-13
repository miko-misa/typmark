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

--no-section-wrap
Do not wrap sections with section tags.

--render
Wrap output in a full HTML document with inline CSS.

--render-js
Wrap output in a full HTML document with inline CSS and JS.

--raw
Output raw HTML without renderer wrapping.

--theme auto|light|dark
Select the theme for rendered output. Default is dark.

--help
Print usage help.

## Output
HTML is written to stdout.
Diagnostics are written to stderr.

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
