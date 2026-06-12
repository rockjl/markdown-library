//! Search index: pre-computed tokens for each note, persisted to JSON.

use crate::note::Note;
use crate::search::normalize;
use serde::{Deserialize, Serialize};

/// A single note with its pre-tokenised fields for fast search.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexedNote {
    /// Unique note identifier.
    pub id: u64,
    /// Display title (not tokenised; kept for search results).
    pub title: String,
    /// Normalised tokens extracted from the title.
    pub title_tokens: Vec<String>,
    /// Normalised tokens extracted from the tag string.
    pub tag_tokens: Vec<String>,
    /// Normalised tokens extracted from the first 200 words of content.
    pub content_tokens: Vec<String>,
}

/// Pre-computed search index persisted to `index.json`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchIndex {
    /// All indexed notes.
    pub notes: Vec<IndexedNote>,
}

impl SearchIndex {
    /// Build a search index from a slice of notes.
    ///
    /// Tokenises the title, tags, and first 200 content words of each note.
    pub fn build(notes: &[Note]) -> Self {
        let indexed: Vec<IndexedNote> = notes
            .iter()
            .map(|n| {
                let content_preview: String = n
                    .content
                    .split_whitespace()
                    .take(200)
                    .collect::<Vec<&str>>()
                    .join(" ");
                IndexedNote {
                    id: n.id,
                    title: n.title.clone(),
                    title_tokens: normalize::normalize(&n.title),
                    tag_tokens: normalize::normalize(&n.tags.join(" ")),
                    content_tokens: normalize::normalize(&content_preview),
                }
            })
            .collect();
        SearchIndex { notes: indexed }
    }

    /// Deserialise a search index from a JSON string.
    pub fn from_json(data: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(data)
    }

    /// Serialise the search index to a pretty-printed JSON string.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
