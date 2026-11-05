# Straft Studio: Trial, Purchase, First-Run Safety and Logo (final package revision)

Companion to the HTML mocks in this folder (screens 182 to 203; open `index.html`). The folder has its own copies of `straft.css`, `themes.css` and `themes.js` in `assets/`. Screenshots of all 22 pages are in `Trial_Licensing_Screenshots/` (same file names, `.png`). The **Theme** button at the bottom-right of any page switches theme and Light/Dark/System, and the logo recolours with it.

The rules in `../02_TRIAL_AND_LICENSING_SPEC.md` win over this note if the two ever differ. This note describes the screens.

## 1. What changed in this revision

- Chip text is now **"Trial ends in N days"** (7 down to 1), on Application Home and inside projects, both platforms. (Was "Trial: N days left".)
- New mocks: 199 (Windows Home chip), 200 (Mac Home chip), 201 (Mac clock looks wrong), 202 (Windows locked while open), 203 (the website contact page).
- "Need help? Contact us" added to the lock, check, clock and first-launch screens. It opens the website contact page. No form in the app.
- Mac first launch (187) and lock (190) show the later price as a token, `{price from the App Store}`. The real price comes from StoreKit at run time. Never type a price.
- The Mac no longer uses a licence server. The trial is counted offline (file 04).
- Windows is unchanged in principle: the Microsoft Store owns the trial clock and expiry.
- Logo and theme notes (section 5) are unchanged.

## 2. What the screens do

1. 7-day free trial.
2. **Windows:** the Microsoft Store runs the trial clock, purchase and expiry. The app shows the chip, the Buy button, the reminders, and a lock screen if the Store says the trial ended while the app is open.
3. **Mac:** the app counts the trial offline from an Apple-signed zero-price "7-day Trial" item, with a clock guard. First launch offers Free trial or Buy.
4. **Reminders:** trial days 5, 6 and 7 only, once per day, over a project or on Home, both platforms.
5. **Chip and Buy button:** top-right for the whole trial, until purchase.
6. **Trial over:** Windows shows the Store's message and our lock if it ends while open. Mac shows a lock that cannot be closed.
7. **After buying (both platforms), once and in this order:** data note, 4-step tutorial, welcome popup. Trial users go straight into the app. Buyers never see the chip or a reminder again.
8. **Contact:** one link to `https://straftstudio.com/contact/`. Nothing is sent from the app.

## 3. Screens

| No. | File | Screen |
|---|---|---|
| 182 | `182-lic-flow.html` | The whole flow at a glance |
| 183 | `183-lic-win-trial.html` | Windows: chip and Buy full version button in a project |
| 184 | `184-lic-win-reminder.html` | Windows: reminder, 3 days left |
| 185 | `185-lic-win-reminder-last.html` | Windows: reminder on the final day |
| 186 | `186-lic-win-ended.html` | Windows: trial ended (Microsoft Store message) |
| 187 | `187-lic-mac-first.html` | Mac: first launch, free trial or buy |
| 188 | `188-lic-mac-trial.html` | Mac: chip and Buy full version button in a project |
| 189 | `189-lic-mac-reminder.html` | Mac: reminder |
| 190 | `190-lic-mac-expired.html` | Mac: lock when the trial is over |
| 191 | `191-lic-mac-apple.html` | Mac: the Apple payment sheet (Apple owned) |
| 192 | `192-lic-mac-verify.html` | Mac: checking or restoring a purchase, offline |
| 193 | `193-lic-data-note.html` | Mandatory note: your data lives on your computer |
| 194 | `194-lic-tut-1.html` | Tutorial step 1: where your data lives |
| 195 | `195-lic-tut-2.html` | Tutorial step 2: back up to Google Drive |
| 196 | `196-lic-tut-3.html` | Tutorial step 3: moving to a new laptop |
| 197 | `197-lic-tut-4.html` | Tutorial step 4: final check |
| 198 | `198-lic-welcome.html` | Welcome to the Straft Studio community (shown once) |
| 199 | `199-lic-win-home.html` | Windows: chip on Application Home |
| 200 | `200-lic-mac-home.html` | Mac: chip on Application Home |
| 201 | `201-lic-mac-clock.html` | Mac: the clock looks wrong (cannot be closed) |
| 202 | `202-lic-win-locked.html` | Windows: locked after the trial ended while open |
| 203 | `203-lic-contact.html` | The website contact page that "Contact us" opens |

## 4. Rules to implement (short form; full rules in file 02)

