# Cloud Sec - Design System

## Overview

Cloud Sec is a single-screen desktop tool, and the design language follows
from that: one reading column on a paper-white canvas, a quiet top bar, and
components that stay out of the analyst's way. The system descends from a
flat, documentation-grade aesthetic (paper-white, pill geometry, hairline
dividers, no shadows), and it stays deliberately close to it: the chrome
should read like the standard's own document, not like an app wrapped
around it.

The screen has four moments, top to bottom:

1. **Top bar** - the brand and one status pill. Nothing else. Technical
   details (model name, chunk count, endpoint) live in the pill's hover
   title, never in the layout.
2. **Ask row** - a pill input and a single black pill button.
3. **Answer card** - the streamed answer in body type, with a small timing
   line under it.
4. **Source cards** - what the answer was grounded in: clause number,
   party, page, score, and the full source text behind a disclosure.

Everything is one column, 860px at most, centered. There are no tabs, no
sidebars, no settings screen.

**Key Characteristics:**

- Paper-white canvas end to end, no surface alternation
- Pill geometry (`9999px`) for every interactive element; `12px` for cards
- Hairline borders (`1px`) and no drop shadows anywhere
- Exactly one semantic color: the status green on the ready dot
- Thai-first typography set in the operating system's sans; monospace only
  for identifiers, tags, and timing values
- Streaming text rendered as plain pre-wrapped prose, never as markdown

## Colors

All colors are defined once as CSS custom properties in
`apps/cloud-sec-app/frontend/style.css`. The palette is closed: adding a
color requires updating this file and AGENTS.md together.

### Brand & Action

- **Ink** (`--ink`, `#000000`): the single action color. Every primary
  button, every heading, every clause tag value.
- **Ink Deep** (`--ink-deep`, `#090909`): pressed state for the primary
  button.

### Surface

- **Canvas** (`--canvas`, `#ffffff`): the page, every card, the input.
- **Soft Surface** (`--surface-soft`, `#fafafa`): status pill, tag chips,
  disabled button.
- **Hairline** (`--hairline`, `#e5e5e5`): card borders, top bar divider,
  input border at rest.
- **Hairline Strong** (`--hairline-strong`, `#d4d4d4`): the error card
  border, the only stronger divider in use.
- **Surface Dark** (`--surface-dark`, `#171717`): reserved for one future
  inverted surface (for example an offline banner). Currently unused; do
  not deploy it without documenting the moment it marks.

### Text

- **Ink** (`--ink`): headings, primary button text, tag values.
- **Charcoal** (`--charcoal`, `#525252`): status pill text, source
  previews, error text.
- **Body** (`--body`, `#737373`): the lede, busy line, source detail text.
- **Mute** (`--mute`, `#a3a3a3`): the timing meta line, inactive status
  dot.

### Semantic

- **Status OK** (`--status-ok`, `#16a34a`): the ready status dot, and
  nothing else. This is the only non-monochrome color in the system.
- **Focus Ring** (`--focus-ring`, `rgba(59, 130, 246, 0.5)`): the
  browser-default focus ring on the input, plus a 1px ink border. The only
  blue in the system.

## Typography

### Font Stacks

- **Display** (`--font-display`): `"SF Pro Rounded", system-ui,
  -apple-system, "Segoe UI", sans-serif`. Used for the brand and the page
  heading only.
- **Body** (`--font-body`): `ui-sans-serif, system-ui, -apple-system,
  "Segoe UI", "Leelawadee UI", sans-serif`. Carries all Thai text.
- **Mono** (`--font-mono`): `ui-monospace, SFMono-Regular, Menlo, Consolas,
  monospace`. Clause tags, timing values, the status pill title.

### Hierarchy

| Role | Size | Weight | Line Height | Font | Use |
|---|---|---|---|---|---|
| Page heading | 30px | 500 | 1.2 | Display | "ถามตอบมาตรฐานคลาวด์ไซเบอร์" |
| Brand | 18px | 600 | 1.2 | Display | Top bar |
| Section heading | 18px | 500 | 1.4 | Body | "ข้อกำหนดที่ใช้ตอบ" |
| Body | 16px | 400 | 1.5 | Body | Lede, input, answer (line height 1.6) |
| Body small | 14px | 400 | 1.43 | Body | Busy line, source preview, source text |
| Button | 14px | 500 | 1 | Body | "ถาม" |
| Tag | 14px | 400 | 1.4 | Mono | Clause, party, page, score chips |
| Caption | 12px | 400 | 1.33 | Mono | Timing meta line |
| Caption quiet | 12px | 400 | 1.33 | Body | Status pill text |

### Principles

The display face appears at two sizes and nowhere else. Everything below
20px is the system sans, which renders Thai correctly without a bundled
font. Monospace is the visual signal for "this is an identifier or a
measurement", which keeps clause numbers and timings scannable inside Thai
prose.

## Layout

### Spacing System

- Base unit: 8px, with 4px and 12px steps for inline gaps.
- Page padding: 40px top, 24px sides, 72px bottom.
- Content column: `max-width: 860px`, centered.
- Top bar: fixed 56px height, 24px horizontal padding, hairline bottom
  border.
- Card padding: 24px for the answer card, 16px for source and error cards.
- Vertical rhythm between blocks: 12px (cards in a list), 32px (answer
  card, busy line), 40px (sources section, 16px to its first card).

### Grid & Container

One column, always. The source cards stack full width. The ask row is a
flex row: the input takes the remaining width, the button keeps its
content width of 24px horizontal padding.

## Elevation & Depth

| Level | Treatment | Use |
|---|---|---|
| 0 - Flat | No border, no shadow | Top bar, page background, examples of quiet text |
| 1 - Hairline | 1px solid `--hairline` | Answer card, source cards, input at rest |
| 2 - Hairline strong | 1px solid `--hairline-strong` | Error card only |
| 3 - Inverted | `--surface-dark` fill | Reserved; unused today |

