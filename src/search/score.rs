use crate::search::index::IndexedNote;

pub fn jaccard(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let a_set: std::collections::HashSet<&str> = a.iter().map(|s| s.as_str()).collect();
    let b_set: std::collections::HashSet<&str> = b.iter().map(|s| s.as_str()).collect();

    let intersection = a_set.intersection(&b_set).count() as f32;
    let union = a_set.union(&b_set).count() as f32;
    intersection / union
}

pub fn lcs_ratio(a: &[String], b: &[String]) -> f32 {
    if a.is_empty() && b.is_empty() {
        return 1.0;
    }
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let max_len = a.len().max(b.len());
    if max_len == 0 {
        return 1.0;
    }
    let lcs_len = lcs_length(a, b);
    lcs_len as f32 / max_len as f32
}

fn lcs_length(a: &[String], b: &[String]) -> usize {
    let m = a.len();
    let n = b.len();
    let mut dp = vec![vec![0usize; n + 1]; m + 1];
    for i in 1..=m {
        for j in 1..=n {
            if a[i - 1] == b[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
            } else {
                dp[i][j] = dp[i - 1][j].max(dp[i][j - 1]);
            }
        }
    }
    dp[m][n]
}

pub fn similarity(a: &[String], b: &[String]) -> f32 {
    0.7 * jaccard(a, b) + 0.3 * lcs_ratio(a, b)
}

pub fn note_score(query: &[String], note: &IndexedNote) -> f32 {
    let title_score = similarity(query, &note.title_tokens);
    let tag_score = similarity(query, &note.tag_tokens);
    let content_score = similarity(query, &note.content_tokens);
    0.60 * title_score + 0.35 * tag_score + 0.05 * content_score
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn test_jaccard_identical() {
        let a = s(&["a", "b", "c"]);
        let b = s(&["a", "b", "c"]);
        assert!((jaccard(&a, &b) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_jaccard_empty() {
        assert!((jaccard(&s(&[]), &s(&[])) - 1.0).abs() < 1e-6);
        assert!((jaccard(&s(&["a"]), &s(&[])) - 0.0).abs() < 1e-6);
    }

    #[test]
    fn test_jaccard_partial() {
        let a = s(&["a", "b", "c"]);
        let b = s(&["a", "b", "d"]);
        let j = jaccard(&a, &b);
        assert!((j - 0.5).abs() < 1e-6);
    }

    #[test]
    fn test_lcs_identical() {
        let a = s(&["a", "b", "c"]);
        assert!((lcs_ratio(&a, &a) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_lcs_partial() {
        let a = s(&["a", "b", "c"]);
        let b = s(&["a", "b", "d"]);
        let l = lcs_ratio(&a, &b);
        let expected = 2.0 / 3.0;
        assert!((l - expected).abs() < 1e-6);
    }

    #[test]
    fn test_similarity() {
        let a = s(&["explain", "ownership", "rust"]);
        let b = s(&["explain", "ownership", "rust"]);
        let sim = similarity(&a, &b);
        assert!((sim - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_note_score() {
        let note = IndexedNote {
            id: 1,
            title: "Ownership".to_string(),
            title_tokens: s(&["ownership"]),
            tag_tokens: s(&["rust"]),
            content_tokens: s(&[]),
        };
        let query = s(&["ownership", "rust"]);
        let score = note_score(&query, &note);
        assert!(score > 0.0);
    }
}
