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
- **Network use is limited and optional.** The app connects to the internet only when you choose to
  download optional components (the local AI runtime and AI models) or open a web link yourself.
  No project content is ever sent.

## What OpenFrame stores, and where

All of the following is stored only on your device, in your Windows user profile:

| Data | Location | Purpose |
|---|---|---|
| Your projects: ideas, story, screenplay, breakdown, schedules, call sheets, comments, notes, and the images, audio, video and documents you add | The project folders you create (by default `Documents\OpenFrame\Projects`) | The app's core function |
| Your Global Idea Vault | `Documents\OpenFrame\Global Idea Vault` | Ideas you keep across projects |
| App settings, the list of recently opened projects, and your local display name | The app's data folder (Microsoft Store edition: the app's private package data folder; direct-download edition: `%LOCALAPPDATA%\OpenFrame`) | Remembering your preferences |
| Diagnostic log files | The app's data folder, `logs` subfolder, kept for up to 14 days | Troubleshooting. Logs record technical events (for example, that an operation failed). They don't record project text, and sensitive values are redacted. They never leave your device unless you send them to someone yourself. |
| Downloaded AI runtime and AI models (only if you install them) | The app's data folder | Running the optional AI assistant locally |

Your projects can contain personal information that you enter, such as the names and contact
details of cast and crew on a call sheet. That information stays in your project files. OpenFrame
does not read it for any purpose other than showing and processing it for you on your device.

## When OpenFrame connects to the internet

1. **Optional AI component downloads.** The AI assistant runs entirely on your computer. It never
   uses a cloud AI service. To use it, you first choose to download the AI runtime and a model
   file. The download goes to [model download host, e.g. https://models.example.org] over an
   encrypted (HTTPS) connection. As with any download, that server receives your IP address,
   standard technical request headers and the name of the file you requested. No project content,
   prompts, account identifier or device identifier is sent. [Describe how long the download
   host keeps its server logs, or state that it keeps none.]
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
or payment information. Downloads use HTTPS. [Confirm against the shipped AI module before
publishing: "The app verifies the checksum of downloaded AI components before it runs them."]

## Your choices and control

- **Access and export:** your projects are ordinary folders on your computer. You can open, copy,
  back up or export them at any time.
- **Delete:** delete a project from within the app, or delete its folder. Deleted items first go to
  Recently Deleted inside the project, where you can remove them permanently.
- **Uninstall:** uninstalling the Microsoft Store edition removes the app and its private data
  folder (settings, logs, downloaded AI components). Your projects and Global Idea Vault in
  Documents are **not** removed, so your work is never lost by uninstalling. Delete those folders
  yourself if you want to remove them.
- **AI downloads are optional:** the app works without them.

## Children

OpenFrame is a general-audience creative tool. It does not knowingly collect personal information
from anyone, including children, because it collects no personal information at all.

## Changes to this policy

If a future version changes what the app stores or when it connects to the internet, we will update
this policy and its effective date before that version is released.

## Contact

Questions about this policy: [privacy contact e-mail address or web form].
