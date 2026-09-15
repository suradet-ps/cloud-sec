//! Default values for Cloud Sec, pinned by measurement on the target
//! machine class (see `tools/rag-spike/README.md` for the run log).
//!
//! Single source of defaults: no other crate hard-codes a model name, an
//! endpoint, or a budget.

/// Loopback endpoint of the local Ollama server.
pub const OLLAMA_BASE: &str = "http://127.0.0.1:11434";

/// Local chat model.
pub const CHAT_MODEL: &str = "scb10x/typhoon2.5-qwen3-4b";

/// Local embedding model.
pub const EMBED_MODEL: &str = "bge-m3";

/// Chunks retrieved per question.
pub const TOP_K: usize = 3;

/// Retrieved context cap in characters (about 1500 tokens of Thai).
pub const MAX_CONTEXT_CHARS: usize = 2800;

/// Generation context window.
pub const NUM_CTX: u32 = 8192;

/// Pages below this Thai-character ratio are never indexed.
pub const MIN_THAI_RATIO: f32 = 0.30;
