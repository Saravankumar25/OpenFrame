// Legal pages and the contact page: hub, privacy, terms, refunds, cookies and browser storage, notices, contact.
// Facts come from the app repo (what the app sends over the network) and from the site's own code (what it stores).
// Anything we do not know yet is a marked placeholder: <span class="ph" data-placeholder="key">. tests/legal.test.mjs
// greps for them and .cache/agents/legal-report.md lists every one. Visitors never see a placeholder: ph() renders an
// empty hidden marker unless the build runs with OF_SHOW_PLACEHOLDERS=1 (a review build for the owner and counsel).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';
import { COMPANY, COMPANY_ADDRESS, SUPPORT_EMAIL } from '../content/nav.mjs';

const UPDATED = '3 November 2026';
const FONTS = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..', 'assets', 'fonts');


const LINKS = {
  msPrivacy: 'https://www.microsoft.com/privacy/privacystatement',
  msRefund: 'https://support.microsoft.com/en-us/accounts-billing/subscriptions/get-a-refund-for-apps-and-games-purchased-from-microsoft-store',
  appleRefund: 'https://support.apple.com/en-us/118223',
  applePrivacy: 'https://www.apple.com/legal/privacy/en-ww/',
  appleEula: 'https://www.apple.com/legal/internet-services/itunes/dev/stdeula/',
  cfPrivacy: 'https://www.cloudflare.com/privacypolicy/',
};

