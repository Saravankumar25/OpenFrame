# ADR-0016: PDF output with embedded subset fonts and complex-script shaping

- **Status:** Accepted 2026-10-07 (owner answer in chat). Amends ADR-0008 (decision 2 and the "Known limitation").
- **Date:** 2026-10-05
- **Related:** ADR-0008; PRD C-25; Screenplay and Production Depth Specification §19 and §45;
  `crates/openframe-import-export/src/{layout.rs,pdfgen.rs,winansi.rs}`

## Context

ADR-0008 renders PDFs with `pdf-writer` and the base-14 fonts in WinAnsi encoding, and the exporter fails with
`export.unsupported_characters` for anything outside Latin-1. ADR-0008 itself says this conflicts with the PRD's
regional-language audience and needs a follow-up ADR. The revised specification makes Telugu, Hindi, Tamil,
Malayalam, Kannada, English and mixed-script text a **must-ship** requirement that must survive typing, save, reload,
**PDF**, FDX, Fountain and DOCX. Those scripts need glyph shaping (conjuncts, reordering), not just a larger
character table.

## Decision

1. Embed **subset TrueType/OpenType fonts** in PDFs. Screenplay Latin text keeps Courier-metric output
   (an embedded Courier-compatible face with a clear licence, or the base-14 Courier when every character is
   WinAnsi). Non-Latin runs use bundled fonts with an open licence (SIL OFL, for example Noto families), chosen per script.
2. Shape text with a Rust shaping library before layout (`rustybuzz` or equivalent), then position glyphs.
   Dependency choice follows the dependency policy (`deny.toml`, licence inventory).
3. **Determinism is preserved**: fixed font set and version, fixed subsetting, no timestamps or random ids,
   byte-identical output for the same snapshot and settings, enforced by golden tests.
4. Layout measures text with the same shaped metrics the renderer uses, so pagination stays deterministic.
5. The `export.unsupported_characters` guard stays for characters no bundled font covers, and still writes nothing.
6. Fonts ship inside the app, so the offline guarantee and the clean-install guarantee hold. Font licences are
   added to `THIRD_PARTY_LICENSES.md`.
7. DOCX export writes text as UTF-8 with explicit font hints for the script. FDX and Fountain are UTF-8 already.

## Consequences

- Installer size grows by the bundled fonts (measure and record).
- A multilingual golden corpus is required (see the `XFER-` and `SCRIPT-UNI-` features in the feature status file).
- Editor display uses the WebView fonts, so on-screen and PDF glyph shapes may differ; page counts must not.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| WebView2 print-to-PDF | Not deterministic (ADR-0008). |
| Keep Latin-1 only | Fails a must-ship requirement. |
| Rasterise unsupported text | Loses text selection, search and accessibility. |
