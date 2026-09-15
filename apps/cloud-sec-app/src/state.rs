//! Backend state: the prebuilt retrieval index, loaded once at startup.
//!
//! The index is an asset produced by the spike pipeline
//! (`tools/rag-spike`), not rebuilt at runtime: production ingestion is an
//! open item (see AGENTS.md) because the PDF extraction still needs the PUA
//! combining-mark fix.

use std::sync::Arc;

use cloud_sec_core::Chunk;

/// Shared backend state managed by Tauri.
pub struct AppState {
    /// Indexed chunks; empty when the asset failed to load.
    pub chunks: Arc<Vec<Chunk>>,
}

impl AppState {
    /// Load the bundled index.
    pub fn new() -> Self {
        let json = include_str!("../assets/index.json");
        match cloud_sec_docs::load_index(json) {
            Ok(chunks) => {
                tracing::info!("index loaded: {} chunks", chunks.len());
                Self {
                    chunks: Arc::new(chunks),
                }
            }
            Err(e) => {
                tracing::error!("index asset failed to decode: {e}");
                Self {
                    chunks: Arc::new(Vec::new()),
                }
            }
        }
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}
