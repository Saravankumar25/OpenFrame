# Licence, attribution and third-party IP audit (2026-10-31)

Scope: the Windows app (this tree), the Mac app (`mac/` on GitHub `main`, read in this worktree), the local AI engine
and models, and the public website at `C:\OpenFrameWebsite` (read only; commit `0918496`). Branch:
`claude/audit-licences-16cdc6`. This is an engineering audit, not legal advice. Every item marked "legal review"
needs a lawyer's answer before Chapter 3 (store release). Licence compliance is owner-handled (owner decision
2026-10-16); this report gathers the facts and flags, it decides nothing.

## 1. Verdict in short

- **No copyleft code ships.** 407 Rust crates and 72 npm packages: 0 GPL, LGPL or AGPL, 0 unknown. Five crates are
  MPL-2.0 (file-level copyleft, used unmodified): fine for a closed-source product when their licence text and a
  pointer to their source ship. Both now ship in the app.
- **Model downloads follow the approved design** (ADR-0032): Hugging Face only, 40-hex commit revisions, size and
  SHA-256 compiled into the app, redirects only to Hugging Face hosts. Evidence in section 5.
- **Gaps found and fixed in this branch:** the committed `THIRD_PARTY_LICENSES.md` was stale (missing the third
  model, the Vulkan engine and nine crate changes, and still said the models were bundled); the app showed no
  third-party notices at all; the download screen showed no model terms; the engine notice missed the MIT code
  vendored inside llama.cpp. Section 8 lists the changes.
- **Still open before a store release:** the end-user agreement must carry the Gemma use restrictions; a verbatim
  copy of the Gemma Terms of Use should ship; eleven packages carry no licence file; the Mac app has no notices
  screen and its inventory is generated for the wrong platform. Sections 9 and 10.

## 2. Method

- Rust: `scripts/license-inventory.mjs` (`cargo metadata --locked --filter-platform x86_64-pc-windows-msvc`, normal
  dependency edges from the shipped workspace members). `cargo-about` and `cargo-license` are not installed;
  `cargo-deny` is (its `[licenses]` gate runs in CI).
- npm: `package-lock.json`, production entries only, licence field then the installed `package.json`.
- Licence texts: the licence, copying and notice files in each package's own folder (cargo registry source,
  `node_modules`), collected by the same script into the in-app notices file.
- Engine: the pinned llama.cpp b11259 files in `.ai-assets`, searched for vendored library names; the upstream
  `vendor/` and `licenses/` folders at tag b11259 read on GitHub.
- Models: `packaging/ai/ai-assets.lock.json`, `crates/openframe-ai/src/{download,fetch,install}.rs`, the model notice
  files, and the Gemma Terms of Use and Prohibited Use Policy at ai.google.dev (paraphrased below, from the published
  text as the auditor knows it; counsel must read the current text).
- Website: `package.json`, `src/assets`, `src/static`, the built `dist/` (every external host referenced, embedded
  images in the screen recreations sampled), `.gitignore`.
- Licence server (`services/licence-server`, Chapter 3, runs on a server, not shipped to users): `cargo metadata`,
  138 crates, all permissive.

## 3. Inventory: the desktop app

### 3.1 Code compiled into the app

| Kind | Count or item | Licences | Notes |
|---|---|---|---|
| Rust crates (Windows target) | 407 | MIT or Apache-2.0 (most), MIT 140, Unicode-3.0 18, BSD-3-Clause 7, MPL-2.0 5, Zlib, ISC, 0BSD, BSL-1.0, CDLA-Permissive-2.0 (webpki-roots) | Full list: `THIRD_PARTY_LICENSES.md` |
| npm production packages (bundled into the web view) | 72 | MIT, ISC (lucide-react icons), Apache-2.0, BSD | Full list: `THIRD_PARTY_LICENSES.md` |
| SQLite (through `rusqlite` feature `bundled`) | 1 | Public domain | No notice required |
| sqlite-vec | 0.1.9 | MIT OR Apache-2.0 | No licence file in the crate folder (section 9) |
| WebView2 bindings (`webview2-com*`) | 3 | MIT | No licence file in the crate folder; the WebView2 runtime itself is part of Windows |
| Swift packages (Mac) | none from third parties | n/a | `apps/macos/Package.swift` and `StoreKitBridge` use only first-party targets and Apple frameworks |
| Licence server (Chapter 3, server side) | 138 crates | all permissive | Not distributed to users |

