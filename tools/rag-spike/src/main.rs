//! CLI spike: clause-aware retrieval over cloud-sec.pdf through local Ollama.
//!
//! Usage:
//!   cargo run --release -- index
//!   cargo run --release -- ask "คําถาม"

mod chunk;
mod ollama;
mod search;

use anyhow::{Context, Result, bail};
use chunk::{Chunk, chunk_pages, parse_pages};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::time::Instant;

const CHAT_MODEL: &str = "scb10x/typhoon2.5-qwen3-4b";
const EMBED_MODEL: &str = "bge-m3";
const TOP_K: usize = 3;
const EMBED_BATCH: usize = 16;
const NUM_CTX: u32 = 8192;
/// Retrieved context is capped so prefill stays around 15 seconds on CPU.
const MAX_CONTEXT_CHARS: usize = 2800;
/// Pages below this Thai ratio are never indexed.
const MIN_THAI_RATIO: f32 = 0.30;

const SYSTEM_PROMPT: &str = "\
คุณเป็นผู้ช่วยวิเคราะห์มาตรฐานด้านการรักษาความมั่นคงปลอดภัยไซเบอร์ระบบคลาวด์ พ.ศ. 2567
ตอบคำถามโดยใช้เฉพาะข้อมูลใน \"เอกสารอ้างอิง\" ที่ให้มาเท่านั้น
กฎ:
1. ทุกข้อความที่อ้างถึงข้อกำหนด ต้องระบุเลขข้อ (เช่น 5.2.3.4) และฝ่ายที่รับผิดชอบ (ผู้ใช้บริการคลาวด์ หรือ ผู้ให้บริการคลาวด์)
2. ถ้าเอกสารอ้างอิงไม่ครอบคลุมคำถาม ให้ตอบว่า \"ไม่พบในเอกสาร\" แล้วบอกว่าข้อใดใกล้เคียงที่สุด
3. ถ้าคำถามระบุระดับผลกระทบ (ต่ำ/กลาง/สูง) ให้ตอบเฉพาะข้อกำหนดของระดับนั้น ห้ามนำข้อกำหนดของระดับอื่นมาตอบ ถ้าข้อมูลที่ให้มาไม่พอ ให้บอกว่าไม่พบข้อมูลของระดับนั้น
4. ถ้าคำถามถามว่าต้องปฏิบัติตามข้อกำหนดใด ให้ระบุเลขข้อทั้งหมดที่ปรากฏในเอกสารอ้างอิงของระดับหรือฝ่ายนั้นให้ครบ
5. ตอบเป็นภาษาไทย กระชับ ตรงประเด็น ไม่แต่งเติมข้อกำหนดขึ้นเอง";

fn data_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("data")
}

fn party_label(party: &str) -> &'static str {
    match party {
        "csc" => "ผู้ใช้บริการคลาวด์ (CSC)",
        "csp" => "ผู้ให้บริการคลาวด์ (CSP)",
        _ => "ทั้งสองฝ่าย",
    }
}

