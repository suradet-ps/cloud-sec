# rag-spike

CLI spike that proves the retrieval pipeline for Cloud Sec: clause-aware
chunking over `cloud-sec.pdf`, local embeddings through Ollama, hybrid
retrieval, and grounded answers with clause citations.

Status: spike complete. Findings feed the `crates/` layout described in the
repository `AGENTS.md`.

## Run

Prerequisites: Ollama running locally with `scb10x/typhoon2.5-qwen3-4b` and
`bge-m3`, plus Python with `pypdf` for the extraction step.

```sh
py scripts/extract_pdf.py      # cloud-sec.pdf -> data/cloud-sec.pages.txt
cargo run --release -- index   # chunk + embed -> data/index.json
cargo run --release -- ask "คําถาม"
```

`ask` appends every question, the retrieved clauses, the answer, and the
timing counters to `data/qa-log.txt` (UTF-8, safe to read on any console).

## Measured on the target machine

16 GB RAM, Intel Core Ultra 5 225, CPU-only inference, no GPU offload.

| Item | Value |
|---|---|
| Index build (126 chunks, bge-m3) | ~55 s once |
| Embedding throughput (warm) | ~0.15 s per short chunk |
| Prefill | 73 to 100 tok/s (1.5k tokens in ~15 s) |
| Generation | ~8 tok/s |
| End-to-end question | 20 to 70 s, depending on context size |
| Thai tokenization | ~0.55 tokens per character |
| Extraction | pages 4 to 24 clean, pages 1 to 3 legacy font and excluded |

## What works

- Clause citations: answers cite clause numbers and pages, for example
  5.2.5.1 and 5.2.3.4.
- Party separation: CSC and CSP requirements are separate chunks.
- Hybrid retrieval: an explicit clause number in the question filters to that
  clause, a named party re-ranks towards its column, and a named impact level
  re-ranks towards the matching rows. Vector search alone clusters tightly on
  this document (all top scores within ~0.03), so the deterministic signals
  are load-bearing.
- Refusal path: questions outside the document answer "ไม่พบในเอกสาร".
- The section 4 table is curated as explicit text (`curated_level_chunks` in
  `src/main.rs`) because the PDF extraction produces column fragments that
  cannot be attributed to a level or party.

## Known limitations to carry into the product

1. The embedded font (THSarabunPSK) maps combining marks into the private use
   area (for example U+F70A, U+F70B, U+F70E). Matching ignores marks on both
   sides, but displayed text still carries the PUA code points. The product
   needs either a different extractor or a font PUA table for clean display.
2. Pages 1 to 3 (the announcement) do not extract as Unicode and are excluded
   by the per-page quality gate. OCR or manual transcription is an open item.
3. Long clauses are split at 1000 characters. Row-aware split points keep the
   impact level rows whole, but other tables may still fragment.
4. The curated level rows are transcribed by hand from PDF pages 8 and 9.
   Re-verify them against the source before trusting or changing them.
5. This spike has no streaming; the answer arrives in one block. The app must
   stream (the first token appears after 10 to 25 seconds of prefill).
