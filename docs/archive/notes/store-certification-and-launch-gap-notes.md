> **ARCHIVED INTAKE NOTE (2026-10-05).** Raw notes supplied by the owner. Content is absorbed into
> [docs/README.md](../../README.md), [27-microsoft-store.md](../../engineering/27-microsoft-store.md) and the launch register Appendix F.
> See [03-reconciliation-log.md](../../implementation/03-reconciliation-log.md) §2. Do not edit.

**Yes, it is entirely possible to pass Microsoft Store certification on your first attempt.**

Unlike Apple's App Store, which relies heavily on subjective human curation, the majority of Microsoft Store certification is **automated technical verification** followed by policy and metadata checks. If your package passes the automated test suite and your store listing matches Microsoft's guidelines, it will pass without friction.

---

### 1. Run the Local Test Tool First (The "Cheat Code")

Before uploading anything to Partner Center, run the **Windows App Certification Kit (WACK)** locally on your development machine:

* **What it is:** WACK is the exact same automated testing suite Microsoft runs in the cloud during certification.
* **How to run it:** It comes pre-installed with the Windows SDK (or accessible via `appcert.exe` / Visual Studio).
* **Rule:** If WACK outputs an XML/HTML report showing **Pass** (all green) with zero fatal errors, your app is guaranteed to pass the automated technical phase (crashes, memory leaks, restricted API calls, and manifest formatting).

---

### 2. The 5 Most Common Reasons Submissions Fail

Most first-time rejections happen not because of complex bugs, but because of avoidable policy and metadata oversights:

1. **Missing or Inaccessible Privacy Policy URL:**
* If your app connects to the internet, uses analytics, or gathers crash logs, Microsoft policy requires a live, publicly accessible Privacy Policy link in your submission. A broken link or placeholder (`example.com`) causes an instant automated rejection.


2. **First-Launch Crash on a Clean Machine:**
* Reviewers test the package on a bare-bones, freshly installed Windows Sandbox/VM. If your app implicitly relies on dependencies you installed locally during development (e.g., specific Visual C++ Redistributables, .NET runtimes, or bundled webview runtimes) that aren't packaged with the installer, it will crash on launch and be rejected.


3. **Mismatched Age Rating Questionnaire:**
* When completing the IARC (International Age Rating Coalition) questionnaire in Partner Center, answer accurately. If your app handles user-generated content or features screenplays with mature themes/violence and you check "No violence/mature content," reviewers will reject it for rating discrepancies.


4. **Misleading or Missing "Notes for Certification":**
* If your app requires a login, an account, or an active test key to evaluate features, you **must** provide dummy test credentials in the *Notes for Certification* field. If the tester cannot get past the opening screen, they will reject it immediately.


5. **Restricted Capabilities in the App Manifest:**
* If you package via MSIX, only declare capabilities your app actually uses (e.g., `internetClient`, `runFullTrust`). Requesting broad system permissions without clear operational justification will trigger a security flag.



---

### 3. Packaging Format: MSIX vs. Raw EXE/MSI

How you package your software changes the test criteria:

* **Packaged as MSIX (Recommended):**
* Passes cleanest because Windows handles sandboxing, background installation, and 100% clean uninstallation automatically.
* Integrates natively with Microsoft Store licensing and trial APIs.


* **Direct EXE / MSI Installer:**
* Must support **silent installation flags** (`/quiet` or `/s`) without spawning unexpected UI popups.
* Must be a **standalone offline installer** (not a tiny web bootstrapper that downloads code from external servers during setup).
* Must register proper metadata in Windows Add/Remove Programs and uninstall completely cleanly.



---

### The Pre-Submission Checklist

* [ ] Tested the build in a fresh **Windows Sandbox** or VM to verify all dependencies are bundled.
* [ ] Ran the **Windows App Certification Kit (WACK)** locally and resolved all warnings/errors.
* [ ] Provided a working **Privacy Policy URL** hosted on HTTPS.
* [ ] Added **test credentials / demo license notes** in the Partner Center submission box.
* [ ] Uploaded at least one clear, un-cropped screenshot showing the actual in-app UI.
* [ ] If using a trial, verified that the app functions properly when `StoreAppLicense.IsTrial` is active.BRAND / TRADEMARK
Before launch:
- [ ] OpenFrame name availability
- [ ] domain
- [ ] social handles
- [ ] logo ownership
- [ ] font licenses
- [ ] trademark search
- [ ] app-store identity
- [ ] company/publisher identity
You don't necessarily need to file every trademark immediately, but do the clearance work before spending heavily on the brand.From everything you've told me, these are the big things I would add to your original list:
🔴 Critical
1. Formal release criteria / Definition of Done
2. FDX migration regression suite
3. PDF visual regression
4. Crash recovery testing
5. Backup/restore disaster test
6. Upgrade/migration testing
7. Offline testing
8. Real filmmaker beta
9. Windows hardware matrix
10. Security/dependency audit
11. Microsoft Store certification test
12. Privacy policy
13. Terms/EULA
14. Refund policy
15. Support system
16. Crash/diagnostic system
17. Third-party license audit
18. Accessibility testing
19. Analytics/funnel
20. Post-launch incident response
🟠 Important
21. macOS RC + testing
22. Website SEO
23. Social account reservation
24. 30-day content bank
25. Creator outreach list
26. Reddit community strategy
27. Discord partnerships
28. Launch trailer
29. Founder story
30. Email/transactional communication
31. Store screenshots/video
32. localization architecture
33. performance benchmarks
34. public documentation
35. release notes/changelog
36. versioning strategy
37. support SLA/response target
38. domain/email infrastructure
39. status/outage communication
40. customer feedback systemDocumentation you can start with new features nundi audit and end to end implementation and hld lld and vibe coding and os compatibility and master checklist