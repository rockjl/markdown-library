use crate::note::Note;
use crate::search::normalize;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexedNote {
    pub id: u64,
    pub title: String,
    pub title_tokens: Vec<String>,
    pub tag_tokens: Vec<String>,
    pub content_tokens: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchIndex {
    pub notes: Vec<IndexedNote>,
}

impl SearchIndex {
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

    pub fn from_json(data: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(data)
    }

    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}
