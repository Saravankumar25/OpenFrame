# ADR-0008: Deterministic PDF output via `pdf-writer` and the PDF base-14 fonts

- **Status:** Accepted. **Implementation in progress** (export module; `pdf-writer` 0.13 and `lopdf` 0.36 are
  workspace dependencies).
- **Date:** 2026-09-30
- **Related:** Import/Export spec; FSD exports and printing; invariant 8 (issued exports are immutable snapshots);
  PRD C-24 (paper sizes), C-25 (regional scripts)

## Context

OpenFrame must produce professional PDFs offline: screenplay (industry format, Courier 12 pt), sides, call sheets,
schedules, breakdown and reports. Exports are snapshots handed to cast and crew, so the same input must produce the
same file, which allows golden tests and meaningful "has this changed?" checks. Headless-browser printing (WebView2
`PrintToPdf`) depends on the webview version and is not byte-stable. Big PDF engines are heavy or GPL/AGPL-licensed.

## Decision

1. Render PDFs in Rust with **`pdf-writer`**: a low-level, allocation-light writer (MIT/Apache-2.0) that performs
   our own layout (pagination, MORE/CONT'D, scene numbers, headers and footers).
2. Use only the **standard base-14 fonts**: Courier (screenplay and sides), Helvetica (production documents), and
   their bold/oblique variants. They are referenced by name with `WinAnsiEncoding` and no embedded font files.
3. **Determinism rules:** no creation/modification timestamps or random ids in the output. `/ID` and document
   metadata are derived from the export's content hash and its snapshot time as recorded by OpenFrame. Object order is
   fixed. Floating-point coordinates are rounded to a fixed precision. Same snapshot + same settings ⇒ byte-identical
   PDF, enforced by golden-file tests.
4. Paper size is an explicit export setting (US Letter and A4). OpenFrame never guesses silently.
5. **`lopdf`** is used only for reading and validating (tests, import inspection), never for authoring.
6. **Character coverage guard:** base-14 fonts cover WinAnsi (≈ Latin-1) only. Before rendering, the exporter checks
   every string. If any character cannot be encoded, the export fails with `export.unsupported_characters`, listing
   where it occurs, instead of dropping or substituting glyphs. Nothing is written in that case.

## Consequences

- Small, fast, reproducible PDFs with no font licensing questions (viewers supply base-14 fonts).
- **Known limitation:** scripts beyond Latin-1 (Devanagari, Tamil, CJK, Arabic, etc.) cannot be exported to PDF in
  v1. This conflicts with the PRD's regional-language audience (C-25) and needs a follow-up ADR introducing embedded,
  subset TrueType/OpenType fonts (with shaping for complex scripts). The guard makes the limitation explicit rather
  than a silent data loss.
- Visual output does not match the on-screen webview typography pixel-for-pixel. Layout rules are shared as data
  (margins and element indents from the UX spec) rather than as CSS.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| WebView2 `PrintToPdf` / headless Chromium | Not deterministic. Depends on the runtime version. Harder to paginate screenplay rules exactly. |
| `printpdf` with embedded fonts now | Solves Unicode but adds font licensing, subsetting and shaping work. Deferred to the follow-up ADR. |
| Typst / LaTeX engine | Large dependency surface. Typst's embedded use pulls in many crates. |
| wkhtmltopdf / external tools | External binary, unmaintained or unsafe, and needs installation |