MPL-2.0 crates (unmodified, reached through Tauri and `directories`): `cssparser`, `cssparser-macros`, `dtoa-short`,
`option-ext`, `selectors`. Obligation: keep the MPL text and say where the source is. The in-app notices give the
MPL text and the source address of each.

### 3.2 Offline AI engine (in the package)

| Item | Version | Licence | Ships | Notes |
|---|---|---|---|---|
| llama.cpp server and libraries | b11259 | MIT, Copyright (c) 2023-2026 The ggml authors | Windows CPU and Vulkan builds; Mac Metal build | Pinned by size and SHA-256 in the lock |
| cpp-httplib (vendored in llama.cpp) | from b11259 | MIT, Copyright (c) 2017 yhirose | compiled into `llama-server-impl.dll` and `llama-common.dll` (name found in the binaries) | Notice was missing; added |
| nlohmann/json (vendored) | from b11259 | MIT, Copyright (c) 2013-2025 Niels Lohmann | compiled into `llama-common.dll` | Notice was missing; added |
| stb_image, miniaudio, subprocess.h (vendored) | from b11259 | public domain (each offers a public-domain option) | possibly in `mtmd.dll` | No notice required; named anyway |
| LLVM OpenMP runtime (`libomp.dll`) | Windows only | Apache-2.0 WITH LLVM-exception | its licence text ships next to it | Already correct |

### 3.3 Offline AI models (downloaded after install, never packaged)

| Model | Repository and pinned revision | Licence | Origin of the file |
|---|---|---|---|
| Gemma 3 1B Instruct, Q8_0 GGUF | `ggml-org/gemma-3-1b-it-GGUF` @ `f9c28bcd85737ffc5aef028638d3341d49869c27` | Gemma Terms of Use and Gemma Prohibited Use Policy | GGUF conversion by ggml-org of Google's model |
| BGE small English v1.5, Q8_0 GGUF | `ggml-org/bge-small-en-v1.5-Q8_0-GGUF` @ `f2068edd9b54f2a369549ccc71f70ed273a2a801` | MIT (BAAI) | GGUF conversion by ggml-org |
| Multilingual E5 small, Q8_0 GGUF | `TwinSunsLLC/multilingual-e5-small-gguf` @ `b6cac9615d4ecce28d7f22539b7322d695fc2886` | MIT (intfloat) | GGUF conversion by a third party (owner decision 2026-10-16, ADR-0032) |

These three are the only models the code references (`download::pins` builds the plan from the lock's `models`; its
unit test asserts three files).

### 3.4 Fonts, icons, images and content in the app

| Asset | Origin | Licence | Status |
|---|---|---|---|
| Courier Prime (3 styles) in PDF export | Quote-Unquote Apps | OFL-1.1 | Licence text next to the files; subset embedding in PDFs is allowed by the OFL |
| Noto Sans Devanagari, Kannada, Malayalam, Tamil, Telugu in PDF export | Google Noto | OFL-1.1 | Same |
| User interface fonts | Windows system fonts (Segoe UI) and macOS system fonts | not shipped | No obligation |
| App icons (`src-tauri/icons`, `source.svg`) and `openframe-logo.png` in PDFs | OpenFrame's own | own | Logo ownership is an open owner task (owner-decisions: "Trademark and name clearance, social handles, logo ownership, font licences") |
| UI icons | lucide-react | ISC | In the npm list and the notices |
| Sample scripts (`templates/samples`) | written by OpenFrame (owner decision 2026-10-07) | own, copyright line inside each file | Fictional, no third-party text |
| Test fixtures (`tests/fixtures`, PDF goldens) | own | not shipped | n/a |