/// Hand-transcribed rows of the annex section 4 table (PDF pages 8 to 9).
///
/// The table extracts as column fragments that the model cannot attribute to
/// a level or a party, so the rows are stored explicitly. Every value here is
/// transcribed from the source table; re-verify against the PDF before any
/// change.
fn curated_level_chunks() -> Vec<Chunk> {
    let rows = [
        (
            "low",
            "ผลกระทบระดับต่ำ: ข้อกำหนดขั้นต่ำ ส่วนที่ 1 (การกำกับดูแล) เฉพาะข้อ 5.1.1, 5.1.2; \
ส่วนที่ 2 (การปฏิบัติการ) เฉพาะข้อ 5.2.1, 5.2.2, 5.2.3, 5.2.4, 5.2.8, 5.2.9. \
ผู้ใช้บริการคลาวด์ (CSC): ประเมินตนเอง (Self-assessment) พร้อมแนบหลักฐานและขออนุมัติไปยังผู้บริหารสูงสุดของหน่วยงาน โดยเก็บรักษาไว้ที่หน่วยงานและส่งให้สำนักงาน และทบทวนอย่างน้อยปีละ 1 ครั้ง. \
ผู้ให้บริการคลาวด์ (CSP): ได้รับการรับรองโดยหน่วยงานให้บริการตรวจรับรอง (Certify Body) ตามวงรอบ 3 ปี (ตรวจรับรองปีที่ 1, ตรวจสำรวจปีที่ 2 และ 3) และได้รับการรับรองตามมาตรฐาน ISO/IEC 27001 Certification และ CSA STAR Level 1/CCM Lite เป็นอย่างน้อย",
        ),
        (
            "medium",
            "ผลกระทบระดับกลาง: ข้อกำหนดขั้นต่ำ ส่วนที่ 1 (การกำกับดูแล) ทุกข้อ; \
ส่วนที่ 2 (การปฏิบัติการ) เฉพาะข้อ 5.2.1, 5.2.2, 5.2.3, 5.2.4, 5.2.7, 5.2.8, 5.2.9, 5.2.10. \
ผู้ใช้บริการคลาวด์ (CSC): ได้รับการรับรองโดยหน่วยงานควบคุมหรือกำกับดูแล (Attestation) หรือได้รับการรับรองโดยหน่วยงานให้บริการตรวจรับรอง (Certify Body) ตามวงรอบ 3 ปี (ตรวจรับรองปีที่ 1, ตรวจสำรวจปีที่ 2 และ 3). \
ผู้ให้บริการคลาวด์ (CSP): ได้รับการรับรองโดยหน่วยงานให้บริการตรวจรับรอง (Certify Body) ตามวงรอบ 3 ปี และได้รับการรับรองตามมาตรฐาน CSA STAR Level 2/CCM และ ISO/IEC 27701 Certification เป็นอย่างน้อย",
        ),
        (
            "high",
            "ผลกระทบระดับสูง: ข้อกำหนดขั้นต่ำ ทุกข้อทั้งส่วนที่ 1 และส่วนที่ 2. \
ผู้ใช้บริการคลาวด์ (CSC): ได้รับการรับรองโดยหน่วยงานให้บริการตรวจรับรอง (Certify Body) ตามวงรอบ 3 ปี (ตรวจรับรองปีที่ 1, ตรวจสำรวจปีที่ 2 และ 3). \
ผู้ให้บริการคลาวด์ (CSP): ได้รับการรับรองโดยหน่วยงานให้บริการตรวจรับรอง (Certify Body) ตามวงรอบ 3 ปี และได้รับการรับรองตามมาตรฐาน ISO/IEC 27017 Certification หรือ CSA STAR Level 2/CCM และ ISO/IEC 27018 Certification และ ISO/IEC 27701 Certification เป็นอย่างน้อย",
        ),
    ];
    rows.iter()
        .map(|(level, text)| Chunk {
            id: format!("4#curated-{level}"),
            clause: "4".to_string(),
            heading: "ข้อกำหนดขั้นต่ำและการตรวจรับรอง (ตาราง แปลงเป็นข้อความ)".to_string(),
            party: "both".to_string(),
            page: 8,
            text: (*text).to_string(),
            embed_text: format!(
                "มาตรฐานความมั่นคงปลอดภัยไซเบอร์ระบบคลาวด์ พ.ศ. 2567\nข้อ 4: ข้อกำหนดขั้นต่ำและการตรวจรับรอง\n{text}"
            ),
            vector: Vec::new(),
        })
        .collect()
}

fn cmd_index() -> Result<()> {
    let pages_path = data_dir().join("cloud-sec.pages.txt");
    let raw = fs::read_to_string(&pages_path)
        .with_context(|| format!("read {}", pages_path.display()))?;
    let pages = parse_pages(&raw)?;
    let total = pages.len();
    let skipped: Vec<u32> = pages
        .iter()
        .filter(|p| p.thai_ratio < MIN_THAI_RATIO)
        .map(|p| p.no)
        .collect();
    let usable: Vec<_> = pages
        .iter()
        .filter(|p| p.thai_ratio >= MIN_THAI_RATIO)
        .cloned()
        .collect();

    let mut chunks = chunk_pages(&usable);
    chunks.extend(curated_level_chunks());
    if chunks.is_empty() {
        bail!("no chunks produced; run scripts/extract_pdf.py first");
    }
    let total_chars: usize = chunks.iter().map(|c| c.text.chars().count()).sum();
    println!(
        "pages: {} total, {} indexed, skipped low quality: {:?}",
        total,
        usable.len(),
        skipped
    );
    println!(
        "chunks: {} | avg {} chars | max {} chars",
        chunks.len(),
        total_chars / chunks.len(),
        chunks.iter().map(|c| c.text.chars().count()).max().unwrap_or(0)
    );

    let agent = ollama::agent();
    let started = Instant::now();
    let chunk_count = chunks.len();
    let mut done = 0;
    for batch in chunks.chunks_mut(EMBED_BATCH) {
        let texts: Vec<String> = batch.iter().map(|c| c.embed_text.clone()).collect();
        let vectors = ollama::embed(&agent, EMBED_MODEL, &texts)?;
        for (chunk, vector) in batch.iter_mut().zip(vectors) {
            chunk.vector = vector;
        }
        done += batch.len();
        println!("embedded {done}/{chunk_count}");
    }
    let index_path = data_dir().join("index.json");
    fs::write(&index_path, serde_json::to_string(&chunks)?)
        .with_context(|| format!("write {}", index_path.display()))?;
    println!(
        "index written to {} | embedding took {:.1}s",
        index_path.display(),
        started.elapsed().as_secs_f32()
    );
    Ok(())
}

