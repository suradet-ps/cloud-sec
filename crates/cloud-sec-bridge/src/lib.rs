//! The IPC boundary between the Leptos frontend and the Tauri backend.
//!
//! Carries the wire types for every command and the two webview
//! capabilities the UI needs: invoking a command and listening to backend
//! events. On native targets the webview half compiles to a stub so the
//! whole workspace stays `cargo check`-clean everywhere.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors raised while invoking a backend command.
#[derive(Debug, Error)]
pub enum BridgeError {
    /// Command call failed on the backend (its serialized error payload).
    #[error("backend command error: {0}")]
    Command(String),
    /// Payload serialization failed.
    #[error("payload serialization failed: {0}")]
    Serialize(String),
    /// Not running inside a Tauri webview.
    #[error("cloud-sec-bridge is only available inside the Tauri webview (wasm32)")]
    NotWebView,
}

/// Result alias for bridge calls.
pub type Result<T> = std::result::Result<T, BridgeError>;

/// One retrieved clause segment shown next to the answer.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceView {
    /// Canonical clause number, for example `5.2.3.4`.
    pub clause: String,
    /// Clause heading.
    pub heading: String,
    /// Party key: `csc`, `csp`, or `both`.
    pub party: String,
    /// PDF page number.
    pub page: u32,
    /// Retrieval score.
    pub score: f32,
    /// Full source text.
    pub text: String,
}

/// Result of one `ask` command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AskResult {
    /// Full answer text (also streamed token by token).
    pub answer: String,
    /// Retrieved sources, in rank order.
    pub sources: Vec<SourceView>,
    /// Prefill token count.
    pub prefill_tokens: u64,
    /// Prefill time in milliseconds.
    pub prefill_ms: u64,
    /// Generated token count.
    pub eval_tokens: u64,
    /// Generation time in milliseconds.
    pub eval_ms: u64,
    /// Total wall time in milliseconds.
    pub wall_ms: u64,
}

/// Backend readiness and configuration summary.
///
/// `ready` and `ollama_online` drive the user-facing status pill; the model
/// and endpoint fields are diagnostics, shown only as a hover title.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusResult {
    /// True when the index holds at least one chunk.
    pub ready: bool,
    /// True when the local Ollama server answered a liveness ping.
    pub ollama_online: bool,
    /// Number of indexed chunks.
    pub chunks: usize,
    /// Chat model tag.
    pub chat_model: String,
    /// Embedding model tag.
    pub embed_model: String,
    /// Ollama endpoint.
    pub ollama_base: String,
}

/// One streamed answer fragment, emitted as the `answer-token` event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenEvent {
    /// The fragment.
    pub text: String,
}

/// The serialized shape of a backend command failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandErrorPayload {
    /// Failure class: `offline`, `model`, `empty`, `badRequest`, `internal`.
    pub kind: String,
    /// User-facing message (Thai).
    pub message: String,
}

/// Invoke a Tauri command with a serde-serializable payload and decode the
/// JSON response into `T`.
///
/// # Errors
///
/// Returns [`BridgeError::Serialize`] on payload encoding failures,
/// [`BridgeError::Command`] when the backend rejects, and
/// [`BridgeError::NotWebView`] outside the webview.
pub async fn invoke<T: DeserializeOwned>(cmd: &str, args: impl Serialize) -> Result<T> {
    let args = serde_json::to_value(&args).map_err(|e| BridgeError::Serialize(e.to_string()))?;
    invoke_payload(cmd, &args).await
}

#[cfg(target_arch = "wasm32")]
mod imp {
    use super::*;
    use js_sys::Promise;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["__TAURI__", "core"], js_name = "invoke")]
        fn tauri_invoke(cmd: &str, args: JsValue) -> Promise;
    }

    /// Invoke with an already-serialized payload.
    pub async fn invoke_payload<T: DeserializeOwned>(
        cmd: &str,
        args: &serde_json::Value,
    ) -> Result<T> {
        let args_json =
            serde_json::to_string(args).map_err(|e| BridgeError::Serialize(e.to_string()))?;
        let args_value = js_sys::JSON::parse(&args_json)
            .map_err(|e| BridgeError::Serialize(format!("{e:?}")))?;
        let promise = tauri_invoke(cmd, args_value);
        let value = wasm_bindgen_futures::JsFuture::from(promise)
            .await
            .map_err(|e| {
                // The rejection value is the backend's serialized error payload.
                let json = js_sys::JSON::stringify(&e)
                    .ok()
                    .and_then(|j| j.as_string())
                    .unwrap_or_else(|| format!("{e:?}"));
                BridgeError::Command(json)
            })?;
        let json = js_sys::JSON::stringify(&value)
            .map_err(|e| BridgeError::Serialize(format!("{e:?}")))?;
        let text = json.as_string().unwrap_or_else(|| "null".to_string());
        serde_json::from_str(&text).map_err(|e| BridgeError::Serialize(e.to_string()))
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use super::*;

    /// Stub used when compiling for native targets.
    pub async fn invoke_payload<T: DeserializeOwned>(
        _cmd: &str,
        _args: &serde_json::Value,
    ) -> Result<T> {
        Err(BridgeError::NotWebView)
    }
}

use imp::invoke_payload;

/// Backend event subscription (the `answer-token` stream).
#[cfg(target_arch = "wasm32")]
pub mod events {
    use serde::de::DeserializeOwned;
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::prelude::*;

    #[wasm_bindgen]
    extern "C" {
        #[wasm_bindgen(js_namespace = ["__TAURI__", "event"], js_name = "listen")]
        fn tauri_listen(event: &str, handler: &Closure<dyn FnMut(JsValue)>) -> js_sys::Promise;
    }

    /// Subscribe to a backend event; the payload is decoded into `T`.
    ///
    /// The listener lives for the whole app session (the closure is
    /// intentionally forgotten; the UI registers each event once).
    pub fn listen<T, F>(event: &str, mut on_payload: F)
    where
        T: DeserializeOwned + 'static,
        F: FnMut(T) + 'static,
    {
        let handler = Closure::<dyn FnMut(JsValue)>::new(move |event_value: JsValue| {
            let payload = js_sys::Reflect::get(&event_value, &JsValue::from_str("payload"));
            let Ok(payload) = payload else {
                return;
            };
            let Ok(text) = js_sys::JSON::stringify(&payload) else {
                return;
            };
            let Some(text) = text.as_string() else {
                return;
            };
            if let Ok(parsed) = serde_json::from_str::<T>(&text) {
                on_payload(parsed);
            }
        });
        let _promise = tauri_listen(event, &handler);
        handler.forget();
    }
}

/// Backend event subscription, native stub.
#[cfg(not(target_arch = "wasm32"))]
pub mod events {
    use serde::de::DeserializeOwned;

    /// Native stub: events are a webview concern.
    pub fn listen<T, F>(_event: &str, _on_payload: F)
    where
        T: DeserializeOwned + 'static,
        F: FnMut(T) + 'static,
    {
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Ping {
        ok: bool,
    }

    #[tokio::test]
    async fn native_stub_returns_not_webview() {
        let r = invoke::<Ping>("ping", serde_json::json!({})).await;
        assert!(matches!(r, Err(BridgeError::NotWebView)));
    }
}
