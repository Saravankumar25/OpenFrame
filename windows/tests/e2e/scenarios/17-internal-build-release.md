# The internal build as it ships

Spec: `tests/e2e/specs/17-internal-build-release.test.mjs`

Tags: `@e2e-data-root: own`

Before: an empty data folder of its own. The app starts for the first time. No test switch is used, so this runs on the release executable; specs 15 and 16 cover the store licence states on a debug build until a packaged store build exists (Chapter 3).

## No trial chip, no Buy and no popup, and About says internal build, not for sale

1. Do the first launch: name 'E2E Tester', project title 'Quiet Pier'. Wait about 1.5 seconds.
2. Check: no free trial chip, no "Buy full version" button and no dialog are shown.
3. Ask the core for the app version. Check: it looks like 1.2.3.
4. Open the project menu and choose Project settings.
5. Check: the About card shows "Version <that version>" and "Internal build, not for sale.".
6. Check: no price appears anywhere on the screen, and the screen is quiet.

## Platform notes

- Mac: the internal disk image build shows the same text. To confirm in phase M-track.
