//! Text search against the search index.

use crate::search::index::SearchIndex;
use crate::search::normalize;
use crate::search::score;

/// A single search result.
pub struct SearchHit {
    /// Identifier of the matched note.
    pub note_id: u64,
    /// Display title of the matched note.
    pub title: String,
    /// Similarity score between 0.0 and 1.0.
    pub score: f32,
}

/// Configuration tweaks for the search algorithm.
pub struct SearchConfig {
    /// Minimum similarity score (0.0–1.0) to include a result.
    pub threshold: f32,
    /// Maximum number of results to return.
    pub top_n: usize,
}

impl Default for SearchConfig {
    fn default() -> Self {
        SearchConfig {
            threshold: 0.50,
            top_n: 5,
        }
    }
}

/// Search the index for notes matching the query.
///
/// * `index` — the pre-computed search index.
/// * `query` — raw user query; tokenised internally.
/// * `threshold` — minimum similarity score to include a result.
/// Returns results sorted by descending score.
pub fn search(index: &SearchIndex, query: &str, threshold: f32) -> Vec<SearchHit> {
    let query_tokens = normalize::normalize(query);
    if query_tokens.is_empty() {
        return Vec::new();
    }

    let mut hits: Vec<SearchHit> = index
        .notes
        .iter()
        .map(|n| {
            let s = score::note_score(&query_tokens, n);
            SearchHit {
                note_id: n.id,
                title: n.title.clone(),
                score: s,
            }
        })
        .filter(|h| h.score >= threshold)
        .collect();

    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    hits
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_index() -> SearchIndex {
        use crate::note::Note;
        let notes = vec![
            Note::new("Ownership", "Rust ownership system"),
            Note::new("Borrow Checker", "How borrowing works in Rust"),
        ];
        SearchIndex::build(&notes)
    }

    #[test]
    fn test_search_found() {
        let index = make_index();
        let hits = search(&index, "ownership", 0.1);
        assert!(!hits.is_empty());
        assert_eq!(hits[0].title, "Ownership");
    }

    #[test]
    fn test_search_threshold() {
        let index = make_index();
        let hits = search(&index, "zzzznotexist", 0.1);
        assert!(hits.is_empty());
    }
}