### 3.5 What is not OpenFrame's to license

Files a person imports, exports, links or packages (scripts, images, PDFs, Exchange and Review packages) are the
person's own. OpenFrame does not distribute them: they stay on the person's computer or travel in files the person
sends. Nothing in the app uploads them: the web view may only reach the app itself (`connect-src ipc:
http://ipc.localhost` in the Tauri CSP), and the core's only outside network use is the model download from Hugging
Face (plus the store purchase checks in Chapter 3). They are outside this inventory by design.

## 4. Inventory: the website (read only)

| Asset | Origin | Licence | Status |
|---|---|---|---|
| Inter, Newsreader, Sora (variable WOFF2, latin subset) | `@fontsource-variable/*` (self-hosted) | OFL-1.1 | `OFL-Inter.txt`, `OFL-Newsreader.txt`, `OFL-Sora.txt` are in `src/assets/fonts/` and in the built `dist/assets/fonts/`. Compliant |
| Candidate fonts in `.cache/fonts` (Bricolage Grotesque, Fraunces, Geist, Instrument Sans and Serif, Manrope, Outfit, Plus Jakarta Sans) | font trial | OFL-1.1 | Git-ignored, not published |
| Screen recreations and captures (`mockups/`, `screens/`, `og.png`) | OpenFrame's own app | own | Embedded images sampled: solid colour swatches and empty frames from the test data, no photographs |
| Favicons, app icons | OpenFrame's own | own | |
| Company logo (`company/vighnajith-advisories-logo.webp`) | the company's own | own | Confirm ownership with the owner |
| Third-party scripts, CDNs, analytics, embedded media | none | n/a | External hosts in the built pages: only `openframe-studio.pages.dev`, `schema.org`, `example.com` placeholders, and text links to `huggingface.co` and `ai.google.dev` |
| Build tools (`esbuild` MIT, `sharp` Apache-2.0 with LGPL libvips binaries, `playwright` Apache-2.0, `@axe-core/playwright` MPL-2.0) | dev dependencies | various | Run on the build machine only; nothing of them is served to visitors |
| Forms, accounts, uploads, comments | none | n/a | Static site on Cloudflare Pages with no functions |

Trademarks: the footer already says Microsoft, Microsoft Store, Apple, Mac, macOS and Mac App Store belong to their
owners. The compare and switch pages name Final Draft, Celtx, Highland and Movie Magic. Text for that is in section 11.

## 5. Model downloads match the approved design (verified in code)

| Requirement (ADR-0026, ADR-0032) | Where in code | Result |
|---|---|---|
| Straight from the public Hugging Face repository | `download::huggingface_url_ok` (`crates/openframe-ai/src/download.rs`) accepts only `https://huggingface.co/<org>/<repo>/resolve/<rev>/<file>` | Pass |
| Pinned commit revision, never a branch | same function: `rev.len() == 40`, lowercase hex only; test `only_pinned_hugging_face_addresses_are_accepted` rejects `main`, uppercase, `blob`, extra path parts and look-alike hosts | Pass |
| Size and SHA-256 compiled into the app | `AssetLock::embedded()` reads the lock through `include_str!`; `download::pins` copies `bytes` and `sha256` per file | Pass |
| A file is used only when it matches | `install::file_matches` checks the size, then the SHA-256 of the whole file, after download (`install.rs` line 123) and on import (line 464) | Pass |
| No other host | `fetch.rs`: redirects only to `huggingface.co` or a host under `.hf.co`, at most five, https only; a loopback test host only in test builds | Pass |
| More bytes than pinned are refused | `fetch.rs`: "the host sent more than the pinned size" | Pass |
| Nothing touches the network before the click | `DownloadAi.tsx` header comment; the download starts on `ai.download.start` | Pass by reading (not re-tested here) |

