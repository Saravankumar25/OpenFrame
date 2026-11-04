# 06. Placeholders and company details

## A. Company details you may use in the app, the website copy and the listings

| Item | Value |
|---|---|
| Publisher (legal name) | Vighnajith Advisories Private Limited |
| Registered address | 12/1, Ground Floor, 22nd Main Road, Bengaluru, Karnataka 560078, India |
| Product name | Straft Studio |
| Product website | https://straftstudio.com |
| Privacy policy | https://straftstudio.com/legal/privacy/ |
| Contact page (support URL) | https://straftstudio.com/contact/ |
| Public support email | support@straftstudio.com |
| Other official emails (Google Workspace on straftstudio.com) | contact@straftstudio.com (general contact), hello@straftstudio.com (greetings and press), developer@straftstudio.com (developer and store account matters), admin@straftstudio.com (account contact, see section B) |
| Company website (product page) | https://vighnajith.com/products (being added; the website prompt covers it) |
| Copyright line | Copyright 2026 Vighnajith Advisories Private Limited |
| Mac bundle ID | app.straftstudio.studio |
| Mac product IDs | app.straftstudio.studio.trial7, app.straftstudio.studio.lifetime |
| Minimum OS | Windows 11 (10.0.22000.0); macOS 14 |
| CPU | Windows x64 now, ARM64 when it passes its gate (file 03, W8); Mac Apple silicon only |

Where the company name and address may appear: the About screen, the Licences screen, the store listing fields, the privacy and terms pages, the website footer. Nowhere else.

## B. Deliberately not in this package, and never to be committed

- Bank account, PAN, GSTIN, TAN, company registration (CIN) and D-U-N-S numbers, the registration PDF.
- Any personal phone number, personal address or personal email of the owner. The public phone number on the store listings is typed by the owner into the store portals only. The admin mailbox (`admin@straftstudio.com`) used to register the store accounts is an account contact, not a public address: never print it in the app, the website or a store listing.
- Apple and Microsoft credentials, certificates, API keys, provisioning profiles.

If a task seems to need one of these, stop and put a placeholder in the config; the owner types it in the portal or in CI secrets.

## C. Placeholder rules

- Name: `REPLACE_WITH_<WHAT>`. This matches the existing guard (`PLACEHOLDER = /REPLACE_/` in `scripts/build-msix.mjs`).
- Add `scripts/check-release-placeholders.mjs` (both trees) that scans the files below for `REPLACE_WITH_` and fails. Wire it into `release-validate` for store and release builds only. Debug, test and internal builds ignore placeholders.
- Each placeholder carries a one-line comment saying who supplies it and where to find it.

## D. The placeholder list

| Placeholder | Lives in | Supplied by | Where the owner gets it |
|---|---|---|---|
| `REPLACE_WITH_PACKAGE_IDENTITY_NAME` | `windows/packaging/msix/store.config.json`, CI variable `MSIX_IDENTITY_NAME` | Owner | Partner Center, the app, Product management, Product identity |
| `CN=REPLACE_WITH_PACKAGE_IDENTITY_PUBLISHER` | same file, `MSIX_PUBLISHER` | Owner | same page (the Publisher value) |
| `REPLACE_WITH_PUBLISHER_DISPLAY_NAME` | same file, `MSIX_PUBLISHER_DISPLAY_NAME` | Owner decides | Choose between `Vighnajith Advisories` (safe, the company's own name) and `Straft Studio` (needs a trademark check first; undecided) |
| `REPLACE_WITH_STORE_PRODUCT_ID` | Store link constants if any (rate or review links) | Owner | Partner Center, the app overview (the Store ID, starts with 9) |
| `REPLACE_WITH_TEAM_ID` | Mac Keychain access group, export options, signing config | Owner | Apple Developer account, Membership |
| `REPLACE_WITH_APPLE_APP_ID` | Mac review or rate links if any | Owner | App Store Connect, the app, App Information (Apple ID) |
| `REPLACE_WITH_APP_CATEGORY` | Mac `Info.plist` | Owner | Owner picks the category in App Store Connect and tells you |
| `REPLACE_WITH_MAC_APPLICATION_IDENTITY` | Mac CI | Owner | Apple Distribution certificate name |
| `REPLACE_WITH_MAC_INSTALLER_IDENTITY` | Mac CI | Owner | Mac Installer Distribution certificate name |
| `REPLACE_WITH_PROVISIONING_PROFILE` | Mac CI secret (base64) | Owner | Apple Developer, Profiles |
| `REPLACE_WITH_ASC_API_KEY_ID`, `..._ISSUER_ID`, `..._KEY` | Mac CI secrets for upload | Owner | App Store Connect, Users and Access, Integrations |
| Social URLs (8) | `rules.ts` `SOCIAL_LINKS`, the Swift equivalent | Owner | See file 07. Until then they stay `null` and the row is hidden in release builds |
| Lifetime price | Never in code | Owner | Set in App Store Connect and Partner Center; the apps read it from the store |
| Screenshots, descriptions, keywords, age rating, tax and banking forms | Store portals | Owner | Not your job |

CI secrets for the Windows Store channel: none (Microsoft signs the MSIX). The direct-installer channel keeps its own existing secrets; do not touch them.

## E. Files to scan with the placeholder check
`windows/packaging/msix/store.config.json`, `windows/packaging/msix/AppxManifest.template.xml` output, `mac/apps/macos/packaging/*`, `mac/apps/macos/Sources/**/*.swift` (store constants only), `.github/workflows/release.yml` (both trees), `apps/desktop/src/features/licensing/rules.ts`.
