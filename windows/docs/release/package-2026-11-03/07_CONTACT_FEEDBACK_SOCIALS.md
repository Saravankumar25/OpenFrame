# 07. Contact, feedback and socials

## Decision

People reach the team through the website. The app has **no form, no mail composer, no data sent**. One link, `Contact us`, opens `https://straftstudio.com/contact/` in the default browser. That page already exists in the website source (`website/src/pages/legal.mjs`, the `/contact/` page) and already says the site has no contact form and collects nothing, so the privacy page stays true.

## App side (both sessions)

1. Add a single constant for the address (one place per shell): Windows `apps/desktop/src/features/licensing/` or a shared constants file; Mac next to the licence rules. Value `https://straftstudio.com/contact/`.
2. Show `Contact us` in these places:
   - About or Settings (find the existing About screen and add the row there).
   - The lock screen (trial ended, refunded, clock changed): `Need help? Contact us`.
   - The check screen (`Checking your purchase`): same link.
   - Mac first-launch screen: same link.
   - Mac: also a menu item `Help > Contact Straft Support`. Windows: no Help menu exists (it was removed from the design on purpose); do not add one.
3. Open it with the existing link-opening path (`of_open_url`, which uses `straft_security::web_link` and accepts only http(s); on the Mac the system URL opener). Opening a link is a shell action, so it works while the core has locked every project operation.
4. Add the address to `scripts/app-addresses.json` with a reason ("opened in the browser when the person chooses Contact us; the app sends nothing"), so the governance test accepts it. Do not add any other address.
5. Do not whitelist it in `isFixedSocialUrl`; that list is only for the welcome popup's social links.
6. Tests: the link appears on the screens above; clicking calls the opener with exactly this address; no network call is made by the app.

## Website side (separate prompt; listed here so the app and site agree)

- The contact page shows `support@straftstudio.com` (set `OF_CONTACT_EMAIL` for the build so the page stops saying the address is "being set up"), the company name and address, and where the refund help lives.
- Keep "no form, collects nothing" true. If a form is ever added later, the privacy page must change first.

## Socials

Superseded 2026-11-05: the owner approved the eight real addresses. They live in `apps/desktop/src/features/licensing/communityLinks.ts` and the welcome popup ("Welcome to the Straft Studio community") shows all eight in every build.

- Instagram, YouTube, X, Facebook, LinkedIn, Reddit, Threads: the network's own page for Straft Studio.
- Discord: `https://straftstudio.com/discord`, a forwarding page on the website. The app never holds the invite code.
- `isFixedSocialUrl` accepts only https and only the hosts of these eight addresses. The app fetches nothing; an address opens only when the person clicks it.
- Still to do: add each address to `app-addresses.json`, and update the manual (`website/src/content/manual/win/trial-and-purchase.mjs`) if it mentions the handles.
