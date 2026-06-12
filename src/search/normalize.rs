//! Text tokenization pipeline: lowercasing, filler/stop-word removal, stemming, synonym application.

const FILLERS: &[&str] = &[
    "um", "uh", "like", "actually", "basically", "you know", "well", "so", "right", "okay",
];

const STOP_WORDS: &[&str] = &[
    "a", "an", "the", "is", "it", "are", "was", "were", "be", "been",
    "have", "has", "had", "do", "does", "did", "can", "could", "will", "would",
    "shall", "should", "may", "might", "must", "need", "dare", "ought", "used",
    "to", "of", "in", "for", "on", "with", "at", "by", "from", "as",
    "into", "through", "during", "before", "after", "above", "below", "between",
    "out", "off", "over", "under", "again", "further", "then", "once",
    "here", "there", "when", "where", "why", "how",
    "all", "each", "every", "both", "few", "more", "most", "other", "some", "such",
    "no", "nor", "not", "only", "own", "same", "so", "than", "too", "very",
    "just", "because", "about", "up", "if", "and", "or", "but",
    "what", "which", "who", "whom", "this", "that", "these", "those",
    "i", "you", "he", "she", "we", "they",
    "me", "him", "her", "us", "them",
    "my", "your", "his", "its", "our", "their",
    "mine", "yours", "hers", "ours", "theirs",
    "myself", "yourself", "himself", "herself", "itself",
    "ourselves", "yourselves", "themselves",
];

/// Normalise a text string into a deduplicated list of stemmed tokens.
///
/// Pipeline: lowercase → remove punctuation → split whitespace → remove filler/stop words
/// → apply synonyms → deduplicate adjacent identical tokens → English stemmer.
pub fn normalize(text: &str) -> Vec<String> {
    let text = text.to_lowercase();
    let text = regex::Regex::new(r"[^\w\s'-]")
        .unwrap()
        .replace_all(&text, " ")
        .to_string();

    let tokens: Vec<&str> = text
        .split_whitespace()
        .filter(|t| !is_filler(t) && !is_stop_word(t))
        .collect();

    let mut deduped: Vec<String> = Vec::new();
    for t in tokens {
        let s = crate::search::synonym::apply_synonym(t);
        if deduped.last().map(|d| d.as_str()) != Some(s) {
            deduped.push(stem(s));
        }
    }
    deduped
}

fn is_filler(word: &str) -> bool {
    FILLERS.contains(&word)
}

fn is_stop_word(word: &str) -> bool {
    STOP_WORDS.contains(&word)
}

fn stem(word: &str) -> String {
    let stemmer = rust_stemmers::Stemmer::create(rust_stemmers::Algorithm::English);
    stemmer.stem(word).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_basic() {
        let result = normalize("um can you explain ownerships in rust");
        assert_eq!(result, vec!["explain", "ownership", "rust"]);
    }

    #[test]
    fn test_normalize_synonym() {
        let result = normalize("tell me about borrowing");
        assert!(result.contains(&"explain".to_string()));
        assert!(result.contains(&"borrow".to_string()));
    }

    #[test]
    fn test_normalize_empty() {
        let result = normalize("um uh like");
        assert!(result.is_empty());
    }

    #[test]
    fn test_normalize_dedup_adjacent() {
        let result = normalize("explain explain ownership");
        assert_eq!(result, vec!["explain", "ownership"]);
    }
}
