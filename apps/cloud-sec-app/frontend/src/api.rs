//! Frontend API layer - the only place the webview talks to the Tauri
//! backend.
//!
//! Every call is a thin `invoke` to a Rust command, which owns retrieval
//! and model access. Errors cross the IPC as a typed payload
//! (kind + Thai message): components switch on `kind` and display
//! `message` verbatim.

use cloud_sec_bridge::events;
use cloud_sec_bridge::{AskResult, BridgeError, CommandErrorPayload, StatusResult, TokenEvent};

/// A backend command failure, decoded from the bridge payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiError {
    /// Failure class: `offline`, `model`, `empty`, `badRequest`, `internal`.
    pub kind: String,
    /// User-facing message (Thai).
    pub message: String,
}

impl ApiError {
    fn from_bridge(err: BridgeError) -> Self {
        if let BridgeError::Command(text) = &err
            && let Ok(payload) = serde_json::from_str::<CommandErrorPayload>(text)
        {
            return Self {
                kind: payload.kind,
                message: payload.message,
            };
        }
        Self {
            kind: "internal".to_string(),
            message: err.to_string(),
        }
    }
}

/// Ask one question; the answer streams on the `answer-token` event and the
/// resolved result carries sources and timings.
pub async fn ask(question: String) -> Result<AskResult, ApiError> {
    cloud_sec_bridge::invoke("ask", serde_json::json!({ "question": question }))
        .await
        .map_err(ApiError::from_bridge)
}

/// Backend readiness and configuration summary.
pub async fn status() -> Result<StatusResult, ApiError> {
    cloud_sec_bridge::invoke("status", serde_json::json!({}))
        .await
        .map_err(ApiError::from_bridge)
}

/// Subscribe to streamed answer fragments.
pub fn on_answer_token(callback: impl FnMut(TokenEvent) + 'static) {
    events::listen("answer-token", callback);
}
