# AGENTS.md - Cloud Sec

## Project Overview

Cloud Sec is a local-first desktop application that puts a local AI to work on a
single normative document: the Thai national standard for cloud cybersecurity
(มาตรฐานด้านการรักษาความมั่นคงปลอดภัยไซเบอร์ระบบคลาวด์ พ.ศ. ๒๕๖๗), published by
the National Cyber Security Committee in the Royal Gazette in September 2024.
The application reads `cloud-sec.pdf` as its primary and only grounding source
and helps an analyst:

- look up what the standard requires, clause by clause
- separate the duties of the cloud service customer (CSC) from those of the
  cloud service provider (CSP)
- determine which clauses apply at a given impact level (low, medium, high)
- map clauses to the certification tracks they feed (self-assessment,
  attestation, Certify Body, ISO/IEC and CSA STAR references)
- assemble evidence checklists and gap notes per clause

The exact feature set, UX flows, and domain model rationale live in
**docs/PRODUCT.md** (not yet written). This file covers agent-facing
conventions: stack, boundaries, build commands, coding rules, and the document
reference needed to work on this repository. Do not duplicate the docs content
here - link to it.

The product has one promise: an answer is only as good as its citation. Every
substantive statement the AI produces must point back to a clause and page of
the source document.

## Tech Stack

- **Shell:** Tauri 2 (desktop; Windows is the primary target)
- **Frontend:** Leptos 0.8, CSR (client-side rendered, compiled to WASM via
  Trunk), no npm toolchain
- **Local AI runtime:** Ollama over HTTP on `127.0.0.1:11434` (see the
  AI Layer section; integration details are still open)
- **Document pipeline:** pure Rust (PDF text extraction, chunking, retrieval)
- **Storage:** local files under the platform app-data directory plus a local
  index (SQLite planned)
- **Rust:** edition 2024, workspace layout, shared dependencies via
  `[workspace.dependencies]`
- **Documentation convention:**
  - `docs/DESIGN.md` - the visual design system: paper-white canvas, pill
    geometry, hairline borders, no shadows, and exactly one semantic color
    (the status green on the ready dot). The UI implements this file, and
    the palette stays closed: a new color is a change to `docs/DESIGN.md`,
    the CSS variables, and this file in the same commit.
  - `docs/PRODUCT.md` - product scope, UX flows, data model, retrieval and
    prompt rationale (not yet written)
  - `AGENTS.md` - this file, agent-facing conventions and rules
  - `docs/AGENTS-RUST.md` - Rust style, lint, testing, and CI baseline (not yet
    written; adapt from the med-recon workspace)

This repository follows the same shell and frontend conventions as the sibling
med-recon workspace (Tauri 2 + Leptos 0.8 CSR compiled by Trunk). Any deviation
must be recorded in this file before it is introduced.

## Core Constraints

1. **Local first.** No outbound network calls at runtime except HTTP to the
   local Ollama endpoint on loopback. No telemetry, no cloud APIs, no remote
   fonts or assets.

2. **The source document is the truth.** `cloud-sec.pdf` is normative input.
   Never edit, rewrite, or re-export it; the app may only read it. If the file
   is replaced by a newer edition, treat that as a new document version: the
   SHA-256 pin changes, the index is rebuilt, and citations are re-verified.

3. **Grounding and citation.** Every AI answer that states a requirement must
   cite the clause identifier (for example `5.2.3.4`) and the source page. If
   retrieval finds no relevant clause, the answer must say the document does not
   cover the question. Raw source text must be reachable from the UI next to any
   AI statement. No improvised requirements, ever.

4. **Party separation.** Each clause in the standard carries two columns of
   requirements: ผู้ใช้บริการคลาวด์ (Cloud Service Customer, CSC) and
   ผู้ให้บริการคลาวด์ (Cloud Service Provider, CSP). An analysis must never mix
   the two. Any generated checklist or answer must state which party the
   requirement binds.

