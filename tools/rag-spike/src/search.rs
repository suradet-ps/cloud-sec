//! Cosine similarity search over the in-memory index, with hybrid filters.
//!
//! Pure vector search clusters tightly on this document because most clauses
//! open with the same boilerplate. Two deterministic signals break the ties:
//! an explicit clause number in the question filters candidates to that
//! clause, and a named party re-ranks towards the matching column.

use crate::chunk::{Chunk, chunk_level, query_clause, query_level, query_party};

fn cosine(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0_f32;
    let mut na = 0.0_f32;
    let mut nb = 0.0_f32;
    for (x, y) in a.iter().zip(b.iter()) {
        dot += x * y;
        na += x * x;
        nb += y * y;
    }
    if na == 0.0 || nb == 0.0 {
        return 0.0;
    }
    dot / (na.sqrt() * nb.sqrt())
}

/// Party preference bonus, small enough never to beat a clause filter.
const PARTY_BONUS: f32 = 0.05;
/// Bonus for chunks that describe the impact level the question asks about.
const LEVEL_BONUS: f32 = 0.05;

/// Search result with the filters that were applied, for transparency.
pub struct SearchOutcome {
    pub hits: Vec<(usize, f32)>,
    pub clause_filter: Option<String>,
    pub party_filter: Option<&'static str>,
    pub level_filter: Option<&'static str>,
}

/// Return the `k` best chunks for the question.
pub fn search(question: &str, query: &[f32], chunks: &[Chunk], k: usize) -> SearchOutcome {
    let clause_filter = query_clause(question);
    let party_filter = query_party(question);
    let level_filter = query_level(question);

    let mut scored: Vec<(usize, f32)> = chunks
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let mut score = cosine(query, &c.vector);
            if party_filter == Some(c.party.as_str()) {
                score += PARTY_BONUS;
            }
            if level_filter.is_some() && chunk_level(&c.text) == level_filter {
                score += LEVEL_BONUS;
            }
            (i, score)
        })
        .collect();

    if let Some(clause) = clause_filter.as_ref() {
        let exact: Vec<usize> = chunks
            .iter()
            .enumerate()
            .filter(|(_, c)| c.clause == *clause)
            .map(|(i, _)| i)
            .collect();
        let allowed = if !exact.is_empty() {
            Some(exact)
        } else {
            let prefix = format!("{clause}.");
            let nested: Vec<usize> = chunks
                .iter()
                .enumerate()
                .filter(|(_, c)| c.clause.starts_with(&prefix))
                .map(|(i, _)| i)
                .collect();
            (!nested.is_empty()).then_some(nested)
        };
        if let Some(allowed) = allowed {
            scored.retain(|(i, _)| allowed.contains(i));
        }
    }

    scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    scored.truncate(k);
    SearchOutcome {
        hits: scored,
        clause_filter,
        party_filter,
        level_filter,
    }
}