**Reminders:** trial days 5, 6, 7 only, at most once per calendar day, never after purchase. Wording: "ends in 3 days", "ends in 2 days", "ends today". Buttons: Buy full version and Continue trial. Remember "shown today" in the notice record.

**Chip:** "Trial ends in N days". It turns amber in the last 3 days. Chip and button disappear at once after purchase.

**Windows:** trial, purchase and expiry belong to the Microsoft Store. No first-launch choice screen on Windows.

**Mac:** see `../04_MAC_TASKS.md`. Offline, no server, no device binding.

**After purchase (both platforms):** keep the flags "data note acknowledged", "tutorial completed", "welcome shown"; each is shown once. The note and tutorial cannot be skipped or closed. OK on the note is enabled only after the list is scrolled to the end and the box is ticked. Finish on the tutorial is enabled only when all three boxes are ticked.

**Data note, 6 points (draft wording):** (1) projects live only on this computer, nobody else holds a copy, so a lost laptop cannot be recovered; (2) back up the whole root folder to Google Drive or another cloud drive and an external disk, and repeat often; (3) moving to a new laptop means copying the root folder and using Open Project; (4) keep at least two copies; (5) never delete or rename the root folder by hand; (6) in-app backups are separate copies and should also be kept off this computer. The real root folder path is shown with a "Show in File Explorer / Finder" button. Add the one true uninstall sentence per platform from file 05, U9.

**Tutorial:** step 1 where your data lives; step 2 back up to Google Drive (or an external disk); step 3 moving to a new laptop (including linked external files); step 4 final check with 3 confirmations.

**Welcome popup:** "Welcome to the Straft Studio community. Thank you for backing independent filmmaking. Be a part of our community." then the eight social buttons with their real addresses (file 07, superseded 2026-11-05), then OK.

## 5. Logo: details and code (supports global themes)

**Design (final Straft artwork):**
- Mark: a rounded film frame drawn as one filled shape on a 1024 by 1024 grid (outer rounded square, eight sprocket holes on the left and right edges, two picture windows in the middle). It is never stretched, outlined or recoloured.
- Mark colour: a diagonal gradient from top left to bottom right with three fixed stops, `#ff5e6c`, `#ff9a45`, `#ffc13d`. The gradient does **not** change with the theme.
- Words: "Straft" and "Studio" drawn as outlines in the same artwork, so they look the same on every machine whatever fonts are installed. "Straft" takes the theme's main text colour (`currentColor`, light on dark surfaces). "Studio" is the brand orange.
- Sizes used in the mock: top bar logo 30px high, welcome and block popups 44px high (class `lg`); the width follows the artwork. The window title bar text "Straft Studio, BLACK RAIN" is separate and unchanged; it uses the 16px mark only.
- The logo has no setting of its own. Choosing a theme or Light/Dark changes only the colour of the word "Straft".

**How it works with the global themes (the real system in `themes.css` / `themes.js`).** Themes set colours through variables on the root element. The logo reads only one of them, the text colour, through `currentColor`. Everything else in the logo is fixed artwork.

**Logo markup (HTML + SVG), reusable everywhere:**

