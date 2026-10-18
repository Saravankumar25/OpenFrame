# Install note for the team (Mac)

This is for the Mac pilot. You need an Apple silicon Mac (M1 or later) with macOS 14 or later. It takes about
10 minutes, plus the AI download.

## 1. The file you received

The disk image is named like this:

`OpenFrame-Studio-0.1.0-internal-<commit>.dmg`

The part after `internal-` is a short code for this exact build. Put that file name in every bug report.

## 2. Check the file is the right one

The owner sends you a long value called a SHA-256 along with the file. It proves the file was not changed on the way.

1. Open Terminal (Applications > Utilities > Terminal).
2. Run this, with the real path to your file:

   `shasum -a 256 ~/Downloads/OpenFrame-Studio-0.1.0-internal-<commit>.dmg`

3. Compare the long value on screen with the one the owner sent. They must match exactly.
4. If they differ, do not install. Tell the owner.

## 3. Install

1. Double-click the `.dmg`. A window opens with OpenFrame Studio and an Applications folder.
2. Drag OpenFrame Studio onto Applications.
3. Eject the disk image (the eject button next to "OpenFrame Studio" in the Finder sidebar).

## 4. "macOS cannot verify" on the first start

This is an internal build. It is not signed by Apple yet; signing comes with the store release. macOS therefore stops it
the first time. This is expected, as long as the SHA-256 matched in step 2. Do one of these, once:

- **Control-click:** in Applications, hold Control, click OpenFrame Studio, choose **Open**, then click **Open** in the
  message.
- **macOS 15 and later**, if there is no Open button: try to open the app once, then open **System Settings > Privacy &
  Security**, scroll down to the message about OpenFrame Studio, click **Open Anyway** and confirm.
- **Terminal**, if neither works:

  `xattr -dr com.apple.quarantine "/Applications/OpenFrame Studio.app"`

After that the app opens normally.

## 5. First start

1. The app asks for the team password. The owner gives it to you in person. It is the same for the whole team.
2. A wrong password is refused, and after several wrong tries the app makes you wait a little.
3. Create a project and start writing. Everything is saved on this Mac as you type.

## 6. The offline AI

The AI is not in the disk image. It is downloaded inside the app, only when you ask for it:

1. Open **OpenFrame Studio > Settings > AI** (or the assistant button in a project) and choose **Download OpenFrame AI**.
2. The download is about 1 GB. You can keep working while it runs.
3. After the download the AI works with Wi-Fi off. Nothing you write is sent anywhere.

## 7. Updating to a newer build

Quit OpenFrame Studio, then drag the new app onto Applications and choose **Replace**. Your projects stay where they are.

## 8. Removing the app

Drag OpenFrame Studio from Applications to the Bin. Your projects are not deleted: they stay in the folders you chose
(by default the OpenFrame folder in your Documents). Settings and logs are in
`~/Library/Application Support/OpenFrame`; delete that folder too only if you want nothing left.