5. **Impact level matters.** The standard's annex section 4 defines minimum
   requirement sets per impact level (low, medium, high) and a certification
   track for each. The app must treat the selected impact level as an input that
   filters what applies. It must never present the full clause set as mandatory
   for every organization.

6. **Thai first.** The document, the users, and the answers are Thai. UI copy is
   Thai. Code, comments, and repository documentation are English. AI answers
   default to Thai unless the user asks otherwise.

7. **Hardware budget.** Target machines are CPU-inference boxes with 16 GB of
   RAM and no discrete GPU. Retrieval must stay small (default top-k 4 chunks)
   and generation context must stay capped (default 8192 tokens). Any change to
   these budgets requires a measured note. Long-context prompts are the enemy of
   the interaction loop on this class of hardware.

8. **House style: no em dashes.** No em dash (U+2014) and no en dash (U+2013) in
   any repository file: documentation, code comments, commit messages, UI copy.
   Use an ASCII hyphen, a colon, or restructure the sentence. This rule applies
   to generated text as well.

9. **No secrets.** The app has no accounts and no credentials of its own. If
   sensitive configuration is ever added it must follow the encryptman
   conventions used in med-recon (AES-256-GCM with the master key in the OS
   keychain). Never write plaintext credentials to disk or logs.

## Source Document Reference

### Identity

- **Title:** ประกาศคณะกรรมการการรักษาความมั่นคงปลอดภัยไซเบอร์แห่งชาติ เรื่อง
  มาตรฐานด้านการรักษาความมั่นคงปลอดภัยไซเบอร์ระบบคลาวด์ พ.ศ. ๒๕๖๗
- **English working title:** Notification of the National Cyber Security
  Committee on the Cloud Cybersecurity Standard, B.E. 2567 (2024)
- **Published:** Royal Gazette (ราชกิจจานุเบกษา), Volume 141, Special Part 248 ง,
  10 September 2024
- **Publisher:** สำนักงานคณะกรรมการการรักษาความมั่นคงปลอดภัยไซเบอร์แห่งชาติ
  (สกมช., NCSA)
- **File:** `cloud-sec.pdf` (repository root), 24 pages
- **SHA-256:** `7e93fca5e7a906e72cf1b5eb53e94ac74137e511e57e399235f412ca2c8a7bc3`

Legal effect: clause 2 of the announcement states when the standard takes
effect. Pages 1 to 3 are the announcement text and currently do not extract as
Unicode (see below), so verify any quoted date against the original gazette
before it appears in the UI.

### Extraction quality

- Pages 1 to 3 (the announcement, clauses 1 to 8) are set in a legacy-encoded
  font and extract as mojibake.
- Pages 4 to 24 (the annex, `แนบท้าย`, containing the actual standard) extract
  as clean Unicode Thai.
- The ingestion pipeline must score extraction quality per page and report pages
  that fail. The app must never ground an answer on garbled text.

### Structure (annex)

- **1. Introduction** - rationale, objectives, legal authority (มาตรา 9(4),
  Cybersecurity Act B.E. 2562), the CSC/CSP risk split, the shared
  responsibility framework, and the certification process and frequency
- **2. Scope** - government agencies, regulators, critical information
  infrastructure, and public cloud providers serving them
- **3. Normative references** - ISO/IEC 27017:2015, ISO/IEC 27018:2019,
  ISO/IEC 22123-1:2023, and three related Thai announcements