```html
<span class="brandlogo" role="img" aria-label="Straft Studio"><svg viewBox="84.5 41.5 1248 277.3" aria-hidden="true" focusable="false"><defs><linearGradient id="lg-brand" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#FF5E6C"></stop><stop offset="0.5" stop-color="#FF9A45"></stop><stop offset="1" stop-color="#FFC13D"></stop></linearGradient></defs><g transform="translate(70 26.4) scale(0.3)"><path fill-rule="evenodd" fill="url(#lg-brand)" d="M179,51H844A128,128 0 0 1 972,179V844A128,128 0 0 1 844,972H179A128,128 0 0 1 51,844V179A128,128 0 0 1 179,51ZM179,119H264V264H119V179A60,60 0 0 1 179,119ZM759,119H844A60,60 0 0 1 904,179V264H759V119ZM119,332H264V478H119V332ZM759,332H904V478H759V332ZM119,546H264V691H119V546ZM759,546H904V691H759V546ZM119,759H264V904H179A60,60 0 0 1 119,844V759ZM759,759H904V844A60,60 0 0 1 844,904H759V759ZM332,119H691V478H332V119ZM332,546H691V904H332V546Z"></path></g><path fill="currentColor" d="M477.68 236.25Q465.83 236.25 456.34 232.13Q446.85 228 440.74 220.31Q434.63 212.63 432.98 202.05L451.73 199.2Q454.13 208.8 461.59 214.05Q469.05 219.3 478.8 219.3Q484.58 219.3 489.67 217.5Q494.78 215.7 497.96 212.25Q501.15 208.8 501.15 203.78Q501.15 201.53 500.4 199.61Q499.65 197.7 498.19 196.2Q496.73 194.7 494.44 193.46Q492.15 192.23 489.15 191.33L461.25 183.08Q457.65 182.03 453.49 180.26Q449.33 178.5 445.61 175.39Q441.9 172.28 439.54 167.44Q437.18 162.6 437.18 155.48Q437.18 145.05 442.46 137.96Q447.75 130.88 456.71 127.35Q465.68 123.83 476.63 123.9Q487.73 123.98 496.42 127.69Q505.13 131.4 510.98 138.45Q516.83 145.5 519.23 155.48L499.8 158.85Q498.6 153.15 495.19 149.14Q491.78 145.13 486.86 143.03Q481.95 140.93 476.4 140.78Q471 140.7 466.39 142.39Q461.78 144.07 458.96 147.23Q456.15 150.38 456.15 154.57Q456.15 158.55 458.55 161.06Q460.95 163.57 464.51 165.04Q468.08 166.5 471.75 167.48L491.1 172.88Q495.08 173.93 500.03 175.73Q504.98 177.53 509.55 180.75Q514.13 183.98 517.09 189.23Q520.05 194.48 520.05 202.58Q520.05 210.98 516.56 217.31Q513.08 223.65 507.15 227.85Q501.23 232.05 493.61 234.15Q486 236.25 477.68 236.25ZM582.8 234Q574.78 235.5 567.09 235.31Q559.4 235.13 553.33 232.46Q547.25 229.8 544.1 224.03Q541.33 218.78 541.18 213.3Q541.03 207.83 541.03 200.93V130.5H559.03V199.88Q559.03 204.75 559.14 208.43Q559.25 212.1 560.68 214.43Q563.38 218.93 569.3 219.45Q575.23 219.98 582.8 218.85ZM526.33 167.18V153H582.8V167.18ZM599.28 234V153H615.25V172.73L613.3 170.18Q614.8 166.13 617.31 162.79Q619.83 159.45 623.35 157.28Q626.35 155.25 629.99 154.09Q633.63 152.93 637.45 152.66Q641.28 152.4 644.88 153V169.88Q641.28 168.83 636.59 169.2Q631.9 169.58 628.15 171.83Q624.4 173.85 622.04 177Q619.68 180.15 618.55 184.16Q617.43 188.18 617.43 192.83V234ZM680.78 236.25Q672 236.25 665.93 232.91Q659.85 229.58 656.74 224.02Q653.63 218.48 653.63 211.8Q653.63 205.95 655.58 201.26Q657.53 196.58 661.58 193.16Q665.63 189.75 672.08 187.58Q676.95 186 683.51 184.73Q690.08 183.45 697.76 182.36Q705.45 181.28 713.85 180L707.4 183.68Q707.48 175.28 703.65 171.3Q699.83 167.33 690.75 167.33Q685.28 167.33 680.18 169.88Q675.08 172.43 673.05 178.65L656.55 173.48Q659.55 163.2 667.99 156.98Q676.43 150.75 690.75 150.75Q701.55 150.75 709.76 154.28Q717.98 157.8 721.95 165.83Q724.13 170.1 724.58 174.56Q725.03 179.03 725.03 184.28V234H709.2V216.45L711.83 219.3Q706.35 228.08 699.04 232.16Q691.73 236.25 680.78 236.25ZM684.38 221.85Q690.53 221.85 694.88 219.68Q699.23 217.5 701.81 214.35Q704.4 211.2 705.3 208.43Q706.73 204.98 706.91 200.51Q707.1 196.05 707.1 193.28L712.65 194.93Q704.48 196.2 698.63 197.18Q692.78 198.15 688.58 199.01Q684.38 199.88 681.15 200.93Q678 202.05 675.83 203.55Q673.65 205.05 672.49 207Q671.33 208.95 671.33 211.58Q671.33 214.58 672.83 216.86Q674.33 219.15 677.21 220.5Q680.1 221.85 684.38 221.85ZM750.58 234V151.73Q750.58 148.65 750.8 144.94Q751.03 141.23 752.08 137.55Q753.13 133.88 755.9 130.88Q759.35 127.05 763.36 125.59Q767.38 124.13 771.28 123.94Q775.18 123.75 778.1 123.75H788.23V138.45H778.85Q773.68 138.45 771.16 141.04Q768.65 143.63 768.65 147.82V234ZM737.53 167.18V153H788.23V167.18ZM846.7 234Q838.68 235.5 830.99 235.31Q823.3 235.13 817.23 232.46Q811.15 229.8 808 224.03Q805.23 218.78 805.08 213.3Q804.93 207.83 804.93 200.93V130.5H822.93V199.88Q822.93 204.75 823.04 208.43Q823.15 212.1 824.58 214.43Q827.28 218.93 833.2 219.45Q839.13 219.98 846.7 218.85ZM790.23 167.18V153H846.7V167.18Z"></path><path fill="#FF9A45" d="M935.68 236.25Q924.35 236.25 915.31 232.46Q906.28 228.68 900.39 221.7Q894.5 214.73 892.7 205.13L903.95 203.25Q906.65 213.75 915.35 219.71Q924.05 225.68 936.13 225.68Q944.08 225.68 950.38 223.16Q956.68 220.65 960.28 216Q963.88 211.35 963.88 204.98Q963.88 200.78 962.38 197.81Q960.88 194.85 958.4 192.9Q955.93 190.95 953 189.64Q950.08 188.33 947.3 187.5L919.93 179.33Q914.98 177.9 910.78 175.69Q906.58 173.48 903.46 170.36Q900.35 167.25 898.63 163.05Q896.9 158.85 896.9 153.45Q896.9 144.3 901.7 137.63Q906.5 130.95 914.83 127.35Q923.15 123.75 933.8 123.83Q944.6 123.83 953.11 127.65Q961.63 131.48 967.18 138.56Q972.73 145.65 974.53 155.32L962.98 157.43Q961.85 150.38 957.8 145.2Q953.75 140.03 947.49 137.21Q941.23 134.4 933.73 134.32Q926.45 134.32 920.79 136.76Q915.13 139.2 911.83 143.4Q908.53 147.6 908.53 153Q908.53 158.48 911.56 161.78Q914.6 165.08 919.06 166.95Q923.53 168.83 927.73 170.03L948.88 176.25Q952.18 177.15 956.64 178.84Q961.1 180.53 965.34 183.64Q969.58 186.75 972.35 191.78Q975.13 196.8 975.13 204.53Q975.13 212.03 972.16 217.95Q969.2 223.88 963.84 227.93Q958.48 231.98 951.31 234.11Q944.15 236.25 935.68 236.25ZM1033.75 234Q1026.93 235.43 1020.29 235.13Q1013.65 234.83 1008.44 232.27Q1003.23 229.73 1000.53 224.33Q998.35 219.83 998.16 215.21Q997.98 210.6 997.98 204.68V130.5H1008.93V204.23Q1008.93 209.33 1009.04 212.74Q1009.15 216.15 1010.58 218.78Q1013.28 223.8 1019.16 224.78Q1025.05 225.75 1033.75 224.33ZM980.65 162.45V153H1033.75V162.45ZM1079.7 236.03Q1072.43 236.03 1066.99 233.78Q1061.55 231.53 1057.76 227.7Q1053.98 223.88 1051.65 218.96Q1049.33 214.05 1048.28 208.54Q1047.23 203.03 1047.23 197.55V153H1058.33V194.33Q1058.33 201 1059.6 206.7Q1060.88 212.4 1063.69 216.64Q1066.5 220.88 1070.96 223.2Q1075.43 225.53 1081.8 225.53Q1087.65 225.53 1092.11 223.5Q1096.58 221.48 1099.61 217.69Q1102.65 213.9 1104.23 208.46Q1105.8 203.03 1105.8 196.2L1113.6 197.93Q1113.6 210.38 1109.25 218.85Q1104.9 227.33 1097.25 231.68Q1089.6 236.03 1079.7 236.03ZM1106.93 234V214.05H1105.8V153H1116.83V234ZM1168.25 236.25Q1156.93 236.25 1148.9 230.59Q1140.88 224.93 1136.6 215.25Q1132.33 205.58 1132.33 193.43Q1132.33 181.35 1136.56 171.68Q1140.8 162 1148.75 156.38Q1156.7 150.75 1167.8 150.75Q1179.2 150.75 1187 156.3Q1194.8 161.85 1198.81 171.49Q1202.83 181.13 1202.83 193.43Q1202.83 205.5 1198.85 215.21Q1194.88 224.93 1187.15 230.59Q1179.43 236.25 1168.25 236.25ZM1169.38 225.98Q1178 225.98 1183.63 221.77Q1189.25 217.58 1192.03 210.19Q1194.8 202.8 1194.8 193.43Q1194.8 183.9 1192.03 176.59Q1189.25 169.28 1183.66 165.15Q1178.08 161.03 1169.6 161.03Q1160.9 161.03 1155.2 165.3Q1149.5 169.58 1146.76 176.93Q1144.03 184.28 1144.03 193.43Q1144.03 202.65 1146.84 210.04Q1149.65 217.43 1155.28 221.7Q1160.9 225.98 1169.38 225.98ZM1194.8 234V173.03H1193.6V126H1204.7V234ZM1226.2 137.63V125.25H1237.23V137.63ZM1226.2 234V153H1237.23V234ZM1292.55 236.25Q1280.48 236.25 1271.93 230.78Q1263.38 225.3 1258.8 215.63Q1254.23 205.95 1254.23 193.43Q1254.23 180.68 1258.88 171.08Q1263.53 161.48 1272.11 156.11Q1280.7 150.75 1292.55 150.75Q1304.7 150.75 1313.29 156.19Q1321.88 161.63 1326.41 171.26Q1330.95 180.9 1330.95 193.43Q1330.95 206.18 1326.38 215.81Q1321.8 225.45 1313.18 230.85Q1304.55 236.25 1292.55 236.25ZM1292.55 225.68Q1306.05 225.68 1312.65 216.71Q1319.25 207.75 1319.25 193.43Q1319.25 178.73 1312.61 170.02Q1305.98 161.32 1292.55 161.32Q1283.48 161.32 1277.59 165.41Q1271.7 169.5 1268.81 176.74Q1265.93 183.98 1265.93 193.43Q1265.93 208.05 1272.68 216.86Q1279.43 225.68 1292.55 225.68Z"></path></svg></span>
```

