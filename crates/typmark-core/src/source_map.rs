use crate::span::Span;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Position {
    pub line: usize,
    pub character: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Clone, Debug)]
pub struct SourceMap {
    source_len: usize,
    line_starts: Vec<usize>,
}

impl SourceMap {
    pub fn new(source: &str) -> Self {
        let mut line_starts = Vec::new();
        line_starts.push(0);
        for (idx, byte) in source.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push(idx + 1);
            }
        }
        Self {
            source_len: source.len(),
            line_starts,
        }
    }

    pub fn line_count(&self) -> usize {
        self.line_starts.len()
    }

    pub fn position(&self, offset: usize) -> Position {
        let offset = offset.min(self.source_len);
        let line = match self.line_starts.binary_search(&offset) {
            Ok(index) => index,
            Err(index) => index.saturating_sub(1),
        };
        let line_start = self.line_starts[line];
        Position {
            line,
            // Byte offset from line start (ASCII-safe for now).
            character: offset.saturating_sub(line_start),
        }
    }

    pub fn range(&self, span: Span) -> Range {
        Range {
            start: self.position(span.start),
            end: self.position(span.end),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Position, SourceMap};
    use crate::span::Span;

    #[test]
    fn positions_are_line_based() {
        let source = "a\nb\n";
        let map = SourceMap::new(source);

        assert_eq!(map.line_count(), 3);
        assert_eq!(
            map.position(0),
            Position {
                line: 0,
                character: 0
            }
        );
        assert_eq!(
            map.position(2),
            Position {
                line: 1,
                character: 0
            }
        );
        assert_eq!(
            map.position(4),
            Position {
                line: 2,
                character: 0
            }
        );

        let span = Span { start: 0, end: 3 };
        let range = map.range(span);
        assert_eq!(range.start.line, 0);
        assert_eq!(range.end.line, 1);
    }
}