- **4. Minimum requirements and certification** - the impact level table
- **5. The standard:**
  - **5.1 Cloud Security Governance**
    - 5.1.1 Information Security Policies
    - 5.1.2 Organization of Information Security (roles and responsibilities;
      contact with authorities)
    - 5.1.3 Compliance (applicable legislation; intellectual property rights;
      protection of records; regulation of cryptographic controls; independent
      review)
  - **5.2 Cloud Infrastructure Security and Operation**
    - 5.2.1 Human Resource Security (awareness, education, and training)
    - 5.2.2 Asset Management (inventory of assets; labelling of information)
    - 5.2.3 Access Control (networks; user registration and deregistration;
      user access provisioning; privileged access rights; secret
      authentication information; information access restriction; privilege
      utility programs; secure log-on procedures)
    - 5.2.4 Cryptography (policy on the use of cryptographic controls; key
      management)
    - 5.2.5 Physical and Environment Security (data center location; secure
      disposal or reuse of equipment)
    - 5.2.6 Operations Security (change management; capacity management;
      information backup; event logging; protection of log information;
      administrator and operator logs; clock synchronization; management of
      technical vulnerabilities; separation of development, testing, and
      operational environments)
    - 5.2.7 Communication Security (information transfer policies and
      procedures; segregation in networks)
    - 5.2.8 System Acquisition, Development, and Maintenance (information
      security requirements analysis and specification; secure development
      policy)
    - 5.2.9 Supplier Relationships (policy; addressing security within supplier
      agreements; ICT supply chain)
    - 5.2.10 Information Security Incident Management (responsibilities and
      procedures; reporting information security events)

### Clause addressing

- Canonical identifier: Arabic numerals, dotted to the deepest level used in the
  document, for example `5.2.3.4`. Thai numerals (๕.๒.๓.๔) appear in print and
  are display-only.
- Page citation: PDF page number (1 to 24), with the annex page label when one
  exists.
- Requirement identity is the pair (clause, party). The same clause number can
  produce two different requirements, one per column.

### Impact levels (annex section 4)

- **Low:** Part 1 only 5.1.1 and 5.1.2; Part 2 only 5.2.1, 5.2.2, 5.2.3, 5.2.4,
  5.2.8, 5.2.9. CSC: self-assessment with at least an annual review. CSP:
  certification by a Certify Body on a 3-year cycle plus ISO/IEC 27001 and CSA
  STAR Level 1 / CCM Lite at minimum.
- **Medium:** Part 1 all clauses; Part 2 5.2.1, 5.2.2, 5.2.3, 5.2.4, 5.2.7,
  5.2.8, 5.2.9, 5.2.10. CSC: attestation or certification on a 3-year cycle
  (audit in year 1, surveillance in years 2 and 3). CSP: certification plus CSA
  STAR Level 2 / CCM and ISO/IEC 27701 at minimum.
- **High:** all clauses. CSC: certification by a Certify Body on a 3-year cycle.
  CSP: certification plus ISO/IEC 27017 or CSA STAR Level 2 / CCM, ISO/IEC
  27018, and ISO/IEC 27701 at minimum.

## Repository and Workspace Layout

The workspace is built: five crates, the Tauri app, and the Leptos frontend,
mirroring med-recon. The app ships a prebuilt index asset produced by the
spike (`tools/rag-spike/data/index.json` copied to
`apps/cloud-sec-app/assets/index.json`); runtime index building is not wired
yet.

```
cloud-sec/
├── AGENTS.md
├── Cargo.toml                    workspace root
├── rust-toolchain.toml           pinned toolchain
├── deny.toml                     cargo-deny policy (before first release)
├── cloud-sec.pdf                 normative source document (read-only)
├── tools/
│   └── rag-spike/                retrieval spike (CLI, non-streaming)
├── apps/
│   └── cloud-sec-app/            Tauri 2 shell
│       ├── build.rs, tauri.conf.json, Trunk.toml
│       ├── src/                  commands.rs, state.rs, main.rs, lib.rs
│       └── frontend/             Leptos 0.8 CSR crate (cloud-sec-frontend)
│           ├── Cargo.toml, index.html
│           └── src/              main.rs, app.rs, state.rs, api.rs, components/
└── crates/
    ├── cloud-sec-core/           pure domain, no IO
    ├── cloud-sec-bridge/         IPC invoke wrapper and wire types
    ├── cloud-sec-docs/           extraction, chunking, indexing, retrieval
    ├── cloud-sec-ollama/         local Ollama HTTP client
    └── cloud-sec-config/         settings and defaults
```

