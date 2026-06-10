use std::collections::HashMap;
use std::sync::LazyLock;

static SYNONYMS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("tell", "explain");
    m.insert("describe", "explain");
    m.insert("talk", "explain");
    m.insert("walk", "explain");
    m.insert("ownerships", "ownership");
    m.insert("borrowing", "borrow");
    m.insert("references", "reference");
    m.insert("differences", "difference");
    m
});

pub fn apply_synonym(token: &str) -> &str {
    SYNONYMS.get(token).copied().unwrap_or(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_synonym_tell() {
        assert_eq!(apply_synonym("tell"), "explain");
    }

    #[test]
    fn test_synonym_ownerships() {
        assert_eq!(apply_synonym("ownerships"), "ownership");
    }

    #[test]
    fn test_no_synonym() {
        assert_eq!(apply_synonym("rust"), "rust");
    }
}
