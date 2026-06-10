use std::collections::HashMap;

use crate::search::index::SearchIndex;
use crate::search::matcher::{SearchConfig, SearchHit, search};

/// Split a transcript into multiple sub-queries using conjunction markers
pub fn split_queries(text: &str) -> Vec<String> {
    let markers = ["and also", "also", "next question", "what about", "another question", "and", "plus", "vs", "versus"];
    let mut segments = vec![text.to_string()];

    for marker in &markers {
        let mut new_segments = Vec::new();
        for seg in &segments {
            let parts: Vec<&str> = seg.splitn(2, marker).collect();
            new_segments.push(parts[0].trim().to_string());
            if parts.len() > 1 {
                new_segments.push(parts[1].trim().to_string());
            }
        }
        segments = new_segments;
    }

    segments.retain(|s| !s.is_empty());
    segments
}

/// Extract question-like segments from a transcript
pub fn extract_questions(transcript: &str) -> Vec<String> {
    let markers = [
        "what is", "what are", "why", "how", "when", "where",
        "difference between", "compare", "explain", "describe", "tell me about",
        "could you explain", "tell me about", "walk me through", "what makes",
        "why did you choose", "how does", "how would you", "can you describe",
        "what happens when", "what is the difference between",
    ];

    let lower = transcript.to_lowercase();
    let mut questions = Vec::new();

    let sentences: Vec<&str> = lower
        .split(|c: char| c == '.' || c == '?' || c == '!')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    for sentence in &sentences {
        for marker in &markers {
            if let Some(pos) = sentence.find(marker) {
                let after_marker = sentence[pos + marker.len()..].trim();
                if !after_marker.is_empty() {
                    let cleaned = after_marker
                        .trim_start_matches(|c: char| c.is_whitespace() || c == ',')
                        .trim();
                    if !cleaned.is_empty() {
                        questions.push(cleaned.to_string());
                    }
                }
                break;
            }
        }
    }

    questions
}

/// Process ASR transcript: extract questions → split queries → search → merge → dedup → sort
pub fn process_transcript(index: &SearchIndex, transcript: &str) -> Vec<SearchHit> {
    eprintln!("[ASR] {}", transcript);

    let questions = extract_questions(transcript);
    let config = SearchConfig::default();
    let mut merged: HashMap<u64, (String, f32)> = HashMap::new();

    let queries = if questions.is_empty() {
        vec![transcript.to_string()]
    } else {
        questions
    };

    for q in &queries {
        let sub_queries = split_queries(q);
        for sq in &sub_queries {
            eprintln!("[QUERY] {}", sq);
            let hits = search(index, sq, config.threshold);
            for h in hits {
                let entry = merged.entry(h.note_id).or_insert_with(|| (h.title.clone(), 0.0));
                if h.score > entry.1 {
                    *entry = (h.title.clone(), h.score);
                }
            }
        }
    }

    let mut results: Vec<SearchHit> = merged
        .into_iter()
        .map(|(note_id, (title, score))| SearchHit { note_id, title, score })
        .collect();

    results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(std::cmp::Ordering::Equal));
    let results = results.into_iter().take(config.top_n).collect::<Vec<_>>();

    for r in &results {
        eprintln!("[RESULT] {} {:.2}", r.title, r.score);
    }

    results
}
