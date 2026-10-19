# Install note for the team

This is for Windows 11. It takes about 10 minutes, plus the AI download.

## 1. The file you received

The owner sends you one file. Its name looks like this:

`OpenFrame-Studio-0.1.0-internal-028d8b6.exe`

The last part is a short code for this exact version. Write the file name in every bug report.

## 2. Install

1. Double-click the file.
2. Windows may show a blue window that says "Windows protected your PC". This is normal. It appears because the app is
   not yet signed, and a team test version is not. Click **More info**, then click **Run anyway**.
3. You do not need an administrator. Follow the steps and finish.
4. The app is added to your Start menu, like any other app.

Do not report the blue window as a bug.

## 3. First start

1. Open OpenFrame Studio from the Start menu.
2. The app asks for the team password every time you start it. The owner gives you the password in person. Do not write
   it in a message or in a bug report.
3. You land on Home. Make a project or open a sample one.

## 4. Download the AI (once)

The AI helper is not inside the installer. You download it one time.

1. Open Settings, then **Offline AI**, and start the download. You can also start it from the AI panel.
2. It is about 1.2 GB. You need internet for this step only. It can take a while on a slow connection.
3. You can pause it and come back later. Keep the laptop awake.
4. After it finishes, the AI works with no internet.

## 5. Where your projects are

Your projects are saved in the `Documents\OpenFrame` folder. Keep a copy of anything important somewhere else. This is a
test version.

## 6. Uninstall

Open Settings, Apps, OpenFrame Studio, then Uninstall. Uninstalling never removes your projects.

## For the owner

Before you send the installer, check its SHA-256 once on your own computer:

`Get-FileHash "<path to the .exe>" -Algorithm SHA256`

The expected value is written in `release-out` next to the file. If they match, send the file through a trusted channel
(a direct message or a private link). Testers do not check anything.
