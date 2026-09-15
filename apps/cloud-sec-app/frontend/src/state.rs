//! Client-side application state (signals shared by the views).

use cloud_sec_bridge::{AskResult, SourceView, StatusResult};
use leptos::prelude::*;

use crate::api::ApiError;

/// Shared state for the single-screen flow. `Copy` because all fields are
/// copyable signals.
#[derive(Debug, Clone, Copy)]
pub struct AppState {
    /// Current question text.
    pub question: RwSignal<String>,
    /// Streamed answer text (authoritative copy arrives with the result).
    pub answer: RwSignal<String>,
    /// Whether a question is in flight.
    pub busy: RwSignal<bool>,
    /// Resolved result of the last question: sources and timings.
    pub result: RwSignal<Option<AskResult>>,
    /// Last failure, if any.
    pub error: RwSignal<Option<ApiError>>,
    /// Backend status, fetched once on mount.
    pub status: RwSignal<Option<StatusResult>>,
}

impl AppState {
    /// Fresh state for a new app session.
    pub fn new() -> Self {
        Self {
            question: RwSignal::new(String::new()),
            answer: RwSignal::new(String::new()),
            busy: RwSignal::new(false),
            result: RwSignal::new(None),
            error: RwSignal::new(None),
            status: RwSignal::new(None),
        }
    }

    /// Sources from the last resolved result, if any.
    pub fn sources(&self) -> Vec<SourceView> {
        self.result
            .get()
            .map(|result| result.sources.clone())
            .unwrap_or_default()
    }
}

impl Default for AppState {
    fn default() -> Self {
        Self::new()
    }
}

/// Thai label for a party key.
pub fn party_label(key: &str) -> &'static str {
    match key {
        "csc" => "ผู้ใช้บริการคลาวด์ (CSC)",
        "csp" => "ผู้ให้บริการคลาวด์ (CSP)",
        _ => "ทั้งสองฝ่าย",
    }
}

/// Milliseconds rendered as seconds with one decimal.
pub fn secs(ms: u64) -> String {
    format!("{:.1}", ms as f64 / 1000.0)
}