Nothing lifts, nothing floats, nothing layers. The only depth cues are
hairline borders and the single reserved dark surface.

## Shapes

| Token | Value | Use |
|---|---|---|
| Pill | `9999px` | Buttons, input, status pill, tags, status dot |
| Card | `12px` | Answer card, source cards, error card |

Pills for everything interactive, `12px` for every card. There are no
medium-radius surfaces.

## Components

### Top Bar

**`topbar`** - height 56px, hairline bottom border, flex space-between.

**`brand`** - the text "Cloud Sec" in display type, 18px, weight 600.

**`status-pill`** - soft surface pill, 12px body text in charcoal, 6px by
14px padding, inline-flex with an 8px gap before the dot. A native `title`
carries the diagnostics: model, chunk count, endpoint.

**`status-dot`** - 8px circle, `--mute` by default; `--status-ok` when the
backend reports both the index and Ollama ready. Four pill states:

| State | Dot | Text |
|---|---|---|
| Loading | mute | "กำลังเชื่อมต่อ..." |
| Ready | status-ok | "พร้อมใช้งาน" |
| Ollama unreachable | mute | "ออฟไลน์" |
| Index missing | mute | "ยังไม่พร้อม" |

### Inputs & Forms

**`ask-row`** - flex row, 12px gap, contains the input and the primary
button.

**`ask-input`** - 44px tall, canvas background, 1px hairline border, pill
radius, 20px horizontal padding, 16px body text. Focus: border switches to
ink and the focus ring appears.

**`button-primary`** - 40px tall, ink background, canvas text, 14px weight
500, 24px horizontal padding, pill radius. Active: `--ink-deep`. Disabled:
soft surface background with mute text, default cursor.

**`busy-line`** - 14px body text in `--body`, 32px below the ask row, with
an animated trailing ellipsis (1.2s steps). Copy: "กำลังค้นเอกสารและ
ประมวลผล ครั้งแรกอาจใช้เวลา 20-60 วินาที".

### Cards & Containers

**`answer-card`** - 1px hairline border, 12px radius, 24px padding, 32px
below the ask row. The answer is rendered with `white-space: pre-wrap` and
`word-break: break-word` at 16px body type with 1.6 line height. Streaming
appends into this element; it never re-renders the card.

**`answer-meta`** - the timing line under the answer, 12px mono in
`--mute`, 12px below the text. Format: prefill tokens and seconds, answer
tokens and seconds, total seconds.

**`source-card`** - one retrieved requirement: 1px hairline border, 12px
radius, 16px padding, 12px between cards.

**`source-head`** - flex wrap row of tags with 8px gaps.

**`tag`** - soft surface pill, 14px mono, 4px by 10px padding. The first
tag carries the clause ("ข้อ 5.2.3.4"); the rest are `tag-muted`: party
label, page ("หน้า 11"), score ("score 0.741").

**`source-preview`** - the first 120 characters, 14px charcoal, 8px below
the head.

**Source disclosure** - a native `details` element: summary "ดูข้อความ
ต้นฉบับ" at 14px in `--body`; the body is `pre` at 14px body text,
pre-wrapped, 8px below the summary.

**`error-card`** - hairline strong border, 12px radius, 16px padding,
charcoal text, 32px below the ask row. Copy: "ไม่สำเร็จ: {message}".

## Do's and Don'ts

### Do

- Keep one black pill per action. The ask button is the only primary
  button on the screen.
- Default any new interactive element to pill geometry; any new container
  to a 12px card with a 1px hairline.
- Reserve mono for identifiers and measurements: clause numbers, scores,
  token counts, timings.
- Keep the status green exclusive to the ready dot.
- Stream answers as plain pre-wrapped text at 16px with 1.6 line height.
- Keep diagnostics (model, chunks, endpoint) in the status pill's title
  attribute, not in the layout.

### Don't

- Don't introduce shadows, gradients, or atmospheric backgrounds.
- Don't add colors beyond this file's tokens. The system is black, white,
  four grays, one green, one focus blue.
- Don't switch pills to cards or the reverse; delete the element before
  re-shaping it.
- Don't render markdown in the answer card. The model's plain Thai with
  line breaks is the design.
- Don't put technical values in visible chrome; a hover title is the
  ceiling.
- Don't add illustrations or mascots. There are none in the system.

## Responsive Behavior

The surface is a desktop window: Tauri enforces a 900x600 minimum
(`tauri.conf.json`), and the layout has no breakpoints. The content column
is fluid up to 860px; below that the 24px page padding defines the edge.
There is no mobile surface and none is planned.

## Iteration Guide

1. Work one component at a time and reference its token by name
   (`--surface-soft`, `--radius-card`), never by hex value in prose.
2. Keep the palette closed. A new color is a change to this file, the CSS
   variables, and AGENTS.md in the same commit.
3. New states get their own rows or blocks (`-active`, `-disabled`,
   `-focused`); do not bury them inside sentences.
4. Prefer composing existing pieces: pill + hairline card + mono tag +
   disclosure covers nearly everything this screen needs.
5. If a change makes the chrome more interesting than the answer, it is
   wrong; delete it.

## Known Gaps

- **Hover states** are deliberately minimal: a native cursor on buttons
  and summaries, plus the status pill's title. No visual hover treatment is
  defined.
- **No dark mode.** The canvas is white by design.
- **Disabled input** is not defined; the input has no disabled state today.
- **Long-answer overflow** is untested: the answer card grows with the
  content and the page scrolls; a collapsed view has not been designed.
- **Print styles** are absent. The app is not a report generator.
