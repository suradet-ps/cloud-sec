//! Tauri commands: thin wrappers over the domain crates.
//!
//! Commands validate input, call a crate, and map errors to a typed
//! [`CommandError`] (kind + Thai message) so the frontend can switch on the
//! failure class instead of matching message text.

use std::time::Instant;

use cloud_sec_bridge::{AskResult, SourceView, StatusResult, TokenEvent};
use cloud_sec_config as config;
use cloud_sec_core::{Chunk, intent, prompt, search};
use cloud_sec_ollama::{Client, OllamaError};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;

/// Failure payload shape the frontend switches on.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    /// Failure class: `offline`, `model`, `empty`, `badRequest`, `internal`.
    pub kind: &'static str,
    /// User-facing message (Thai).
    pub message: String,
}

impl CommandError {
    fn new(kind: &'static str, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn bad_request(message: impl Into<String>) -> Self {
        Self::new("badRequest", message)
    }

    fn empty(message: impl Into<String>) -> Self {
        Self::new("empty", message)
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::new("internal", message)
    }
}

impl From<OllamaError> for CommandError {
    fn from(err: OllamaError) -> Self {
        match &err {
            OllamaError::Connection { .. } => Self::new(
                "offline",
                "เชื่อมต่อ Ollama ไม่ได้ ตรวจสอบว่าเปิดโปรแกรม Ollama อยู่ (127.0.0.1:11434)",
            ),
            OllamaError::Status { status, .. } => {
                Self::new("model", format!("โมเดลตอบกลับผิดพลาด (HTTP {status})"))
            }
            OllamaError::Decode(_) | OllamaError::Stream(_) => {
                Self::new("model", format!("อ่านคำตอบจากโมเดลไม่ได้: {err}"))
            }
        }
    }
}

/// Backend readiness and configuration summary.
///
/// The Ollama liveness ping uses a short timeout so a hung server cannot
/// stall the UI status refresh.
#[tauri::command]
pub fn status(app: AppHandle) -> StatusResult {
    let chunks = app.state::<AppState>().chunks.len();
    let client = Client::new(config::OLLAMA_BASE);
    let ollama_online = client.version().is_ok();
    StatusResult {
        ready: chunks > 0,
        ollama_online,
        chunks,
        chat_model: config::CHAT_MODEL.to_string(),
        embed_model: config::EMBED_MODEL.to_string(),
        ollama_base: config::OLLAMA_BASE.to_string(),
    }
}

/// Answer one question from the indexed document, streaming tokens as they
/// arrive on the `answer-token` event.
///
/// # Errors
///
/// Returns [`CommandError`] when the question is empty, Ollama is offline,
/// or retrieval finds no context.
#[tauri::command]
pub async fn ask(app: AppHandle, question: String) -> Result<AskResult, CommandError> {
    let question = question.trim().to_string();
    if question.is_empty() {
        return Err(CommandError::bad_request("กรุณาพิมพ์คำถาม"));
    }
    if app.state::<AppState>().chunks.is_empty() {
        return Err(CommandError::internal("ยังไม่ได้โหลดดัชนีเอกสาร"));
    }
    let chunks = app.state::<AppState>().chunks.clone();
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || run_ask(&handle, &chunks, &question))
        .await
        .map_err(|e| CommandError::internal(format!("งานเบื้องหลังล้มเหลว: {e}")))?
}

/// A canned, model-free answer (help and greeting intents).
fn canned(answer: &str, started: Instant) -> AskResult {
    AskResult {
        answer: answer.to_string(),
        sources: Vec::new(),
        prefill_tokens: 0,
        prefill_ms: 0,
        eval_tokens: 0,
        eval_ms: 0,
        wall_ms: started.elapsed().as_millis() as u64,
    }
}

fn run_ask(app: &AppHandle, chunks: &[Chunk], question: &str) -> Result<AskResult, CommandError> {
    let started = Instant::now();

    // Meta and greeting questions never enter retrieval: forcing them
    // through search pulls a random clause and the model then refuses.
    if let Some(answer) = intent::canned_answer(question) {
        tracing::info!("canned intent answer for: {question}");
        return Ok(canned(answer, started));
    }

    let client = Client::new(config::OLLAMA_BASE);

    let mut vectors = client.embed(config::EMBED_MODEL, &[question.to_string()])?;
    let query = vectors
        .pop()
        .ok_or_else(|| CommandError::internal("ไม่ได้รับ embedding ของคำถาม"))?;

    let outcome = search(question, &query, chunks, config::TOP_K);
    if outcome.hits.is_empty() {
        return Err(CommandError::empty("ไม่พบข้อมูลในเอกสารสำหรับคำถามนี้"));
    }
    let (user_prompt, used) =
        prompt::build_user_prompt(question, &outcome.hits, chunks, config::MAX_CONTEXT_CHARS);
    if used == 0 {
        return Err(CommandError::empty("ไม่พบข้อมูลในเอกสารสำหรับคำถามนี้"));
    }

    let sources: Vec<SourceView> = outcome
        .hits
        .iter()
        .map(|(index, score)| {
            let chunk = &chunks[*index];
            SourceView {
                clause: chunk.clause.clone(),
                heading: chunk.heading.clone(),
                party: chunk.party.as_str().to_string(),
                page: chunk.page,
                score: *score,
                text: chunk.text.clone(),
            }
        })
        .collect();

    let stats = client.chat_stream(
        config::CHAT_MODEL,
        prompt::SYSTEM_PROMPT,
        &user_prompt,
        config::NUM_CTX,
        |token| {
            if let Err(e) = app.emit(
                "answer-token",
                TokenEvent {
                    text: token.to_string(),
                },
            ) {
                tracing::warn!("emit answer-token failed: {e}");
            }
        },
    )?;

    Ok(AskResult {
        answer: stats.text,
        sources,
        prefill_tokens: stats.prompt_tokens,
        prefill_ms: stats.prompt_ns / 1_000_000,
        eval_tokens: stats.eval_tokens,
        eval_ms: stats.eval_ns / 1_000_000,
        wall_ms: started.elapsed().as_millis() as u64,
    })
}