fn load_index() -> Result<Vec<Chunk>> {
    let path = data_dir().join("index.json");
    let raw =
        fs::read_to_string(&path).with_context(|| format!("read {} (run `index` first)", path.display()))?;
    Ok(serde_json::from_str(&raw)?)
}

fn build_user_prompt(hits: &[(usize, f32)], chunks: &[Chunk]) -> (String, usize) {
    let mut context = String::new();
    let mut used_chars = 0;
    let mut used = 0;
    for (rank, (idx, score)) in hits.iter().enumerate() {
        let chunk = &chunks[*idx];
        let block = format!(
            "[{}] ข้อ {} ({}) หน้า {} ฝ่าย {} (score {:.3})\n{}\n\n",
            rank + 1,
            chunk.clause,
            chunk.heading,
            chunk.page,
            party_label(&chunk.party),
            score,
            chunk.text,
        );
        let block_chars = block.chars().count();
        if used_chars + block_chars > MAX_CONTEXT_CHARS && used > 0 {
            break;
        }
        context.push_str(&block);
        used_chars += block_chars;
        used += 1;
    }
    (context, used)
}

fn cmd_ask(question: &str) -> Result<()> {
    let chunks = load_index()?;
    let agent = ollama::agent();

    let embedded = ollama::embed(&agent, EMBED_MODEL, &[question.to_string()])?;
    let query = embedded.into_iter().next().context("no query embedding")?;
    let outcome = search::search(question, &query, &chunks, TOP_K);
    let hits = outcome.hits;

    println!(
        "=== retrieved (clause filter: {}, party filter: {}, level filter: {}) ===",
        outcome.clause_filter.as_deref().unwrap_or("none"),
        outcome.party_filter.unwrap_or("none"),
        outcome.level_filter.unwrap_or("none"),
    );
    for (rank, (idx, score)) in hits.iter().enumerate() {
        let chunk = &chunks[*idx];
        let preview: String = chunk.text.chars().take(90).collect();
        println!(
            "[{}] ข้อ {} | {} | หน้า {} | score {:.3}\n    {}",
            rank + 1,
            chunk.clause,
            chunk.party.to_uppercase(),
            chunk.page,
            score,
            preview.replace('\n', " "),
        );
    }

    let (context, used) = build_user_prompt(&hits, &chunks);
    let user_prompt = format!("เอกสารอ้างอิง:\n\n{context}---\n\nคําถาม: {question}");
    if used == 0 {
        bail!("no context fit the budget");
    }

    println!("=== answer (context chunks: {used}) ===");
    let started = Instant::now();
    let stats = ollama::chat(&agent, CHAT_MODEL, SYSTEM_PROMPT, &user_prompt, NUM_CTX)?;
    let wall = started.elapsed().as_secs_f32();
    println!("{}", stats.text);
    println!(
        "--- prefill {} tok in {:.1}s ({:.1} tok/s) | gen {} tok in {:.1}s ({:.1} tok/s) | wall {:.1}s",
        stats.prompt_tokens,
        stats.prompt_ns as f32 / 1e9,
        stats.prompt_tokens as f32 / (stats.prompt_ns as f32 / 1e9).max(0.001),
        stats.eval_tokens,
        stats.eval_ns as f32 / 1e9,
        stats.eval_tokens as f32 / (stats.eval_ns as f32 / 1e9).max(0.001),
        wall,
    );

    let log_path = data_dir().join("qa-log.txt");
    let mut log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
        .with_context(|| format!("open {}", log_path.display()))?;
    writeln!(
        log,
        "===== Q: {question}\n--- retrieved: {}\n--- answer:\n{}\n--- prefill {} tok / gen {} tok / wall {:.1}s\n",
        hits.iter()
            .map(|(idx, score)| format!("{} {:.3}", chunks[*idx].clause, score))
            .collect::<Vec<_>>()
            .join(" | "),
        stats.text,
        stats.prompt_tokens,
        stats.eval_tokens,
        wall,
    )?;
    println!("--- logged to {}", log_path.display());
    Ok(())
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("index") => cmd_index(),
        Some("ask") => {
            let question = args[1..].join(" ");
            if question.trim().is_empty() {
                bail!("usage: cargo run --release -- ask \"คําถาม\"");
            }
            cmd_ask(question.trim())
        }
        _ => {
            println!("usage:");
            println!("  cargo run --release -- index");
            println!("  cargo run --release -- ask \"คําถาม\"");
            Ok(())
        }
    }
}