Observation, not a defect: the Gemma file comes from `ggml-org`, not from `google/gemma-3-1b-it` (which Hugging Face
gates behind accepting the Gemma Terms). Because the app's source is not gated, the person never sees the Gemma Terms
on Hugging Face, so the app itself must give them (section 6).

## 6. When the model notices must be shown, and what the app shows

What the Gemma Terms of Use ask of anyone who distributes Gemma or a Model Derivative (section 3.1, paraphrased):
(a) carry the section 3.2 use restrictions as an enforceable provision in the agreement that governs use, and tell
recipients Gemma is subject to them; (b) give every recipient a copy of the Gemma Terms of Use; (c) mark modified
files; (d) ship a "Notice" text file with the exact sentence "Gemma is provided under and subject to the Gemma
Terms of Use found at ai.google.dev/gemma/terms". Section 3.2 binds use to the Prohibited Use Policy. The terms also
say Google may update them and may restrict use it reasonably believes breaks them.

Decision for engineering (subject to legal review, section 10):

| Moment | Required? | What the app shows now |
|---|---|---|
| Before the download click | Yes. This is the moment the person receives Gemma | **Added:** the download card shows the exact Gemma notice, the Prohibited Use Policy address, that the other models use the MIT License, and where every licence is listed |
| Install or purchase (installer or store licence page) | Yes, through the end-user agreement | Not yet: the agreement is not written (`LICENSE-PENDING.md`, now listing the Gemma clauses as inputs) |
| Any time afterwards | Yes, the notice file and the terms must stay available | **Added:** Settings > About > Third-party notices shows the Gemma notice and policy at once, and on request the engine, model, font and library licence texts. The `ai/notices` folder with `gemma-NOTICE.txt` ships in the package (already true). Settings > Offline AI > Technical details shows the licence of each component (already true) |
| Each use of the AI | No. Nothing in the Gemma Terms or the MIT License asks for a notice per use | Nothing added |

MIT models (BGE, E5): the copyright and permission notice must accompany copies. Their notice files ship in
`ai/notices` and are shown in the notices screen.

## 7. Copyright complaints and DMCA

- **The app hosts no user content.** It is local-first; nothing a person writes is uploaded or shared by OpenFrame
  (section 3.5). There is no service to which a takedown would apply.
- **The website hosts no user content.** It is static, with no forms, comments, accounts or uploads.
- **The licence server** (Chapter 3) holds purchase records only, no user works.
- Conclusion: a DMCA designated agent (17 U.S.C. 512) and a notice-and-takedown procedure are not needed for the
  current design, because OpenFrame does not store material at a user's direction. India's IT Rules duties for
  intermediaries likewise do not reach a product that hosts nothing. Legal review should confirm (section 10,
  question 6). Good practice anyway: a copyright contact on the website for complaints about OpenFrame's own pages
  (text in section 11).
- If a future feature hosts or relays user content (cloud sync, shared links, a gallery), this answer changes.

## 8. Changes made in this branch

