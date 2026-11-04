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

## Socials (placeholders)

- The welcome popup lists Instagram, YouTube, X, Facebook, LinkedIn, Reddit, Discord and Threads. In the code today each has handle `@straftstudio` and `url: null` (`SOCIAL_LINKS` in `rules.ts`).
- Those handles are placeholders, not real accounts. Shipping eight fake handles to a store would be wrong.
- Rule: in a release build, an entry whose `url` is `null` is not shown. If none has a URL, the welcome popup shows only the thank-you text and OK. In debug and test builds keep showing them so the mocks and tests still work.
- When the owner sends the real accounts: each `url` becomes a fixed `https://` address on that network's own site, and `isFixedSocialUrl` already whitelists those hosts. Update the welcome test and the manual (`website/src/content/manual/win/trial-and-purchase.mjs` mentions the handles).
- Add each real social address to `app-addresses.json` at that time.
