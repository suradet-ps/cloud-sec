//! The single-screen UI: ask a question, watch the streamed answer, and
//! inspect the retrieved clauses next to it.

use leptos::prelude::*;
use leptos::task::spawn_local;

use cloud_sec_bridge::TokenEvent;

use crate::api;
use crate::state::{AppState, party_label, secs};

/// Mounts the app into the document body.
pub fn run() {
    leptos::mount::mount_to_body(|| view! { <App /> });
}

#[component]
fn App() -> impl IntoView {
    let state = AppState::new();

    // Streamed fragments append to the answer signal.
    api::on_answer_token(move |event: TokenEvent| {
        state.answer.update(|answer| answer.push_str(&event.text));
    });

    // Backend status once on mount.
    spawn_local(async move {
        if let Ok(status) = api::status().await {
            state.status.set(Some(status));
        }
    });

    let on_input = move |ev: web_sys::Event| {
        state.question.set(event_target_value(&ev));
    };
    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        submit(state);
    };

    let status_text = move || match state.status.get() {
        None => "กำลังเชื่อมต่อ...".to_string(),
        Some(status) if !status.ollama_online => "ออฟไลน์".to_string(),
        Some(status) if !status.ready => "ยังไม่พร้อม".to_string(),
        Some(_) => "พร้อมใช้งาน".to_string(),
    };
    let status_ready = move || {
        state
            .status
            .get()
            .is_some_and(|status| status.ollama_online && status.ready)
    };
    // Technical details stay available as a native hover title only.
    let status_title = move || {
        state
            .status
            .get()
            .map(|status| {
                format!(
                    "โมเดล {} · {} บล็อก · {}",
                    status.chat_model, status.chunks, status.ollama_base
                )
            })
            .unwrap_or_default()
    };

    view! {
        <div class="topbar">
            <div class="brand">"Cloud Sec"</div>
            <div class="status-pill" title=status_title>
                <span class="status-dot" class:status-dot-ok=status_ready></span>
                {status_text}
            </div>
        </div>
        <main class="page">
            <h1>"ถามตอบมาตรฐานคลาวด์ไซเบอร์"</h1>
            <p class="lede">
                "ถามจากเอกสารมาตรฐานด้านการรักษาความมั่นคงปลอดภัยไซเบอร์ระบบคลาวด์ พ.ศ. 2567 "
                "คำตอบทุกข้ออ้างอิงเลขข้อและฝ่ายที่รับผิดชอบ (CSC/CSP) จากเอกสารต้นฉบับเท่านั้น"
            </p>
            <form class="ask-row" on:submit=on_submit>
                <input
                    class="ask-input"
                    type="text"
                    placeholder="พิมพ์คําถามเกี่ยวกับมาตรฐาน..."
                    prop:value=move || state.question.get()
                    on:input=on_input
                />
                <button class="btn-primary" type="submit" disabled=move || state.busy.get()>
                    {move || if state.busy.get() { "กำลังคิด" } else { "ถาม" }}
                </button>
            </form>

            <Show when=move || state.busy.get() && state.answer.get().is_empty()>
                <div class="busy-line">
                    "กำลังค้นเอกสารและประมวลผล ครั้งแรกอาจใช้เวลา 20-60 วินาที"
                </div>
            </Show>

            <Show when=move || !state.answer.get().is_empty()>
                <section class="answer-card">
                    <div>{move || state.answer.get()}</div>
                    <div class="answer-meta">{move || stats_line(state)}</div>
                </section>
            </Show>

            <Show when=move || state.error.get().is_some()>
                <div class="error-card">
                    {move || {
                        state
                            .error
                            .get()
                            .map(|error| format!("ไม่สำเร็จ: {}", error.message))
                            .unwrap_or_default()
                    }}
                </div>
            </Show>

            <Show when=move || !state.sources().is_empty()>
                <section class="sources">
                    <h2>"ข้อกำหนดที่ใช้ตอบ"</h2>
                    <For
                        each=move || state.sources()
                        key=|source| format!("{}#{}", source.clause, source.party)
                        children=move |source| {
                            let preview: String = source.text.chars().take(120).collect();
                            view! {
                                <article class="source-card">
                                    <div class="source-head">
                                        <span class="tag">{format!("ข้อ {}", source.clause)}</span>
                                        <span class="tag tag-muted">
                                            {party_label(&source.party)}
                                        </span>
                                        <span class="tag tag-muted">
                                            {format!("หน้า {}", source.page)}
                                        </span>
                                        <span class="tag tag-muted">
                                            {format!("score {:.3}", source.score)}
                                        </span>
                                    </div>
                                    <p class="source-preview">{preview}</p>
                                    <details>
                                        <summary>"ดูข้อความต้นฉบับ"</summary>
                                        <pre class="source-text">{source.text}</pre>
                                    </details>
                                </article>
                            }
                        }
                    />
                </section>
            </Show>

        </main>
    }
}

fn submit(state: AppState) {
    let question = state.question.get_untracked().trim().to_string();
    if question.is_empty() || state.busy.get_untracked() {
        return;
    }
    state.busy.set(true);
    state.answer.set(String::new());
    state.result.set(None);
    state.error.set(None);
    spawn_local(async move {
        match api::ask(question).await {
            Ok(result) => {
                state.answer.set(result.answer.clone());
                state.result.set(Some(result));
            }
            Err(error) => state.error.set(Some(error)),
        }
        // Keep the status pill honest after every attempt: an offline
        // server flips it to "ออฟไลน์" without waiting for a reload.
        if let Ok(status) = api::status().await {
            state.status.set(Some(status));
        }
        state.busy.set(false);
    });
}

fn stats_line(state: AppState) -> String {
    match state.result.get() {
        Some(result) => format!(
            "ดึง context {} โทเคน ใน {} วิ · สร้างคำตอบ {} โทเคน ใน {} วิ · รวม {} วิ",
            result.prefill_tokens,
            secs(result.prefill_ms),
            result.eval_tokens,
            secs(result.eval_ms),
            secs(result.wall_ms),
        ),
        None => {
            if state.busy.get() {
                "กำลังประมวลผล".to_string()
            } else {
                String::new()
            }
        }
    }
}
