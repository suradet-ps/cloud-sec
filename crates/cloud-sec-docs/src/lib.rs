//! Document index IO for Cloud Sec.
//!
//! Owns the serialized index file and the build path from page-tagged text
//! to chunks. The pure logic (parsing, chunking, curated rows) lives in
//! `cloud-sec-core`; this crate adds the JSON shape and the quality gate.
//!
//! The index contains no vectors until an embedder fills them; the app
//! ships a prebuilt index produced by the spike pipeline.

use cloud_sec_core::{Chunk, CoreError, chunk_pages, curated_level_chunks, parse_pages};
use thiserror::Error;

/// Failures raised by the document layer.
#[derive(Debug, Error)]
pub enum DocsError {
    /// The pure domain layer rejected the input.
    #[error(transparent)]
    Core(#[from] CoreError),
    /// The index JSON could not be decoded.
    #[error("index JSON invalid: {0}")]
    Index(String),
    /// No chunk survived the quality gate and chunking.
    #[error("no usable chunks produced from the page text")]
    Empty,
}

/// Decode an index file into chunks (vectors included when present).
///
/// # Errors
///
/// Returns [`DocsError::Index`] when the JSON does not decode.
pub fn load_index(json: &str) -> Result<Vec<Chunk>, DocsError> {
    serde_json::from_str(json).map_err(|e| DocsError::Index(e.to_string()))
}

/// Encode chunks as the index JSON.
///
/// # Errors
///
/// Returns [`DocsError::Index`] when serialization fails.
pub fn save_index(chunks: &[Chunk]) -> Result<String, DocsError> {
    serde_json::to_string(chunks).map_err(|e| DocsError::Index(e.to_string()))
}

/// Build document chunks from page-tagged text.
///
/// Pages below `min_thai_ratio` are skipped (the extraction quality gate),
/// then the clause-aware chunker runs and the curated section 4 rows are
/// appended. Vectors stay empty; an embedder fills them.
///
/// # Errors
///
/// Returns [`DocsError`] when the page text cannot be parsed or no chunk
/// survives.
pub fn build_document_chunks(
    pages_text: &str,
    min_thai_ratio: f32,
) -> Result<Vec<Chunk>, DocsError> {
    let pages = parse_pages(pages_text)?;
    let usable: Vec<_> = pages
        .iter()
        .filter(|p| p.thai_ratio >= min_thai_ratio)
        .cloned()
        .collect();
    let mut chunks = chunk_pages(&usable);
    chunks.extend(curated_level_chunks());
    if chunks.is_empty() {
        return Err(DocsError::Empty);
    }
    Ok(chunks)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn low_quality_pages_are_excluded() {
        let pages_text = "\
===== PAGE 1 | thai=0.10 =====
garbled legacy text that must never be indexed and is long enough to chunk
===== PAGE 4 | thai=0.85 =====
๕.๑.๑ นโยบายด้านความมั่นคงปลอดภัยสารสนเทศ (Information Security Policies)
ก) ผู้ใช้บริการคลาวด์ต้องกำหนดนโยบายความมั่นคงปลอดภัยสารสนเทศสำหรับการประมวลผลบนคลาวด์
";
        let chunks = build_document_chunks(pages_text, 0.30).expect("chunks");
        assert!(chunks.iter().all(|c| c.page != 1), "no page 1 chunks");
        assert_eq!(chunks.iter().filter(|c| c.clause == "4").count(), 3);
    }

    #[test]
    fn index_round_trips() {
        let chunks = curated_level_chunks();
        let json = save_index(&chunks).expect("save");
        let back = load_index(&json).expect("load");
        assert_eq!(back.len(), chunks.len());
        assert_eq!(back[0].clause, chunks[0].clause);
    }
}
