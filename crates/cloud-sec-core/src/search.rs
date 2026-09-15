//! Hybrid retrieval: deterministic filters first, vector score as the tie
//! breaker.
//!
//! Pure vector similarity clusters tightly on this document because most
//! clauses open with the same boilerplate. Three deterministic signals are
//! therefore applied: an explicit clause number filters candidates to that
//! clause, a named party re-ranks towards its column, and a named impact
//! level re-ranks towards the matching rows.

use crate::model::{Chunk, ImpactLevel, Party};
use crate::text::{chunk_level, query_clause, query_level, query_party};

/// Party preference bonus, small enough never to beat a clause filter.
const PARTY_BONUS: f32 = 0.05;
/// Bonus for chunks that describe the impact level the question asks about.
const LEVEL_BONUS: f32 = 0.05;

/// Search result with the filters that were applied, for transparency.
#[derive(Debug, Clone)]
pub struct SearchOutcome {
    /// Winning chunks as (index into the chunk slice, score).
    pub hits: Vec<(usize, f32)>,
    /// Clause number detected in the question, if any.
    pub clause_filter: Option<String>,
    /// Party detected in the question, if any.
    pub party_filter: Option<Party>,
    /// Impact level detected in the question, if any.
    pub level_filter: Option<ImpactLevel>,
}

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
            if party_filter == Some(c.party) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Party;

    fn chunk(id: &str, clause: &str, party: Party, vector: Vec<f32>) -> Chunk {
        Chunk {
            id: id.to_string(),
            clause: clause.to_string(),
            heading: String::new(),
            party,
            page: 1,
            text: String::new(),
            embed_text: String::new(),
            vector,
        }
    }

    #[test]
    fn clause_number_filters_candidates() {
        let chunks = vec![
            chunk("a", "5.1.1", Party::Both, vec![1.0, 0.0]),
            chunk("b", "5.2.3.4", Party::Csc, vec![0.9, 0.1]),
            chunk("c", "5.2.3.4", Party::Csp, vec![0.8, 0.2]),
        ];
        let outcome = search("ข้อ 5.2.3.4 ทำอะไร", &[1.0, 0.0], &chunks, 3);
        assert_eq!(outcome.clause_filter.as_deref(), Some("5.2.3.4"));
        assert_eq!(outcome.hits.len(), 2);
        assert!(outcome.hits.iter().all(|(i, _)| *i != 0));
    }

    #[test]
    fn party_bonus_prefers_the_named_column() {
        let chunks = vec![
            chunk("csc", "5.2.5.1", Party::Csc, vec![1.0, 0.0]),
            chunk("csp", "5.2.5.1", Party::Csp, vec![0.999, 0.0]),
        ];
        let outcome = search("ผู้ให้บริการคลาวด์ต้องทำอะไร", &[1.0, 0.0], &chunks, 2);
        assert_eq!(outcome.party_filter, Some(Party::Csp));
        assert_eq!(outcome.hits[0].0, 1);
    }
}
