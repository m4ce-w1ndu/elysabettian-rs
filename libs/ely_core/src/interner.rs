use std::collections::HashMap;

/// Global string pool providing deduplicated string storage and O(1) comparison via 32-bit IDs.
#[derive(Debug, Default, Clone)]
pub struct StringInterner {
    strings: Vec<String>,
    lookup: HashMap<String, u32>,
}

impl StringInterner {
    /// Creates a new, empty string interner.
    pub fn new() -> Self {
        Self::default()
    }

    /// Interns a string slice, returning its unique 32-bit identifier.
    pub fn intern(&mut self, s: &str) -> u32 {
        if self.lookup.contains_key(s) {
            return self.lookup[s];
        }

        let id = self.strings.len() as u32;
        let string = s.to_string();
        self.lookup.insert(string.clone(), id);
        self.strings.push(string);

        id
    }

    /// Resolves an interned string slice by its 32-bit identifier.
    pub fn get(&self, id: u32) -> Option<&str> {
        self.strings.get(id as usize).map(|s| s.as_str())
    }
}
