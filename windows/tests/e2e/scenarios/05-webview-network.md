# The web view stays off the network

Spec: `tests/e2e/specs/05-webview-network.test.mjs`

Tags: `@e2e-data-root: own`, `@e2e-net-log`

Before: an empty data folder of its own. The app starts for the first time. The web view keeps a network log of its own network code in the data folder.

"An allowed connection" means the one exception the owner allowed on 2026-10-16: a TCP connection to port 443 held by the main web view process. It is the web view runtime's own Microsoft diagnostic-data upload, which follows the Windows diagnostic data setting and which no app can switch off. Any other connection outside this computer fails.

## Driven by the test driver, with a project open, no web view process of the app leaves this computer

1. Do the first launch: name 'E2E Tester', project title 'Quiet Room'.
2. Visit each workspace in turn: Idea Vault, Story, Screenplay, Breakdown, Production, Call Sheets, Files. After each one, record every network connection held by the web view processes of the app.
3. Keep recording once a second for 10 seconds.
4. Check: at least two web view processes of the app were found.
5. Check: the connection query sees a listener the test opens on this computer for the purpose (so "nothing seen" means nothing was there).
6. Check: the main web view process was started with every argument the app sets for its web view in its configuration.
7. Check: apart from an allowed connection, no web view process of the app held a connection outside this computer, none listened on an outside address, and none used UDP outside this computer.
8. Check: the web view's network log names no host outside this computer (only the app's own local addresses).

## Started directly, as a person starts it, the web view stays on this computer for 45 seconds

1. Quit the app and check that it has ended.
2. Start the app directly, without the test driver, on the same data folder.
3. Record every network connection held by the web view processes of the app, once a second for 45 seconds.
4. Check: at least two web view processes of the app were found.
5. Check: the connection query sees the test's own listener.
6. Check: the main web view process was started with every argument the app sets for its web view in its configuration.
7. Check: apart from an allowed connection, no web view process of the app held a connection outside this computer.
8. End the app.

## Platform notes

- The web view is WebView2 (`msedgewebview2.exe` processes). The second run exists because msedgedriver replaces the web view arguments with its own. Mac: WKWebView runs in WebKit helper processes; the check moves to those processes and to whatever the SwiftUI shell uses. To decide in phase M-track.
- The list of web view arguments lives in the Tauri configuration (`additionalBrowserArgs`). Mac: to decide in phase M-track.
- The network log is Chromium's `--log-net-log`. Mac: to decide in phase M-track.
- Connections are read with the Windows network commands. Mac: an equivalent such as `lsof` or `nettop`, to decide in phase M-track.
