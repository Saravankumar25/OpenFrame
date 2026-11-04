# Microsoft Store submission checklist

Status: written 2026-11-03. Nothing here is ticked. Source: section C of
[package file 05](package-2026-11-03/05_UPDATE_SAFETY_AND_STANDARDS.md), [package file 06](package-2026-11-03/06_PLACEHOLDERS_AND_COMPANY_DETAILS.md),
[27-microsoft-store.md](../engineering/27-microsoft-store.md) and [ADR-0039](../adr/ADR-0039-store-release-licensing.md). Items marked
(confirm) come from general practice: check them against Microsoft's current policy text on the day you submit.

Who: **owner** means a person with the Partner Center login or a decision only the owner can make. **session** means a coding
session can do it in the repository. Never type a bank, tax or registration number, a personal phone number or a personal email
into the repository. The owner types those into the Partner Center forms.

## A. Order of the owner's next steps

1. Open the Partner Center developer account and finish identity or business verification (owner).
2. Reserve the name **Straft Studio** in Partner Center (owner).
3. Decide the publisher display name, see section E (owner).
4. Copy the three identity values from Product management > Product identity and give them to the repository: either commit them
   in `packaging/msix/store.config.json` or set the repository variables `MSIX_IDENTITY_NAME`, `MSIX_PUBLISHER` and
   `MSIX_PUBLISHER_DISPLAY_NAME` (owner). Until then `node scripts/check-release-placeholders.mjs` fails on purpose.
5. Choose the final licence so `LICENSE-PENDING.md` can go (owner; see `docs/implementation/owner-decisions.md`).
6. Make sure the website privacy page and contact page are live and match this repository (owner, with the website session).
7. Provide the pinned AI build inputs to the runner: repository variable `STRAFT_AI_ASSETS_DIR` (owner).
8. Tag the release (`vX.Y.Z`). The `validate-store`, `e2e` and `store` jobs run (session or owner, when the owner says so).
9. Run the Windows App Certification Kit by hand on the exact package, section D (session or owner).
10. Upload the package to a **limited audience** submission, install it from the Store, walk
    [windows-store-test.md](windows-store-test.md) and [update-simulation.md](update-simulation.md) (owner, with a session).
11. After the first submission, record the identity and version in `packaging/msix/submissions.json` (session).
12. Make the submission public, with gradual rollout (owner). After every later submission update `lastSubmittedVersion`.

## B. Both stores

- [ ] Privacy policy live at `https://straftstudio.com/legal/privacy/` and accurate: offline app, no accounts, no analytics,
  nothing sent from the app; the optional AI model download comes from Hugging Face and the host sees the download request; the
  feedback path is the website contact page and its email. Repository source: `docs/store/privacy-policy.md` (owner hosts it;
  session keeps it true).
- [ ] Support URL live: `https://straftstudio.com/contact/`, showing `support@straftstudio.com`, the company name and
  address (owner, with the website session).
  The domain and official emails were set by the owner on 2026-11-04 (ADR-0042 decision 5). Neither page is live yet. Any contact
  already entered in Partner Center or App Store Connect with the old domain must be changed to this one.
- [ ] Third-party notices: Gemma terms of use and its prohibited-use notice, the llama.cpp licence, every Rust and JavaScript
  dependency (`node scripts/license-inventory.mjs`, `THIRD_PARTY_LICENSES.md`). Shown in an About or Licences screen and shipped
  as a file (session).
- [ ] The listing says an AI model is downloaded after install, on a click (session writes, owner pastes into Partner Center).
- [ ] Store description and screenshots show the real app. No claim the app does not meet: no cloud sync, no collaboration over the
  internet (owner pastes, session checks against the feature status file).
- [ ] Age rating questionnaire answered truthfully (owner).
- [ ] No external payment link, no licence key entry, no price typed in the app, no "pay on our website" (session).
- [ ] Windows: the Store's own purchase flow plus `Check again` on the lock screen (session).
- [ ] Accessibility: keyboard reachable, screen reader names, contrast in all 15 themes for the new screens (session, with
  Narrator in [windows-store-test.md](windows-store-test.md)).
- [ ] English only; the Store listing language is English (owner).

## C. Microsoft Store only (confirm)

- [ ] The package passes the Windows App Certification Kit, with the report kept for the exact package (section D) (session or
  owner).
- [ ] Capabilities are the minimum: `internetClient` and `runFullTrust`. The manifest and `release-validate --store` agree
  (session). `runFullTrust` is explained in the submission notes, section F (owner pastes).
