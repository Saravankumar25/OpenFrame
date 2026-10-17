# Supply chain and secrets (release gate S11 and S13)

> 2026-10-17, branch `windows/w9-endgame`. Tools: cargo-audit and cargo-deny as installed in the development
> environment, npm 10 `npm audit`, gitleaks 8.30.1 (GitHub release, checksum verified), the repository's own
> `check:secrets` script.

## Before

| Check | Findings |
|---|---|
| `cargo audit` | 3 vulnerabilities, all in `zip` 4.6.1: RUSTSEC-2026-0367, 0368, 0369 (`ZipArchive::extract` with symlinks and permission bits) |
| `cargo deny --locked check` | advisories failed (same three); bans, licenses, sources ok |
| `npm audit` (all dependencies) | 14: 2 critical (vitest, tinypool), 11 high (webdriverio and its browser downloader chain, source-map-js), 1 moderate (@vitest/mocker) |
| `npm audit --omit=dev` | 0 |

OpenFrame never calls `extract` (its own `openframe-security::archive` reads entries one by one with limits), and every
npm finding was in a test tool that never ships. None was reachable, but the gate wants zero findings.

## Fix (commit 2b7cffc)

- `zip` 4 to 9 and `rust_xlsxwriter` 0.90 to 0.100 (the first release that uses zip 9). zip 9 returns entry names as a
  `Result`; an entry name that is not valid text now rejects the archive.
- `vitest` 3.2 to 4.1 (drops tinypool, fixes the mocker advisory); `webdriverio` 9 to 10 for the end-to-end harness.
- Rust workspace: 2106 tests pass, 0 fail. Web tests: 596 pass. The end-to-end harness on webdriverio 10 is proved by the
  release suite run (gate P3).

## After

| Check | Result |
|---|---|
| `cargo audit` | 0 vulnerabilities; 7 warnings listed below |
| `cargo deny --locked check` | advisories ok, bans ok, licenses ok, sources ok |
| `npm audit` | 0 (info 0, low 0, moderate 0, high 0, critical 0) |

The seven `cargo audit` warnings:

| Crate | Advisory | Why it is acceptable |
|---|---|---|
| rustybuzz 0.20.1 | RUSTSEC-2026-0206, unmaintained | Owner decision 2026-10-08, dated exception in `deny.toml` until 2027-01-06: parses only the bundled, hash-pinned fonts |
| ttf-parser 0.25.1 | RUSTSEC-2026-0192, unmaintained | Same exception |
| glib 0.18.5 | RUSTSEC-2024-0429, unsound | Linux-only dependency of Tauri: `cargo tree --target x86_64-pc-windows-msvc` and `--target aarch64-apple-darwin` print nothing |
| zbus_names, zvariant, zvariant_derive | RUSTSEC-2026-0370 to 0372, unmaintained | Same: never compiled for Windows or Apple silicon |
| proc-macro-error 1.0.4 | RUSTSEC-2024-0370, unmaintained | Same: not in the Windows or Apple silicon tree |

Lockfiles `Cargo.lock` and `package-lock.json` are committed. `package-lock.json` had the Windows hidden attribute,
which made npm fail to rewrite it (EPERM); the attribute was cleared.

## Secrets and personal paths

| Check | Result |
|---|---|
| gitleaks on the full history (54 commits, all refs, about 100 MB) | no leaks found |
| `npm run check:secrets` (2216 files, 54 commits) | OK |
| Personal paths in tracked files | One: `.codex/hooks.json` held the owner's Windows user folder. `.codex/` is left over from the GSD framework the owner removed on 2026-10-16; it is deleted in this branch. Other `C:\Users\...` strings are made-up test names (`A`, `me`, `Asha`, `Nisha`, `x`) |
| Personal paths in history | Two commits hold the owner's name: the first commit (the same `.codex/hooks.json`) and a8f91e2 (a W5 evaluation result that echoes the local display name). Removing them needs a history rewrite, which is the owner's decision (release gate S13 exception) |

The installer scan (`npm run scan:installer`) runs on the final build (gate G1).

## Update 2026-10-18: end-to-end harness back on webdriverio 9

The end-to-end harness went back from webdriverio 10 to 9 (commit 1fbc0db). Version 10 has strict selectors, no
`executeAsync`, and changed click and key handling. That failed 22 release specs that pass on 9 (see
[e2e-release-runs.md](e2e-release-runs.md)).

| Check | Result |
|---|---|
| `npm audit` (all dependencies) | Reports the webdriverio 9 dependency chain again. Dev only. High findings in `@puppeteer/browsers`, `extract-zip`, `basic-ftp`, `get-uri` and the proxy agents: the browser downloader |
| `npm audit --omit=dev` | 0 |
| `cargo audit` | 0 vulnerabilities |
| `cargo deny` | ok |

**Exception, dated 2026-10-18, review by 2027-01-18 (waiting for the owner).** The harness never calls the browser
downloader: drivers come from `tests/e2e/scripts/setup-drivers.mjs`. Nothing of the harness ships. It runs only on the
developer laptop, against the app. Gate S11 stays an exception until the owner approves.
