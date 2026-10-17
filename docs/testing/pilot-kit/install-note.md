# Install note for the team

This is for the Windows 11 pilot. It takes about 10 minutes, plus the AI download.

## 1. The file you received

The installer is named like this:

`OpenFrame-Studio-0.1.0-internal-<commit>.exe`

The part after `internal-` is a short code for this exact build. Put that file name in every bug report.

## 2. Check the file is the right one

The owner sends you a long value called a SHA-256 along with the file. It proves the file was not changed on the way.

1. Open PowerShell (Start menu, type PowerShell).
2. Run this, with the real path to your file:

   `Get-FileHash "C:\Users\You\Downloads\OpenFrame-Studio-0.1.0-internal-<commit>.exe" -Algorithm SHA256`

3. Compare the long Hash value on screen with the one the owner sent. They must match exactly.
4. If they differ, do not install. Tell the owner.

## 3. "Windows protected your PC"

When you open the installer, a blue window may say "Windows protected your PC". This is expected.

Why: this is an internal build and it is not signed yet. Signing comes later, with the store release. It does not mean
the file is unsafe, as long as the SHA-256 matched in step 2.

1. Click **More info**.
2. Click **Run anyway**.

Report this window as an "installation restriction", not as a bug.

## 4. Install

- You do not need administrator rights. It installs for your Windows user only.
- Follow the installer and finish.

## 5. First start

1. Start OpenFrame Studio from the Start menu.
2. The app asks for the team password on every start. The owner gives it to you in person. Do not write it in a shared
   place or in a bug report.
3. You land on Home. Create a project or open a sample one.

## 6. Download OpenFrame AI

The assistant needs its AI models. They are not inside the installer.

1. Open Settings, then **Offline AI**, or open the AI panel and start the download there.
2. The download is large (about 1.2 GB in three files; the app shows the exact size). It can take a while on a slow
   connection.
3. You can pause it and come back. Keep the computer awake.
4. After it finishes, the AI works with no internet. Everything runs on your computer.

## 7. Where your projects live

Your projects are in `Documents\OpenFrame`. Keep a copy of anything important somewhere else, as this is a test build.
Use Create Backup before big changes.

## 8. Uninstall

Settings, Apps, OpenFrame Studio, Uninstall. The uninstaller keeps your projects. Delete the folder yourself only if you are
sure.
