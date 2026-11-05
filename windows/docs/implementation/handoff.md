# Handoff: where we are right now

> Rewritten (not appended) at the end of every session. One page. Last rewritten: 2026-11-05, after the sweep, the audit ports,
> the team password removal and the Windows push.

## State

1. **GitHub and local hold the same Windows code and the same website.** `windows/` and `website/` on `main` equal the local
   trees. The local checkout has the app at its root; `StraftWebsite` is its own repository on branch `website/straft-main`.
2. **Installed on the owner's laptop:** `release-out/Straft-4.exe`, built from a clean tree at f701f926. Later commits touch only
   WindowsHTML and documents.
3. **No team password any more** ([ADR-0043](../adr/ADR-0043-team-password-removed.md)). The internal build opens straight to the app.
4. **Fixed this session:** real scene numbers on Schedule, Breakdown, shots and call sheets; Home counts what is current; the
   nearest hospital reaches call sheets; Scene Cards link to any scene; a migration in other line endings is no longer reported as
   edited. The security, licence and accessibility fixes from the 2026-10-31 audit branches are ported; bypass 3 of the store audit
   has no caller in this tree. The welcome popup reads "Welcome to the Straft Studio community" with the eight official links.
5. **WindowsHTML:** 821 screens, 1,658 files, from the final app. 9 of 1,642 pages are 0.54 to 0.71 percent over the diff limit only
   because the checkout path on them is replaced on purpose.
6. **Website:** SEO and GEO pass, IndexNow by change, a live monitor after each deploy, the eight social links, `/discord` 302 to
   the invite. All live. Evidence and the owner's steps: `website/docs/SEO-CHECKLIST.md` and `SEARCH-APPEARANCE.md`.
7. **Test evidence:** see the top entry of the [progress log](progress-log.md). The final build had a targeted run by the owner's
   word, not a full one.

## Next items, in order

1. **Fresh website session (started from this one):** publish the new Windows and Mac screens to straftstudio.com, every screen seen
   in light and dark.
2. Owner: first look at the app on real data, including opening Ananthapuram, which still has the old manifest and takes the new name
   when opened. The old OpenFrame Studio install is still present beside it; uninstalling it is the owner's choice.
3. Owner: Search Console steps, and the Cloudflare Web Analytics switch (the beacon the CSP blocks is the one console error).
4. Mac session: bring `StraftMac/` in line. It has no edit lock and does not report refunds, per the audit check.
5. Straft AI still resolves "scene N" by position. Changing it needs a measured evaluation first (ADR-0028).

## Open blockers

- All store values are placeholders; a Store release fails until they are filled.
- The site says "on sale" in places while store listings are not live: `src/lib/layout.mjs` line 127, `src/pages/info.mjs` line 89,
  `src/content/copy.mjs` line 81. Owner's wording call.
- Spec 05's outside connection on port 443: seen on 2026-11-04 from the WebView2 browser process to a 2603:1046 address, not seen in any
  later run, and the holder of the refused socket in the failed run was not recorded. The spec now records role and host. Not proven.

## Do not

- Do not push without the owner's word. Never `git pull` origin/main here: sync `windows/` and `website/` by the subtree commit method.
- Do not build an installer or a release executable without the owner's word.
- Do not type or move focus on this computer during E2E: a stray `s` failed spec 04 on 2026-11-05.
- Do not delete the four `claude/audit-*` branches or `website/local-checkpoint-2026-11-05`: the first are the source of the ports,
  the second the only copy of old local website work.
- Do not edit old-name spellings marked `LEGACY:`. Old data needs them.
