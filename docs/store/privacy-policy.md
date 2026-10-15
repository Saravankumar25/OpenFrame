> **DRAFT, NOT FOR PUBLICATION (2026-10-05).** Placeholders remain (effective date, publisher legal name, privacy contact,
> retention; the model host placeholder returns with the user-clicked AI model download, ADR-0026). Rewrite for the chosen business model and diagnostics policy (ADR-0018, ADR-0021), have counsel
> review it, and host it on a stable HTTPS URL before Store submission (build plan area: Legal, privacy, website and support).

<!--
  DRAFT for the Microsoft Store privacy policy URL (Store policy 10.5.1; required for every Win32
  app). Before publishing:
    1. Replace every [bracketed] placeholder.
    2. Have the product owner confirm the text still matches the shipped build. Update it whenever
       a feature adds a network connection or changes what is stored.
    3. Host it at a stable public https:// URL and put that URL in packaging/msix/store.config.json
       (privacyPolicyUrl) or the STORE_PRIVACY_POLICY_URL repository variable, and in Partner Center >
       Properties > Privacy policy URL.
  Keep this comment out of the published page.
-->

# OpenFrame Studio Privacy Policy

**Effective date:** [YYYY-MM-DD]
**Publisher:** [Publisher legal name as shown in the Microsoft Store]
**Contact:** [privacy contact e-mail address or web form]

OpenFrame Studio ("OpenFrame", "the app") is a desktop application for filmmakers. It covers
ideas, story, screenplay and production planning. It is built to work on your computer, without an
account and without sending your work anywhere. This policy explains what the app stores, when it
connects to the internet, and what control you have.

## The short version

- **No account.** You don't sign up or sign in, and there is no OpenFrame account.
- **No telemetry, analytics or advertising.** OpenFrame does not collect usage statistics or crash
  reports and does not track you. It contains no ads and no third-party analytics or advertising
  code.
- **Your projects stay on your computer.** Everything you write or import is stored in files on
  your device. OpenFrame never uploads it.
- **Network use is limited.** The app connects to the internet only when you open a web link
  yourself. Offline AI is installed with the app and never uses the network. No project content is
  ever sent.
- **AI runs on your computer.** Offline AI processes your requests and project content locally.
  Nothing is sent to OpenFrame, to Google (whose Gemma model OpenFrame uses) or to any AI cloud
  service, and prompts, project text, retrieved passages, private notes and AI answers are never
  transmitted or used for telemetry.

## What OpenFrame stores, and where

All of the following is stored only on your device, in your Windows user profile:

| Data | Location | Purpose |
|---|---|---|
| Your projects: ideas, story, screenplay, breakdown, schedules, call sheets, comments, notes, and the images, audio, video and documents you add | The project folders you create (by default `Documents\OpenFrame\Projects`) | The app's core function |
| Your Global Idea Vault | `Documents\OpenFrame\Global Idea Vault` | Ideas you keep across projects |
| App settings, the list of recently opened projects, and your local display name | The app's data folder (Microsoft Store edition: the app's private package data folder; direct-download edition: `%LOCALAPPDATA%\OpenFrame`) | Remembering your preferences |
| Diagnostic log files | The app's data folder, `logs` subfolder, kept for up to 14 days | Troubleshooting. Logs record technical events (for example, that an operation failed). They don't record project text, and sensitive values are redacted. They never leave your device unless you send them to someone yourself. |
| Offline AI components: the llama.cpp runtime, the Gemma 3 1B Instruct language model and the BGE small (English) search model | Installed with the app, in its program folder (`ai` subfolder), read-only and shared by all projects. A small record of the last check of these files and the AI engine logs are kept in the app's data folder. | Running the AI assistant locally. They are part of the app: they are installed, updated and removed together with OpenFrame. |
| AI search index for a project (built on your computer when the assistant is used) | Inside the project folder, `cache\intelligence.sqlite` | Finding relevant project content for the local assistant. It is derived from the project (it contains passages of your project text, never contact details), can be deleted at any time and is rebuilt automatically. It travels with the project folder if you copy the folder. |
| Your assistant conversations and proposed changes | Inside the project file | Showing your AI history; changes are applied only when you choose Apply Changes |

Your projects can contain personal information that you enter, such as the names and contact
details of cast and crew on a call sheet. That information stays in your project files. OpenFrame
does not read it for any purpose other than showing and processing it for you on your device.

## When OpenFrame connects to the internet

1. **Offline AI never connects.** The AI assistant runs entirely on your computer with the AI
   components that are installed with the app. It never uses a cloud AI service and never downloads
   anything: no model, no update and no repair. Using AI needs no internet connection.
2. **Updates.** The Microsoft Store edition is updated by the Microsoft Store. OpenFrame itself does
   not contact any server to check for updates. [Direct-download edition: if an in-app update check
   is added, describe it here, including what the request contains and how to turn it off.]
3. **Links you open.** When you choose to open a web link, for example a link saved in the Idea
   Vault, the link opens in your default web browser. The website's own privacy policy applies.

The app displays its interface with the Microsoft Edge WebView2 Runtime, a component of Windows.
OpenFrame does not load remote web content into it, and its content security policy blocks
connections to the internet. Microsoft keeps WebView2 up to date under Microsoft's own terms.

## Sharing

OpenFrame does not sell, rent or share your information with anyone. Your content leaves your
computer only when you do it yourself: exporting a document (for example a PDF or screenplay file),
printing, or creating a review, exchange, project or backup package and passing it to someone. The
app shows what an export or package includes before it creates it, and it excludes private notes
from review and exchange packages.

## Microsoft Store and Windows

If you install OpenFrame from the Microsoft Store, Microsoft processes data about your purchase or
download, installation and updates under the
[Microsoft Privacy Statement](https://privacy.microsoft.com/privacystatement). Windows may also
collect diagnostic data, including reports about app crashes, depending on your Windows settings.
OpenFrame does not receive that data.

## Security

Your data is protected by the security of your Windows user account and your device, for example
your sign-in and, if you enable it, BitLocker device encryption. OpenFrame does not store passwords
or payment information. Before it first starts the AI, the app checks every installed AI component
against the SHA-256 checksum built into the app, and it checks the AI engine files again each time
it starts the engine; a file that fails the check is never used, and reinstalling OpenFrame repairs
it. The AI components run only on this computer, reachable
only from OpenFrame itself. The AI can read only what you are allowed to see, and it can never
change your project on its own: every change it prepares is shown to you first and applied only
when you choose Apply Changes.

## Your choices and control

- **Access and export:** your projects are ordinary folders on your computer. You can open, copy,
  back up or export them at any time.
- **Delete:** delete a project from within the app, or delete its folder. Deleted items first go to
  Recently Deleted inside the project, where you can remove them permanently.
- **Uninstall:** uninstalling the Microsoft Store edition removes the app and its private data
  folder (settings, logs) together with the AI components it installed. Your projects and Global Idea Vault in
  Documents are **not** removed, so your work is never lost by uninstalling. Delete those folders
  yourself if you want to remove them.
- **Using AI is optional:** the app works fully without it. Settings → Offline AI shows the AI
  components, their licences and checksums, and can stop the AI engine to free memory.

## Children

OpenFrame is a general-audience creative tool. It does not knowingly collect personal information
from anyone, including children, because it collects no personal information at all.

## Changes to this policy

If a future version changes what the app stores or when it connects to the internet, we will update
this policy and its effective date before that version is released.

## Contact

Questions about this policy: [privacy contact e-mail address or web form].
