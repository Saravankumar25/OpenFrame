# Mac App Store placeholders

Every value only Apple or the owner can give. A store or release build fails while one remains
(`node scripts/check-release-placeholders.mjs`, also a check in `scripts/release-validate.mjs --store`). Debug, test and
internal builds ignore them. Status on 2026-11-03: all open, because Apple Developer enrolment is under verification.

| Placeholder | Lives in | Where the owner gets it | Status |
|---|---|---|---|
| `REPLACE_WITH_TEAM_ID` | `apps/macos/Sources/OpenFrameUI/Store/GuardKeychain.swift`, `apps/macos/packaging/OpenFrameStudio-store.entitlements` | Apple Developer account, Membership. Never change the access group after the first release | Open |
| `REPLACE_WITH_APP_CATEGORY` | `apps/macos/packaging/Info-store.plist` | The owner picks it in App Store Connect: `public.app-category.productivity` or `public.app-category.video` | Open |
| `REPLACE_WITH_MAC_APPLICATION_IDENTITY` | CI variable `MAC_APPLICATION_IDENTITY` | The Apple Distribution certificate's name | Open |
| `REPLACE_WITH_MAC_INSTALLER_IDENTITY` | CI variable `MAC_INSTALLER_IDENTITY` | The Mac Installer Distribution certificate's name | Open |
| `REPLACE_WITH_PROVISIONING_PROFILE` | CI secret `MAC_PROVISIONING_PROFILE_B64` (base64) | Apple Developer, Profiles | Open |
| `REPLACE_WITH_ASC_API_KEY_ID`, `..._ISSUER_ID`, `..._KEY` | CI secrets `ASC_API_KEY_ID`, `ASC_API_ISSUER_ID`, `ASC_API_KEY_P8_B64` | App Store Connect, Users and Access, Integrations | Open |
| `REPLACE_WITH_APPLE_APP_ID` | No review or rate link exists in the app, so nothing uses it yet | App Store Connect, the app, App Information | Not needed yet |
| Certificates for CI | CI secrets `MAC_APPLICATION_CERT_P12`, `MAC_INSTALLER_CERT_P12` and their passwords | Apple Developer, Certificates | Open |
| Social addresses (8) | `socialLinks` in `apps/macos/Sources/OpenFrameUI/Store/LicenceRules.swift` | The owner sends the real accounts. Until then each `url` is `nil` and a release build hides the row. Each real address also goes in `scripts/app-addresses.json` | Open |
| Lifetime price | Never in code. The Mac reads `Product.displayPrice` at run time | Set in App Store Connect | Open |

The Windows values (`REPLACE_WITH_PACKAGE_IDENTITY_NAME`, the publisher and its display name) stay in
`packaging/msix/store.config.json`, which the same check scans.

## Names that must never change after the first release

`apps/macos/packaging/store-identity.json` records them and `scripts/tests/store-identity.test.mjs` fails when the code
drifts: the bundle id `app.openframestudio.studio`, both product ids, the Keychain access group and service, and the data
root name. After each upload, raise `lastUploadedBuild` in that file: `build-release-app.sh --store` refuses a build number
that did not rise. `CFBundleVersion` is the commit count, `CFBundleShortVersionString` is the workspace version in
`Cargo.toml`, which is the one version source.

## Internal testers

Internal builds were `studio.openframe.mac` and are now `app.openframestudio.studio`. Settings kept by the system under the
old name start fresh once. Data from the non-sandboxed disk image build does not carry over to a sandbox container.