Mark alone (title-bar icon, 16px):

```html
<svg class="mk" viewBox="0 0 1024 1024" aria-hidden="true" focusable="false"><defs><linearGradient id="lg-brand" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#FF5E6C"></stop><stop offset="0.5" stop-color="#FF9A45"></stop><stop offset="1" stop-color="#FFC13D"></stop></linearGradient></defs><path fill-rule="evenodd" fill="url(#lg-brand)" d="M179,51H844A128,128 0 0 1 972,179V844A128,128 0 0 1 844,972H179A128,128 0 0 1 51,844V179A128,128 0 0 1 179,51ZM179,119H264V264H119V179A60,60 0 0 1 179,119ZM759,119H844A60,60 0 0 1 904,179V264H759V119ZM119,332H264V478H119V332ZM759,332H904V478H759V332ZM119,546H264V691H119V546ZM759,546H904V691H759V546ZM119,759H264V904H179A60,60 0 0 1 119,844V759ZM759,759H904V844A60,60 0 0 1 844,904H759V759ZM332,119H691V478H332V119ZM332,546H691V904H332V546Z"></path></svg>
```

```css
.brandlogo { display:inline-flex; align-items:center; color:var(--ink); }
.brandlogo svg { display:block; height:30px; width:auto; flex:none; }
.brandlogo.lg svg { height:44px; }
.titlebar .logo .mk { display:block; width:16px; height:16px; }
```

**Applying a theme (JavaScript, from `themes.js`).** Choosing a theme only sets two attributes; all colours follow (the logo keeps its own colours; only the word "Straft" follows the text colour):

```js
document.documentElement.setAttribute('data-theme', 'midnight-reel'); // any of the 15 theme ids
document.documentElement.setAttribute('data-mode',  'dark');          // light | dark (System resolved from the OS)
```

Working files: `assets/straft.css` (rules `.brandlogo`), `assets/themes.css`, `assets/themes.js`.

**Places to replace the logo:** title-bar icon (mark only), top-bar brand, splash, About box, installer and store listing icons.


## 6. Assumptions
1. The data note and tutorial appear only after purchase (trial users skip them).
2. Reminder days are days 5, 6 and 7 of 7.
3. Prices are never typed in Straft. The Mac shows the StoreKit price on two screens; Windows shows none.
4. All wording is draft text. Social addresses are real since 2026-11-05 (file 07).
5. Screen 186 is a Store-owned message; we do not build it.
6. The Word manual's screenshots were not regenerated and still show an earlier placeholder mark.
