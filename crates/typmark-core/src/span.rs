#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Result<Self, SpanError> {
        if start <= end {
            Ok(Self { start, end })
        } else {
            Err(SpanError::Inverted { start, end })
        }
    }

    pub fn len(&self) -> usize {
        self.end.saturating_sub(self.start)
    }

    pub fn is_empty(&self) -> bool {
        self.start == self.end
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpanError {
    Inverted { start: usize, end: usize },
}
