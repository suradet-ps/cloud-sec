//! Cloud Sec domain core: the clause model, chunking, hybrid retrieval, and
//! prompt assembly.
//!
//! This crate is pure: no IO, no HTTP, no async runtime. Everything here is
//! testable without a database, a model, or a document on disk.

pub mod chunk;
pub mod error;
pub mod intent;
pub mod model;
pub mod prompt;
pub mod search;
pub mod text;

pub use chunk::{Page, chunk_pages, curated_level_chunks, parse_pages};
pub use error::CoreError;
pub use intent::{GREETING_ANSWER, HELP_ANSWER, Intent, canned_answer, classify};
pub use model::{Chunk, ImpactLevel, Party};
pub use search::{SearchOutcome, search};
pub use text::{chunk_level, query_clause, query_level, query_party};
