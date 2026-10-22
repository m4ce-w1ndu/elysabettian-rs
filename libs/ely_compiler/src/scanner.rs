/// Zero-allocation lexical scanner that tokenizes Elysabettian source code.
pub struct Scanner<'a> {
    source: &'a str,
    start: usize,
    current: usize,
    line: u32,
}

impl<'a> Scanner<'a> {
    /// Creates a new scanner initialized at the start of the source string.
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            start: 0,
            current: 0,
            line: 1,
        }
    }
}