Crate responsibilities and rules:

| Crate | Responsibility | Rules |
|---|---|---|
| cloud-sec-core | Clause model, party enum, impact level enum, citation formatting, prompt assembly rules, answer validation | Pure Rust, no IO, no async runtime, unit tested without a database or model |
| cloud-sec-bridge | Invoke Tauri commands from WASM; serde wire types shared by frontend and backend | Compiles to a stub on native; no domain logic |
| cloud-sec-docs | PDF extraction, text quality checks, chunking, embedding index, top-k retrieval | Owns all document IO; deterministic chunk ids keyed to the document hash |
| cloud-sec-ollama | Chat streaming and embeddings against `127.0.0.1:11434` | Model names and URLs injected from config; timeouts on every call; typed errors |
| cloud-sec-config | Model names, context budget, top-k, UI preferences | Single source of default values; no secret material |

Boundary rules:

- The frontend never talks to Ollama directly. All model access goes through
  Tauri commands.
- Commands are thin: validate input, call a crate, map errors. No business logic
  in `commands.rs` beyond error mapping.
- `cloud-sec-core` is the judge: retrieval scoring, citation formatting, and
  answer validation live there so they are testable without a model.
- No crate may touch the network except `cloud-sec-ollama`.

## AI Layer (verified by the spike)

The pipeline below was validated end to end by the spike at `tools/rag-spike`
(see `tools/rag-spike/README.md` for the run log and known limitations). The
model names and budgets below are pinned by measurement on the target machine
class, not preferences.

- **Runtime:** Ollama 0.34.0 on `127.0.0.1:11434`, CPU-only inference (the
  integrated GPU is not used and offload must stay off).
- **Chat model:** `scb10x/typhoon2.5-qwen3-4b` (2.5 GB pull). Thai and English.
- **Embedding model:** `bge-m3` (1.2 GB pull, 1024 dimensions).
- **Measured budgets** (16 GB RAM, Core Ultra 5 225): prefill 73 to 100 tok/s,
  generation about 8 tok/s, embedding about 0.15 s per chunk, index build about
  55 s for 126 chunks. Thai text costs about 0.55 tokens per character.
- **Context budget:** keep the assembled prompt at or below roughly 1500 to
  2000 tokens, which means 15 to 25 seconds of prefill. Top-k default 3 with
  retrieved text capped at about 2800 characters. `num_ctx` stays at 8192.
  Context sizes above 4000 tokens are unusable on this hardware class.
- **Retrieval is hybrid, deterministic first.** Vector similarity alone
  clusters tightly on this document, so retrieval applies, in order: an exact
  clause number found in the question filters candidates to that clause; a
  named party re-ranks towards its column; a named impact level re-ranks
  towards the matching rows. The vector score breaks remaining ties.
- **Chunking:** one chunk per (clause, party), maximum 1000 characters, with
  row-aware split points so impact level rows stay whole. Requirement identity
  is always the pair (clause, party).
- **Refusal path:** when retrieval finds nothing relevant, the app answers
  "ไม่พบในเอกสาร" instead of guessing. This behavior was validated.
- **Intent routing comes before retrieval.** A deterministic classifier in
  `cloud_sec_core::intent` separates three modes: help questions ("ทำอะไรได้บ้าง"),
  greetings, and document questions. Help and greeting intents return canned
  Thai answers with no model call and no retrieval, because forcing them
  through search pulls a random clause and the model then refuses. Only
  document questions enter the retrieval pipeline.
- **Streaming is mandatory.** The backend emits Tauri events per token and the
  UI renders partial text. On this hardware the first token arrives after 10 to
  25 seconds of prefill; the UI must communicate that wait.