| File | Change |
|---|---|
| `scripts/license-inventory.mjs` | AI section rewritten: engine in the package, models downloaded after install (it still said "bundled in every package", the ADR-0025 wording); the Vulkan and Metal engine builds labelled; the Gemma row states the distribution conditions. New: writes `apps/desktop/src/workspaces/settings/third-party-notices.json` (every shipped package with its licence, source and full licence, copying and notice texts, the AI notices and the font licences; each distinct text stored once) and prints packages that carry no licence file. A run with `--out` never touches tracked files |
| `THIRD_PARTY_LICENSES.md` | Regenerated: 407 crates (was 405), the Multilingual E5 small model and the Vulkan engine added, nine crate versions updated. The release gate "THIRD_PARTY_LICENSES.md up to date" would have failed on the old file |
| `apps/desktop/src/workspaces/settings/ThirdPartyNotices.tsx` (new) and its test | Settings > About > Third-party notices: Gemma notice and policy always visible; "Show licences and notices" loads the generated file (a separate chunk, about 97 kB compressed) and lists the engine and models, the fonts and every library, each text opened on demand |
| `apps/desktop/src/workspaces/settings/About.tsx` | Renders the notices section under About (in the start screen's settings and in project settings) |
| `apps/desktop/src/ai/DownloadAi.tsx` | Before the click: the Gemma notice, the Prohibited Use Policy, the MIT licence of the other models and where every licence is listed |
| `apps/desktop/src/ai/model.ts`, `OfflineAi.tsx` | One home for the Gemma notice text (`GEMMA_NOTICE`) and the policy line (`GEMMA_POLICY`) |
| `packaging/ai/notices/llama.cpp-LICENSE.txt` (Windows and Mac trees) | Copyright line updated to the b11259 text (2023-2026); vendored cpp-httplib and nlohmann/json notices added; public-domain vendored code named; libomp note fixed for both Windows engine builds |
| `scripts/check-writing-standard.mjs` | Skips the generated notices file (other authors' licence texts, word for word), as it already skips `THIRD_PARTY_LICENSES.md` |
| `LICENSE-PENDING.md` | The AI bullet corrected (three models, no OpenFrame download endpoint) and the Gemma clauses the agreement must carry |

## 9. Redistribution risks, highest first

1. **Agreement missing the Gemma use restrictions** (Gemma Terms 3.1(a)). Until the end-user agreement exists and
   carries them, a store release would pass Gemma on without the required enforceable provision. Already a release
   blocker through `LICENSE-PENDING.md`; the Gemma clause is now named there.
2. **No verbatim copy of the Gemma Terms of Use ships** (3.1(b)). The app gives the address and the notice sentence,
   not the full text. Fix: the owner or counsel adds the reviewed terms text as `packaging/ai/notices/gemma-TERMS.txt`
   (it then flows into `ai/notices` and the in-app notices on the next `npm run licenses`). Not done here: the version
   to ship must be the one counsel reviewed.
3. **Mac app shows no third-party notices and no download notice.** The SwiftUI shell has the Gemma line only in
   Settings > AI technical details (`AISettingsView.swift`). The Mac track needs the same About section and download
   paragraph, reading the same notices file.
4. **Mac inventory is generated for the Windows target.** `mac/THIRD_PARTY_LICENSES.md` comes from
   `--filter-platform x86_64-pc-windows-msvc` over the Tauri workspace, so it lists Tauri and npm packages the Mac app
   does not ship and misses what it does ship (the UniFFI crates and the macOS-only dependencies of the
   `OpenFrameFFI` xcframework). It needs its own roots and `aarch64-apple-darwin`.
5. **Eleven packages carry no licence file** in their published folder: `alloc-stdlib` 0.2.4 and 0.3.0
   (BSD-3-Clause), `defmt-parser` 1.0.0, `selectors` 0.38.0 (MPL-2.0; the MPL text ships with `cssparser`),
   `sqlite-vec` 0.1.9, `ts-rs` and `ts-rs-macros` 11.1.0, `webview2-com`, `webview2-com-macros`, `webview2-com-sys`
   (MIT), npm `react-remove-scroll-bar` 2.3.8 (MIT). The notices screen shows their declared licence and source. For
   MIT and BSD the upstream copyright line should be added: a small override folder read by the generator, filled
   from each upstream repository.
6. **E5 GGUF from a third-party repository.** The conversion by TwinSunsLLC is a derivative of an MIT model; the
   source choice is settled (ADR-0032). Open only: whether that repository states its own terms for the conversion,
   and whether it keeps the original MIT notice.
7. **ggml-org GGUF files are Model Derivatives of Gemma** (quantised). OpenFrame does not modify them, so 3.1(c)
   does not fall on OpenFrame. Whether ggml-org's repository carries the Gemma notice is their duty; worth one look
   by counsel.
8. **Gemma Terms can change.** The pinned file stays the same, but the terms that govern it are the current ones at
   ai.google.dev. Re-read them before each release.
9. **Website compare pages name competitor products** (nominative use). Low risk with a plain ownership line and no
   logos (none are used).
10. **Bundle size.** The notices file is about 0.9 MB raw; Vite may warn that the lazy chunk passes its 900 kB warning
    limit. A warning only; the main bundle is unchanged.

## 10. Questions for legal review

1. Does the app's direct download from Hugging Face, at OpenFrame's chosen revision and on OpenFrame's button, make
   OpenFrame a distributor of Gemma under the Gemma Terms (and so bound by 3.1(a) to (d))? This audit assumes yes
   and acts accordingly.
2. Is a notice paragraph on the download card enough to pass the use restrictions to the person, or must they accept
   the Gemma Terms (a click-through) before the download? The current UI informs; it does not ask for acceptance.
3. Exact wording of the end-user agreement clause that makes the Gemma Prohibited Use Policy enforceable against the
   user, and how the agreement treats the outputs the user generates (the Gemma Terms say Google claims no rights in
   outputs).
4. Does a link plus the notice sentence satisfy "provide a copy of this Agreement", or must the full text ship
   (risk 2)?
5. Store review: do Microsoft Store and Mac App Store expect third-party or AI model disclosures in the listing, or is
   the in-app screen enough?
6. Confirm that no DMCA agent or Indian IT Rules grievance officer is needed for the current product and website
   (section 7).
7. Trademarks: "Gemma" (a Google trademark) appears on the download card and in the notices because the required
   notice names it; confirm no further attribution is needed. Same question for "Final Draft" in the import and export
   format names.
8. MPL-2.0: confirm that the source address of each MPL crate in the notices meets section 3.2 of the MPL for an
   unmodified executable.

## 11. Text for the website (for the website session to place)

Put on the Legal page, a section titled "Third-party notices":

> OpenFrame Studio is built with open-source software, fonts and AI models made by others. The app lists every one,
> with its licence, in Settings, About, Third-party notices.
>
> OpenFrame AI uses Gemma 3 1B, downloaded once after install from Hugging Face. Gemma is provided under and subject
> to the Gemma Terms of Use found at ai.google.dev/gemma/terms. Its use must follow the Gemma Prohibited Use Policy
> found at ai.google.dev/gemma/prohibited_use_policy. OpenFrame AI also uses BGE small English v1.5 by the Beijing
> Academy of Artificial Intelligence and Multilingual E5 small by Liang Wang and co-authors, both under the MIT
> License, and runs them with llama.cpp by the ggml authors, under the MIT License.
>
> This website uses the Inter, Newsreader and Sora typefaces under the SIL Open Font License 1.1. The licence texts
> are published with the font files.

Add to the trademark line in the footer and on the Legal page, after the Microsoft and Apple sentence:

> Gemma is a trademark of Google LLC. Final Draft, Celtx, Highland and Movie Magic are trademarks of their respective
> owners and are named only to describe compatibility or comparison. OpenFrame Studio is not affiliated with or
> endorsed by them.

Add a copyright contact (good practice, not a legal requirement for this static site):

> Copyright questions about this website: write to the support address on the Contact page with the page address and
> what you believe infringes your rights.

## 12. What was not run or not verified

- The Mac app was read, not built; no Swift change was made.
- No installer, MSIX or production build was made (owner rule), so the size of the notices chunk in a real build was
  not measured. Vitest (71 tests in the touched areas), the TypeScript typecheck and the writing check were run and
  pass.
- The notices screen was not viewed in the running app or a browser; its behaviour is covered by
  `ThirdPartyNotices.test.tsx`.
- The licence statements of the two ggml-org repositories and the TwinSunsLLC repository were not re-read on Hugging
  Face in this session.
- Whether stb_image and miniaudio are compiled into `mtmd.dll` was not proven (no identifying strings found); both
  are public domain either way.