- [ ] The WebView2 requirement is stated in the notes, section G (owner pastes).
- [ ] Free trial set to 7 days in Pricing and availability, section H (owner).
- [ ] The publisher display name is a name the company owns, section E (owner).
- [ ] The app installs or updates nothing outside the Store: the Store build compiles the in-app updater out
  (`release-validate --store` checks it) (session).
- [ ] The only download after install is the AI model, started by a click. No code is downloaded (session, see ADR-0026).
- [ ] The package version is higher than the last submission and ends in `.0`; the identity equals the one in
  `packaging/msix/submissions.json` (checked by `release-validate --store`) (session).
- [ ] x64 only. The x64 package installs and runs on Windows 11 on ARM PCs under emulation. Confirm this in Partner Center's
  package validation on the first upload (owner). ARM64 is a later item (ADR-0039 item 17).
- [ ] Support contact, website and category entered in Properties (owner).
- [ ] At least four real, uncropped screenshots of the release candidate, 1366 by 768 or larger (owner, session helps).
- [ ] Notes for certification filled in (owner pastes, session drafts): no account or sign-in is needed, how to start a project,
  that the AI model is downloaded only after a click, and that the app works fully without it.
- [ ] Generative AI disclosed in the listing and in Partner Center, with a way to report inappropriate output (Store policy 11.16)
  (owner, session).
- [ ] Store policy version re-checked on the day (owner).

## D. Windows App Certification Kit (manual step)

WACK needs an elevated prompt and a package it can install. The Store build is unsigned (Microsoft signs it), so use a test
copy: a dev-signed build, or the Store build re-signed with a throwaway certificate that you trust on the test PC. Details and the
trust steps are in [27-microsoft-store.md](../engineering/27-microsoft-store.md) sections 3 and 8. Trusting a certificate changes
machine trust, so do it on a test PC only and remove it afterwards.

```powershell
# elevated PowerShell
$kit = 'C:\Program Files (x86)\Windows Kits\10\App Certification Kit'
& "$kit\appcert.exe" reset
& "$kit\appcert.exe" test -appxpackagepath "$PWD\target\msix\Straft-Studio-0.1.0-x64-localtest.msix" -reportoutputpath "$PWD\target\msix\wack-report.xml"
```

Replace the package name with the real test copy. Read `wack-report.xml`: every required test must pass. Keep the report with
the release evidence under `docs/release/evidence/`. A failure is fixed in the code or the packaging, then the package is built
again. The same checks run in Partner Center on upload.

## E. Publisher display name (open decision, not decided here)

The owner decides. The two choices, from package file 06 section D:

- `Vighnajith Advisories`: the company's own name. Safe.
- `Straft Studio`: needs a trademark check first.

Until it is decided the value stays the placeholder in `packaging/msix/store.config.json`, or comes from the repository variable
`MSIX_PUBLISHER_DISPLAY_NAME`. The legal publisher name used in the privacy policy and the listing is Vighnajith Advisories
Private Limited, which is a separate field from the display name.

## F. Why runFullTrust and internetClient

For the notes to certification and the restricted capability field:

- `runFullTrust` is a restricted capability that every packaged desktop (Win32) app needs. Straft Studio is a Tauri desktop
  app (Rust and Microsoft Edge WebView2), not a UWP app. It reads and writes the project folders the person chooses in
  Documents, opens files and folders in Explorer on request, and starts a local AI engine (llama.cpp) from its own package folder
  that listens on this computer only. It installs no drivers or services and needs no administrator rights. The full paste text is
  in [27-microsoft-store.md](../engineering/27-microsoft-store.md) section 7.
- `internetClient` is a general capability. The app uses it for one thing: downloading the AI model files from Hugging Face when
  the person clicks Download Straft AI. The app sends no project content, has no account and no telemetry, and has no
  in-app updater in the Store build.

## G. WebView2

A Store package cannot run the WebView2 bootstrapper. Windows 11 includes the Evergreen WebView2 Runtime. If it is missing,
the app says so in a message box and offers Microsoft's download page (`ensure_webview2()`). State this in the notes and in the
system requirements.

## H. Seven-day free trial in Partner Center

In Pricing and availability, give the app its paid price and choose the free **Time-limited trial** of 7 days, using the option
names Partner Center shows on the day. The app never shows a price: the Microsoft Store purchase window does. The target price is 3,000 rupees, 89 dollars, 89 euros (owner, set in Partner Center only). The in-app
trial length constant is 7 days (`TRIAL_DAYS`); the Store's setting and the code must agree.

## Sign-off

| Date | Package version | Who | What was done |
|---|---|---|---|
| (none yet) | | | |
