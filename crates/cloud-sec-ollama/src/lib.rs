//! Minimal Ollama HTTP client: embeddings and streaming chat against the
//! loopback endpoint. The only crate in the workspace allowed to touch the
//! network.

use std::io::{BufRead, BufReader};
use std::time::Duration;

use serde_json::{Value, json};
use thiserror::Error;

/// Failures raised by the Ollama client.
#[derive(Debug, Error)]
pub enum OllamaError {
    /// The server could not be reached.
    #[error("cannot reach Ollama at {base}: {message}")]
    Connection {
        /// Endpoint that was contacted.
        base: String,
        /// Transport failure detail.
        message: String,
    },
    /// The server answered with a non-success status.
    #[error("Ollama returned HTTP {status}: {body}")]
    Status {
        /// HTTP status code.
        status: u16,
        /// Response body (truncated).
        body: String,
    },
    /// The response could not be decoded.
    #[error("decoding Ollama response failed: {0}")]
    Decode(String),
    /// Reading the streaming response failed.
    #[error("reading Ollama stream failed: {0}")]
    Stream(String),
}

/// Timing counters for one chat completion.
#[derive(Debug, Default, Clone)]
pub struct ChatStats {
    /// Full answer text.
    pub text: String,
    /// Prompt (prefill) token count.
    pub prompt_tokens: u64,
    /// Prompt evaluation time in nanoseconds.
    pub prompt_ns: u64,
    /// Generated token count.
    pub eval_tokens: u64,
    /// Generation time in nanoseconds.
    pub eval_ns: u64,
}

/// Blocking HTTP client for the local Ollama server.
pub struct Client {
    base: String,
    agent: ureq::Agent,
}

impl Client {
    /// Build a client for `base`, for example `http://127.0.0.1:11434`.
    pub fn new(base: impl Into<String>) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(5))
            .timeout(Duration::from_secs(900))
            .build();
        Self {
            base: base.into(),
            agent,
        }
    }

    /// The endpoint this client talks to.
    pub fn base(&self) -> &str {
        &self.base
    }

    fn map_error(&self, err: ureq::Error) -> OllamaError {
        match err {
            ureq::Error::Status(status, response) => {
                let body: String = response
                    .into_string()
                    .unwrap_or_default()
                    .chars()
                    .take(300)
                    .collect();
                OllamaError::Status { status, body }
            }
            ureq::Error::Transport(transport) => OllamaError::Connection {
                base: self.base.clone(),
                message: transport.to_string(),
            },
        }
    }

    /// Server version string, used as a lightweight liveness ping.
    ///
    /// # Errors
    ///
    /// Returns an error when the server does not answer within the short
    /// ping timeout.
    pub fn version(&self) -> Result<String, OllamaError> {
        let response = self
            .agent
            .get(&format!("{}/api/version", self.base))
            .timeout(Duration::from_secs(3))
            .call()
            .map_err(|e| self.map_error(e))?;
        let value: Value = response
            .into_json()
            .map_err(|e| OllamaError::Decode(e.to_string()))?;
        Ok(value["version"].as_str().unwrap_or_default().to_string())
    }

    /// Embed a batch of texts.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails, the response has no
    /// `embeddings` array, or the vector count does not match `texts`.
    pub fn embed(&self, model: &str, texts: &[String]) -> Result<Vec<Vec<f32>>, OllamaError> {
        let body = json!({ "model": model, "input": texts });
        let response = self
            .agent
            .post(&format!("{}/api/embed", self.base))
            .send_json(body)
            .map_err(|e| self.map_error(e))?;
        let value: Value = response
            .into_json()
            .map_err(|e| OllamaError::Decode(e.to_string()))?;
        let arr = value["embeddings"]
            .as_array()
            .ok_or_else(|| OllamaError::Decode("missing 'embeddings' array".to_string()))?;
        if arr.len() != texts.len() {
            return Err(OllamaError::Decode(format!(
                "expected {} embeddings, got {}",
                texts.len(),
                arr.len()
            )));
        }
        arr.iter()
            .map(|item| {
                item.as_array()
                    .map(|vec| {
                        vec.iter()
                            .map(|x| x.as_f64().unwrap_or(0.0) as f32)
                            .collect()
                    })
                    .ok_or_else(|| OllamaError::Decode("embedding is not an array".to_string()))
            })
            .collect()
    }

    /// Stream one chat completion, calling `on_token` for every content
    /// fragment as it arrives.
    ///
    /// # Errors
    ///
    /// Returns an error when the request fails or the stream cannot be read.
    pub fn chat_stream(
        &self,
        model: &str,
        system: &str,
        user: &str,
        num_ctx: u32,
        mut on_token: impl FnMut(&str),
    ) -> Result<ChatStats, OllamaError> {
        let body = json!({
            "model": model,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ],
            "stream": true,
            "options": { "num_ctx": num_ctx, "temperature": 0.2 }
        });
        let response = self
            .agent
            .post(&format!("{}/api/chat", self.base))
            .send_json(body)
            .map_err(|e| self.map_error(e))?;
        let reader = response.into_reader();
        let mut stats = ChatStats::default();
        for line in BufReader::new(reader).lines() {
            let line = line.map_err(|e| OllamaError::Stream(e.to_string()))?;
            if line.trim().is_empty() {
                continue;
            }
            let value: Value =
                serde_json::from_str(&line).map_err(|e| OllamaError::Decode(e.to_string()))?;
            if let Some(text) = value["message"]["content"].as_str()
                && !text.is_empty()
            {
                on_token(text);
                stats.text.push_str(text);
            }
            if value["done"].as_bool().unwrap_or(false) {
                stats.prompt_tokens = value["prompt_eval_count"].as_u64().unwrap_or(0);
                stats.prompt_ns = value["prompt_eval_duration"].as_u64().unwrap_or(0);
                stats.eval_tokens = value["eval_count"].as_u64().unwrap_or(0);
                stats.eval_ns = value["eval_duration"].as_u64().unwrap_or(0);
            }
        }
        Ok(stats)
    }
}