export default function (ctx) {
  const { html, md } = ctx;
  const SHOW = process.env.OF_SHOW_PLACEHOLDERS === '1';

  const page = (p) => {
    if (p.title.length > 60) throw new Error(`title too long (${p.title.length}): ${p.path}`);
    const d = p.description.length;
    if (d < 110 || d > 165) throw new Error(`description length ${d}: ${p.path}`);
    return { os: null, alternates: null, nav: null, ...p };
  };
  const ph = (key, text) => (SHOW ? html`<span class="ph" data-placeholder="${key}">[${text}]</span>` : html`<span data-placeholder="${key}" hidden></span>`);
  const ext = (href, text) => html`<a href="${href}" target="_blank" rel="noopener noreferrer">${text}</a>`;
  const mail = () => html`<a href="mailto:${SUPPORT_EMAIL}">${SUPPORT_EMAIL}</a>`;
  const reach = () => html`write to ${mail()}`;
  const phero = (eyebrow, h1, lead) => html`<section class="phero"><div class="wrap"><span class="eyebrow">${eyebrow}</span><h1>${h1}</h1><p class="lead">${md(lead)}</p></div></section>`;
  const doc = (eyebrow, h1, lead, inner) => html`${phero(eyebrow, h1, lead)}
<section class="sec-s pt30"><div class="wrap legal"><p class="upd">Last updated ${UPDATED}</p>
${inner}</div></section>`;
  const crumb = (name, p) => [['Legal', '/legal/'], [name, p]];

  // ---------- Hub
  const CARDS = [
    ['/legal/privacy/', 'Privacy', 'What this website and the app do with your data, and your rights.'],
    ['/legal/terms/', 'Terms', 'The licence to use Straft Studio and the rules for using it and this site.'],
    ['/legal/refunds/', 'Refunds', 'The 7-day free trial, and how to ask the store for a refund.'],
    ['/legal/cookies/', 'Cookies and browser storage', 'Every item this site keeps in your browser, and why.'],
    ['/legal/notices/', 'Notices', 'Licences for the fonts on this site, and where the app’s own notices are.'],
  ];
  const hub = page({
    path: '/legal/',
    title: 'Legal | Straft Studio',
    description: 'Privacy, terms, refunds, cookies and browser storage, and notices for Straft Studio and this website, in plain language.',
    breadcrumbs: [['Legal', '/legal/']],
    body: html`${phero('Legal', 'Plain-language legal pages.', 'Straft Studio keeps your work on your computer and is sold only through the Microsoft Store and the Mac App Store. These pages say what that means for your data and your rights.')}
<section class="sec-s pt30"><div class="wrap"><p class="upd legal-upd">Last updated ${UPDATED}</p><h2 class="sr">The legal pages</h2><div class="grid2">
${CARDS.map(([h, t, b]) => html`<a class="card linkcard" href="${h}"><h3>${t}</h3><p>${b}</p></a>`)}
</div><p class="small-print">${COMPANY} publishes Straft Studio and this website. Questions? See <a href="/contact/">Contact</a>.</p></div></section>`,
  });

  // ---------- Privacy
  const privacy = page({
    path: '/legal/privacy/',
    title: 'Privacy | Straft Studio',
    description: 'How Straft Studio and its website treat your data: your work stays on your computer, the site sets no tracking cookies, and your rights under Indian law.',
    breadcrumbs: crumb('Privacy', '/legal/privacy/'),
    body: doc('Privacy', 'Privacy.', 'Your work stays on your computer. This page covers the app, the website, and the few times either one uses the internet.', html`
<h2>Who is responsible</h2>
<p>${COMPANY} (“we”) publishes Straft Studio and runs this website. Under India’s Digital Personal Data Protection Act, 2023, we are the data fiduciary for the personal data described on this page, which other laws call the data controller. We are a company registered in India.</p>
<p>Address: ${COMPANY_ADDRESS}. Contact: ${mail()}.</p>

<h2>The app</h2>
<ul>
<li><b>Your projects stay on your computer.</b> Scripts, notes, images and everything else you add are stored in files on your device. The app does not upload them.</li>
<li><b>No Straft account.</b> There is nothing to sign up for, so we hold no profile of you.</li>
<li><b>No telemetry, analytics or crash upload.</b> The app sends nothing about how you use it. If something goes wrong, you can create a diagnostics file yourself, read it, and decide whether to send it to anyone.</li>
<li><b>Straft AI runs on your computer.</b> What you type, and the project text it reads, are not sent to us or to any online AI service.</li>
</ul>

<h2>When the app uses the internet</h2>
<ul>
<li><b>The store.</b> The Microsoft Store or the Mac App Store installs and updates the app and handles the free trial, the payment and any refund. The store receives your purchase details under its own privacy terms. We do not receive your card or payment details.</li>
<li><b>Straft AI download, only when you start it.</b> The AI model files are not part of the installer. When you choose to download them, the app fetches them once from the public download service of the model publisher. That service sees the request and your IP address, as any website would. It receives no project content and no account. You can also import the files by hand, with no download.</li>
<li><b>Mac free trial check.</b> On the Mac, while the free trial runs, the app confirms the trial. It sends Apple’s signed purchase records for the trial (product, transaction identifiers and dates) and a one-time random number to a licence check service run for Straft. It sends nothing from your projects, and no name or email address.${ph('licence-log-retention', 'How long the licence check service keeps its technical logs, to be confirmed by the owner')}</li>
<li><b>Links you open.</b> A web link saved in your Idea Vault opens in your normal browser, under that site’s own policy.</li>
</ul>
<p>Windows and macOS themselves may send diagnostic data to Microsoft or Apple, depending on your system settings. We do not receive it and the app cannot switch it off. See the ${ext(LINKS.msPrivacy, 'Microsoft privacy statement')} and the ${ext(LINKS.applePrivacy, 'Apple privacy policy')}.</p>

<h2>This website</h2>
<ul>
<li>It sets no cookies for tracking or for remembering preferences, runs no analytics and no advertising or tracking scripts, and has no forms, accounts or email sign-up.</li>
<li>It loads no scripts, fonts or images from other companies.</li>
<li>Your choices (platform, website theme, app screen theme, price region, recent searches) stay in your own browser and never reach us. The full list is on the ${md('[cookies and browser storage page](/legal/cookies/)')}.</li>
<li>Our host may set strictly necessary security cookies to protect the site from abuse. See the cookies page.</li>
</ul>
<p>The site is hosted by Cloudflare. When you open a page, Cloudflare receives your request and handles your IP address, the page asked for, the time and your browser type, in order to deliver the page and keep the site safe. Cloudflare acts for us in doing this and may keep standard logs, under ${ext(LINKS.cfPrivacy, 'its own policy')}. Cloudflare serves pages from computers in many countries, so your request may be handled outside your own country.</p>
<p>The site uses no analytics today. If that ever changes, this page will change first and will say what is counted.</p>

<h2>What we do with this data</h2>
<p>We use it only to deliver and protect the website and to run the app’s trial check described above. We do not sell it, rent it, or use it to advertise to you.</p>

<h2>Your rights</h2>
<p>Under the Digital Personal Data Protection Act, 2023, you may ask us for a summary of the personal data we process about you and who we share it with, ask us to correct or erase it, use our grievance process below, and nominate another person to exercise these rights if you die or become unable to. Because we hold so little, a request will often get the answer that we hold nothing about you.</p>
<p>If you are in the European Union, the United Kingdom or another place with its own data protection law, you may also have rights under that law, such as access, correction, erasure, restriction, objection and portability, and the right to complain to your local data protection authority. We will honour the rights that apply to you.</p>

<h2>Grievances and contact</h2>
<p>To use a right or raise a concern, ${reach()}. ${ph('grievance-officer', 'Name and designation of the grievance officer, to be confirmed')}${ph('grievance-days', 'Reply time in days, to be confirmed by counsel')}If your concern is not resolved, you may be able to complain to the Data Protection Board of India.</p>

<h2>Children</h2>
<p>Straft Studio and this website are made for adults who write and produce films. We do not knowingly collect personal data from anyone under 18, we do not track or profile anyone, and we show no advertising. Store accounts for younger people are managed by the store and the family.</p>

<h2>Changes</h2>
<p>If the app or the site starts handling data in a new way, we will update this page and its date before that change goes live. The date at the top shows when it last changed.</p>`),
  });

  // ---------- Terms
  const terms = page({
    path: '/legal/terms/',
    title: 'Terms | Straft Studio',
    description: 'The licence to use Straft Studio, the rules for using it and this website, how your work and AI suggestions are treated, and which store terms apply.',
    breadcrumbs: crumb('Terms', '/legal/terms/'),
    body: doc('Terms', 'Terms.', 'The short version: you buy a licence once, your work is yours, and the store you bought from sets the payment terms.', html`
<h2>Who we are</h2>
<p>Straft Studio is published by ${COMPANY}. “You” means the person who installs or uses the app or visits this website.</p>
<p>Address: ${COMPANY_ADDRESS}. Contact: ${mail()}.</p>

<h2>The licence</h2>
<ul>
<li>One purchase gives you a personal, non-exclusive licence to use Straft Studio on the platform you bought it for, Windows or Mac. Buying for the other platform is a separate purchase.</li>
<li>The licence covers your own use, for personal or professional work, on the computers your store account allows.${ph('device-limit', 'How many computers one purchase covers, to be confirmed against the store rules')}</li>
<li>One purchase gives you the full app on that platform, with no subscription.${ph('lifetime-meaning', 'What “lifetime” covers: this version, future updates, future major versions. Counsel and owner to confirm')}</li>
<li>We keep all rights in the app that this licence does not give you.</li>
</ul>

<h2>Free trial</h2>
<p>Every new install starts a 7-day free trial with the whole app. After the trial, buy the app in the store to keep using it. Your project folders stay on your computer either way, and nothing is deleted. See ${md('[Refunds](/legal/refunds/)')}.</p>

<h2>Your work belongs to you</h2>
<p>You own everything you write, import or create in Straft Studio. We do not claim any right in it, and we cannot see it, because it stays on your computer. You are responsible for keeping backups. The app does not encrypt your projects; use your system’s disk encryption if you need that.</p>

<h2>Acceptable use</h2>
<p>You agree not to:</p>
<ul>
<li>copy, resell, rent or share the app or your licence with others;</li>
<li>take the app apart or change it, except where the law says you may;</li>
<li>get around the trial or the licence checks;</li>
<li>use the app to break the law or someone else’s rights, for example by using it to produce or hold content you have no right to.</li>
</ul>

<h2>Straft AI</h2>
<ul>
<li>Straft AI runs on your computer and only suggests. Nothing in your project changes until you choose to apply it.</li>
<li>Its answers can be wrong, incomplete or similar to existing material. Check them before you rely on them or publish them. You are responsible for what you keep and use.</li>
<li>The model files that Straft AI uses are made by third parties and come with their own terms of use. By downloading and using them, you agree to those terms. They are shown in the app, in Settings and in its notices.${ph('ai-model-terms', 'Exact wording and link for the model publisher’s use terms, to be supplied by the owner')}</li>
</ul>

<h2>The stores</h2>
<p>Straft Studio is sold only through the Microsoft Store and the Mac App Store. The store takes the payment, shows the final price and taxes, and handles receipts and refunds. The store’s own terms apply to your purchase. For the Mac, Apple’s ${ext(LINKS.appleEula, 'Standard Licensed Application End User License Agreement')} applies, together with this page. If this page and a store’s terms disagree about your purchase, the store’s terms win. The launch prices on this site are ₹3,999 in India, €89 in Europe and US$89 in the USA and other countries; the store shows the final amount.</p>

<h2>This website</h2>
<p>You may read this site and link to it. We describe the app as accurately as we can, but the app changes over time, so screens and features can differ from what you see here. The site is provided as it is, without a promise that it is complete or always available. Please do not copy its text or screens and present them as your own, or use the Straft Studio name in a way that suggests we endorse you. Other product names belong to their owners. Microsoft and Microsoft Store are trademarks of the Microsoft group of companies. Apple, Mac, macOS and Mac App Store are trademarks of Apple Inc.</p>

${ph('discontinuation', 'What happens if the product is withdrawn, counsel to word')}${ph('liability', 'Disclaimer of warranties and limit of liability, to be drafted by counsel, respecting consumer law')}${ph('governing-law', 'Governing law and courts, to be drafted by counsel')}

<h2>Changes and contact</h2>
<p>If we change these terms, we will update this page and its date. Questions: write to ${mail()} or see ${md('[Contact](/contact/)')}.</p>`),
  });

  // ---------- Refunds
  const refunds = page({
    path: '/legal/refunds/',
    title: 'Refunds | Straft Studio',
    description: 'Straft Studio starts with a 7-day free trial. Refunds are handled only by the Microsoft Store or the Mac App Store, and this page links to both.',
    breadcrumbs: crumb('Refunds', '/legal/refunds/'),
    body: doc('Refunds', 'Refunds.', 'Try the whole app for 7 days before you pay. If you do buy and need a refund, you ask the store.', html`
<h2>Try first</h2>
<p>Every new install starts a 7-day free trial with the whole app, and the trial needs no payment. Use it on a real project before you decide. Digital purchases are often not refundable once made, unless the store’s offer or the law says otherwise, so the trial is the safest way to be sure.</p>

<h2>How refunds work</h2>
<p>You pay the Microsoft Store or the Mac App Store, not us. We do not receive your payment, so we cannot refund it ourselves. Whether a refund is given is decided by the store, under its own rules and the law where you live. Ask in the same store you bought from:</p>
<ul>
<li><b>Windows, Microsoft Store.</b> Follow Microsoft’s guide: ${ext(LINKS.msRefund, 'Get a refund for apps and games purchased from Microsoft Store')}.</li>
<li><b>Mac, Mac App Store.</b> Follow Apple’s guide: ${ext(LINKS.appleRefund, 'Request a refund for apps or content that you bought from Apple')}. Apple notes that eligibility varies by country or region.</li>
</ul>
<p>Each purchase is for one platform, so a Windows purchase and a Mac purchase are refunded separately.</p>

<h2>If a refund is given</h2>
<p>When a store refunds a purchase, the licence for that purchase ends. Your projects stay on your computer as ordinary folders, and nothing is deleted.</p>

<h2>Still stuck</h2>
<p>If the store cannot help, tell us through ${md('[Contact](/contact/)')}. Your rights under consumer law in your country are not affected by this page.</p>`),
  });

  // ---------- Cookies and browser storage
  const KEYS = [
    ['of_os', 'localStorage', 'Windows or Mac, so the site shows the right screens and download button.', 'Until you clear it'],
    ['of_theme', 'localStorage', 'Light or dark for the website.', 'Until you clear it'],
    ['of_mock', 'localStorage', 'Light or dark for the app screens shown on the site.', 'Until you clear it'],
    ['of_region', 'localStorage', 'India, Europe or other countries, so the price shows in the right currency.', 'Until you clear it'],
    ['of_recent_search', 'localStorage', 'Your last searches on this site, to show them again.', 'Until you clear it'],
    ['of_getapp_seen', 'sessionStorage', 'That the Get Straft window has already opened by itself in this tab, so it does not open again.', 'Until you close the tab'],
    ['of_seen', 'sessionStorage', 'That the opening animation has already played in this tab.', 'Until you close the tab'],
  ];
  const cookies = page({
    path: '/legal/cookies/',
    title: 'Cookies and browser storage | Straft Studio',
    description: 'This website sets no tracking cookies. It keeps six small items in your own browser to remember your choices, and none of them is sent to anyone.',
    breadcrumbs: crumb('Cookies and browser storage', '/legal/cookies/'),
    body: doc('Cookies and browser storage', 'Cookies and browser storage.', 'No tracking cookies. A few remembered choices that stay in your browser.', html`
<h2>Cookies</h2>
<p>The site itself sets no cookies, either for tracking or for remembering your preferences. Its host, Cloudflare, may set strictly necessary security cookies to protect the site from abuse and automated traffic. They are used only for that and not for advertising or following you around the web.</p>
<div class="tbl-wrap"><table class="tbl"><thead><tr><th scope="col">Name</th><th scope="col">Set by</th><th scope="col">Purpose</th></tr></thead>
<tbody><tr><td><code>__cf_bm</code></td><td>Cloudflare</td><td>Helps tell real visitors from automated traffic. Only set if the host’s bot protection is switched on.</td></tr>
<tr><td><code>cf_clearance</code></td><td>Cloudflare</td><td>Remembers that you passed a security check, so you are not asked again straight away. Only set if such a check is shown.</td></tr></tbody></table></div>
<p>Strictly necessary security cookies do not need your consent, so there is no cookie banner. Nothing on this site asks you to accept tracking.</p>

<h2>What the site keeps in your browser</h2>
<p>To remember your choices, the site writes the items below to your browser’s own storage. They stay on your device. The site never sends them to us or to anyone else, and they are not used to follow you around the web.</p>
<div class="tbl-wrap"><table class="tbl"><thead><tr><th scope="col">Name</th><th scope="col">Where</th><th scope="col">What it remembers</th><th scope="col">How long</th></tr></thead>
<tbody>${KEYS.map(([k, w, p, d]) => html`<tr><td><code>${k}</code></td><td>${w}</td><td>${p}</td><td>${d}</td></tr>`)}</tbody></table></div>

<h2>Your control</h2>
<p>Clear this site’s data in your browser settings and every item above is gone; the site then falls back to its defaults. The site works without them. A private window keeps nothing after you close it.</p>

<h2>Our host</h2>
<p>Cloudflare hosts the site and handles your IP address to deliver pages and protect the site. That is described on the ${md('[privacy page](/legal/privacy/)')}. The site uses no analytics today. If a cookie-free, aggregated visit count is added later, this page and the privacy page will say so first.</p>`),
  });

  // ---------- Notices
  const FONT_LIST = [['Inter', 'OFL-Inter.txt', 'body text'], ['Newsreader', 'OFL-Newsreader.txt', 'page titles'], ['Sora', 'OFL-Sora.txt', 'headings']];
  const licence = (f) => fs.readFileSync(path.join(FONTS, f), 'utf8').trim();
  const notices = page({
    path: '/legal/notices/',
    title: 'Notices | Straft Studio',
    description: 'Licences for the three fonts this website uses, published in full, and where to find the notices for the Straft Studio app itself.',
    breadcrumbs: crumb('Notices', '/legal/notices/'),
    body: doc('Notices', 'Notices.', 'The licences for the work of others that this website uses.', html`
<h2>Website fonts</h2>
<p>This website uses four fonts, served from this site and not from anyone else’s. Each is licensed under the SIL Open Font License, Version 1.1. The full licence text for each font is below, and also as a plain text file.</p>
<ul>${FONT_LIST.map(([n, f, u]) => html`<li><b>${n}</b> (${u}): <a href="/assets/fonts/${f}">${f}</a></li>`)}</ul>
${FONT_LIST.map(([n, f]) => html`<details class="lic"><summary>${n}: SIL Open Font License 1.1</summary><pre class="lic-text">${licence(f)}</pre></details>`)}

<h2>The app</h2>
<p>The notices for the software and the AI model files inside Straft Studio are in the app itself, in Settings, so they always match the version you have installed.</p>`),
  });

  // ---------- Contact
  const contact = page({
    path: '/contact/',
    title: 'Contact | Straft Studio',
    description: 'How to reach the team behind Straft Studio, and where to find answers first: the step-by-step tutorials, the features and the FAQ.',
    breadcrumbs: [['Contact', '/contact/']],
    body: html`
${phero('Contact us', 'Questions? Start here.', 'Most answers are already in the tutorials and the questions and answers.')}
<section class="sec-s pt30"><div class="wrap"><div class="contact">
<div><h2 class="sr">Where to look first</h2><div class="grid2">
<a class="card linkcard" href="/tutorials/" data-os-href-win="/windows/tutorials/" data-os-href-mac="/mac/tutorials/"><h3>Tutorials</h3><p>Step-by-step tasks on the real screens.</p></a>
<a class="card linkcard" href="/features/"><h3>Features</h3><p>Every capability, for Windows and Mac.</p></a>
<a class="card linkcard" href="/faq/"><h3>Questions and answers</h3><p>Platforms, privacy, files and the trial.</p></a>
<a class="card linkcard" href="/pricing/"><h3>Pricing</h3><p>One payment after a 7-day trial.</p></a>
</div></div>
<div class="cinfo"><h3>Write to us</h3><p>${mail()}</p><h3>Purchases and refunds</h3><p>The Microsoft Store and the Mac App Store handle purchases, receipts and refunds. Use ${ext(LINKS.msRefund, 'Microsoft’s refund help')} for Windows and ${ext(LINKS.appleRefund, 'Apple’s refund help')} for the Mac. More on our <a href="/legal/refunds/">refunds page</a>.</p><h3>Privacy</h3><p>Your rights and how to use them are on the <a href="/legal/privacy/">privacy page</a>.</p><h3>This website</h3><p>The site has no contact form and collects nothing.</p></div>
</div></div></section>`,
  });

  return [hub, privacy, terms, refunds, cookies, notices, contact];
}
