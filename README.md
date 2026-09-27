# Cloud Sec

[![License: MIT OR Apache-2.0](https://img.shields.io/badge/License-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)
[![Rust: stable](https://img.shields.io/badge/rust-stable-orange.svg?logo=rust&logoColor=white)](https://www.rust-lang.org/)
[![Tauri v2](https://img.shields.io/badge/Tauri-v2-24c8db.svg?logo=tauri&logoColor=white)](https://tauri.app/)
[![Leptos v0.8](https://img.shields.io/badge/Leptos-v0.8-blue.svg)](https://leptos.dev)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/suradet-ps/cloud-sec/issues)

---

## ◆ PULSE

Another agency is asking where a cloud service stands against the national
cloud cybersecurity standard. Cloud Sec answers from the Royal Gazette
itself: ask in Thai, and the reply carries the clause number, the party it
binds (CSC or CSP), and the page it came from. The document never leaves the
machine, and neither does the question.

| Clause-cited answers ▣ | CSC / CSP separated ▣ | Impact levels ▣ | Fully local ▣ |
|---|---|---|---|

*v0.1.0 - the retrieval loop is sealed and serving.*

> Built with Tauri 2 + Leptos 0.8, judged by `cloud-sec-core`, grounded by
> `cloud-sec-docs`, spoken to by `cloud-sec-ollama` - every answer cites a
> clause and a page, or it says the document does not cover it.
>
> **suradet-ps**, artifact keeper

---

## ◆ IGNITION

One toolchain, two commands.

```
⟫ rustup target add wasm32-unknown-unknown   # via rust-toolchain.toml
⟫ cargo install trunk
⟫ cargo tauri dev
```

The release artifact: `⟫ cargo tauri build`

<details>
<summary>Prerequisites</summary>

- Rust **1.85+** (edition 2024) with the `wasm32-unknown-unknown` target
- [Trunk](https://trunkrs.dev) - installed above
- Tauri 2 system dependencies for your platform
- [Ollama](https://ollama.com) serving `127.0.0.1:11434` with two models
  pulled: `scb10x/typhoon2.5-qwen3-4b` and `bge-m3`

</details>

The retrieval index ships inside the app
(`apps/cloud-sec-app/assets/index.json`, 126 chunks built by the spike at
`tools/rag-spike`). The status pill in the top bar answers one question:
is this machine ready.

---

## ◆ ANATOMY

Five crates, one promise that never bends: a citation, or a refusal.

- **Judges** - `cloud-sec-core` is the pure domain: clause-aware chunking,
  hybrid retrieval (clause filter, party re-rank, impact-level re-rank),
  prompt assembly, intent routing, and mark-insensitive Thai matching -
  testable without a model.
- **Reads** - `cloud-sec-docs` owns the index: page parsing, the per-page
  extraction quality gate, and index load and save.
- **Speaks** - `cloud-sec-ollama` is the only crate allowed to touch the
  network: embeddings and token streaming against the loopback endpoint.
- **Seals** - `cloud-sec-config` pins the models and the measured budgets
  (top-k 3, 2,800-character context, 8,192-token window) in one place.
- **Carries** - `cloud-sec-bridge` is the IPC boundary: typed command
  errors, wire types, and the `answer-token` event stream that the Leptos
  frontend renders as it arrives.

The tools: `tools/rag-spike` is the CLI that proved the pipeline before the
shell existed - clause/party chunking, hybrid retrieval, and a question log
kept in `data/qa-log.txt`.

---

## ◆ RITUALS

**The core ceremony** - one question:

1. Type a question in Thai. The intent router decides the mode: a document
   question enters retrieval, a meta question gets the help text, a greeting
   gets a greeting.
2. Retrieval filters by the clause number if you named one, re-ranks toward
   the party you named, and re-ranks toward the impact level you named.
   Vector score breaks the remaining ties.
3. The answer streams token by token. The source cards carry the clause,
   the party, the page, and the full source text behind a disclosure.
4. When the document does not cover the question, the answer is
   "ไม่พบในเอกสาร". That is a feature, not a failure.

**The ceremony of the two columns** - every requirement in the standard
belongs to the cloud service customer or to the cloud service provider. The
app never merges them, and every answer says which party it binds.

**The ceremony of the local machine** - the only network call is
`127.0.0.1:11434`. No telemetry, no cloud APIs, no remote assets. The
standard is read; it is never written.

---

## ◆ ECHOES

**Where this artifact is heading**

```
v0.1   ▸ ask, retrieve, cite, stream ──────────────────────────────── ▸ sealed
next   ▸ PDF to Markdown ingest - one command to adopt a new edition
next   ▸ latency pass - brevity rules, warmup, source-first events
next   ▸ production index store (sqlite-vec or brute force), measured
```

**Raising the artifact** - read `AGENTS.md` before touching retrieval or
the prompt: it holds the source document reference (SHA-256 pin, clause
addressing, impact levels) and the hard rules. The visual language lives in
`docs/DESIGN.md`.

**Status** - v0.1.0 runs fully local on a 16 GB CPU-only machine: about
110 tok/s prefill, 8 tok/s generation, and 20 to 70 seconds per grounded
answer.

---

```
  ─────────────────────────────────────────
   An agency asked about this cloud service.
   The answer must be a clause, not a guess.
  ─────────────────────────────────────────
```

Licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your
option. Not affiliated with สกมช. or any cloud service provider.
