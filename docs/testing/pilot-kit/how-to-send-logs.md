# How to send logs

Nothing is uploaded by the app. You save a file and send it to the owner yourself. Attach both files to a bug report when
you can.

## Diagnostics

1. Open Settings, then **Project settings**. Scroll to the **Support** card.
2. Click **Export Diagnostics...**
3. The window shows every file the bundle will contain. You can read them before saving.
4. Choose where to save the ZIP file. Save it outside your project folder.

What it holds: app and Windows version, computer type, settings versions, AI state, and a trimmed copy of the recent log.

What it does not hold: your script or any project text, project names, file paths, your user name, email addresses, keys or
tokens. Free text in the log is replaced by "[message omitted]".

## AI report

1. Open Settings, then **Offline AI**. Find the **AI health** card.
2. Click **Export AI report**.
3. Choose where to save the JSON file.

What it holds: numbers only, such as how many requests and how long they took. No questions, answers or project text.

## Send them

Send the ZIP and the JSON file to the owner directly (email or message). No automatic upload exists. If you do not want to
send a file, tell the owner what you saw and describe the problem in words.

Logs live in `%LOCALAPPDATA%\OpenFrame\logs` if the owner asks you to look. Do not edit or delete them.