- **No arithmetic on clause numbers by the model.** Clause logic (impact level
  applicability, party filtering, certification mapping) is computed in
  `cloud-sec-core` and passed to the model as facts.
- **The section 4 level table is curated structured data,** not raw extracted
  text. The PDF extraction produces column fragments that cannot be attributed
  to a level or party. The curated rows are transcribed from PDF pages 8 and 9
  and must be re-verified against the source on any change.
- **No tool calling, function calling, or autonomous loops** until designed and
  documented here.

Remaining open questions: the app index store (JSON in the spike, SQLite in the
app), the production text extraction fix for the PUA combining marks, whether
section summaries are precomputed, and the event protocol for streaming.

## Development Workflow

Prerequisites:

- Rust toolchain pinned by `rust-toolchain.toml` (edition 2024) plus the
  `wasm32-unknown-unknown` target
- Trunk (`cargo install trunk`)
- Tauri 2 platform dependencies
- Ollama for AI features (optional during pure UI work)

Commands:

```sh
# first run
rustup target add wasm32-unknown-unknown
cargo install trunk
cargo install tauri-cli --version "^2"

# develop
cargo tauri dev      # builds the Leptos frontend via beforeDevCommand and opens the app

# release
cargo tauri build
```

Quality gates before claiming completion, in order:

```sh
cargo fmt --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo check -p cloud-sec-frontend --target wasm32-unknown-unknown
cargo deny check     # once deny.toml exists
```

- Frontend tests that need a browser run under `wasm-bindgen-test` with
  chromedriver; native `cargo test` cannot execute them. At minimum the frontend
  must always pass the wasm32 check.
- CI, when added, mirrors med-recon: fmt, clippy, host tests for backend crates,
  and a separate wasm build and test job for the frontend.

## Open Items to Resolve Before Implementation

- [x] Ollama decisions recorded and measured (see the AI Layer section).
- [x] Retrieval behavior validated by `tools/rag-spike`: clause filter, party
      re-rank, level re-rank, refusal path. Chunking is per (clause, party).
- [x] Section 4 level rows curated as structured text, transcribed from PDF
      pages 8 and 9.
- [x] Workspace scaffolded and verified: fmt, check, clippy, tests, and the
      wasm32 frontend check all pass; `cargo tauri dev` launches and loads
      the 126-chunk index.
- [x] Streaming protocol: the backend emits `answer-token` per fragment; the
      frontend subscribes through `cloud_sec_bridge::events::listen`.
- [x] Citation format shown in the UI: clause chip, party label, PDF page,
      score, and the full source text behind a disclosure.
- [ ] Write `docs/PRODUCT.md` (product scope, screens, data model, retrieval
      and prompt rationale). The visual system lives in `docs/DESIGN.md`;
      the spike README is the retrieval starting material.
- [ ] Write `docs/AGENTS-RUST.md` (adapt the med-recon baseline).
- [ ] Production text extraction: the PDF font (THSarabunPSK) maps combining
      marks into the PUA block, so extracted text carries private use code
      points and pages 1 to 3 do not extract at all. Decide the fix: a
      different extractor, a font PUA normalization table, or OCR for pages 1
      to 3. This blocks clean display and citation rendering.
- [ ] Production index build: v0 ships a prebuilt JSON asset from the spike.
      Wire `cloud_sec_docs::build_document_chunks` into an ingest path once
      the extraction fix lands, then decide the on-disk store (SQLite with
      brute force cosine versus an extension such as sqlite-vec) with a
      measured note for this hardware class.
- [ ] Decide whether the app bundles the PDF or asks the user to select the
      file at first run.
- [ ] Decide how the impact level is captured in the UI and stored.
- [ ] Source text still carries the PUA combining marks, so the "ดูข้อความ
      ต้นฉบับ" view may render tone marks as missing glyphs. This shares the
      extraction fix item above.
- [ ] Initial git repository setup and license selection (med-recon uses
      MIT OR Apache-2.0).
