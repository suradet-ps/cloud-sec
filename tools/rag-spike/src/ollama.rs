//! Minimal Ollama HTTP client: embeddings and chat, loopback only.

use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::time::Duration;

pub const BASE: &str = "http://127.0.0.1:11434";

/// Build the shared HTTP agent with generous timeouts for CPU inference.
pub fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout(Duration::from_secs(900))
        .build()
}

/// Embed a batch of texts with the given model.
///
/// # Errors
///
/// Returns an error when the request fails, when the response has no
/// `embeddings` array, or when the vector count does not match `texts`.
pub fn embed(agent: &ureq::Agent, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>> {
    let body = json!({ "model": model, "input": texts });
    let resp = agent
        .post(&format!("{BASE}/api/embed"))
        .send_json(body)
        .context("POST /api/embed failed")?;
    let value: Value = resp.into_json().context("decoding /api/embed response")?;
    let arr = value["embeddings"]
        .as_array()
        .context("missing 'embeddings' in /api/embed response")?;
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        let vec = item.as_array().context("embedding is not an array")?;
        out.push(
            vec.iter()
                .map(|x| x.as_f64().unwrap_or(0.0) as f32)
                .collect(),
        );
    }
    if out.len() != texts.len() {
        bail!("expected {} embeddings, got {}", texts.len(), out.len());
    }
    Ok(out)
}

/// One non-streaming chat completion plus its timing counters.
pub struct ChatStats {
    pub text: String,
    pub prompt_tokens: u64,
    pub prompt_ns: u64,
    pub eval_tokens: u64,
    pub eval_ns: u64,
}

/// Send a system and user message to the chat model.
///
/// # Errors
///
/// Returns an error when the request fails or the response is malformed.
pub fn chat(
    agent: &ureq::Agent,
    model: &str,
    system: &str,
    user: &str,
    num_ctx: u32,
) -> Result<ChatStats> {
    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ],
        "stream": false,
        "options": { "num_ctx": num_ctx, "temperature": 0.2 }
    });
    let resp = agent
        .post(&format!("{BASE}/api/chat"))
        .send_json(body)
        .context("POST /api/chat failed")?;
    let value: Value = resp.into_json().context("decoding /api/chat response")?;
    Ok(ChatStats {
        text: value["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string(),
        prompt_tokens: value["prompt_eval_count"].as_u64().unwrap_or(0),
        prompt_ns: value["prompt_eval_duration"].as_u64().unwrap_or(0),
        eval_tokens: value["eval_count"].as_u64().unwrap_or(0),
        eval_ns: value["eval_duration"].as_u64().unwrap_or(0),
    })
}
