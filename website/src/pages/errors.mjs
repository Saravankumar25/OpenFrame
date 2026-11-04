// Error pages, in the style of large product sites: the Straft icon drawing itself, the status code, a plain
// explanation, clear ways back, and the main places on the site. Files: /404.html (served by Cloudflare Pages for any
// missing address), /403.html, /500.html and /503.html. Pages serves only 404.html by itself; the others are published
// for the host's custom error pages and for direct links. All are noindex and stay out of the sitemap.
const ERRORS = [
  { code: 403, slug: '403', title: 'Access denied', h1: 'You can’t open this page.',
    lead: 'This address is not available to you. If you followed a link from here, it may have been mistyped.',
    description: 'You do not have access to this address on the Straft Studio website. Use these links to get back to the features, tutorials and pricing.' },
  { code: 404, slug: '404', title: 'Page not found', h1: 'This page is not here.',
    lead: 'The link may be old, or the address may have a typo. The pages below will get you back on track.',
    description: 'This page does not exist on the Straft Studio website. Find the features, the step-by-step tutorials for Windows and Mac, pricing and answers instead.' },
  { code: 500, slug: '500', title: 'Something went wrong', h1: 'Something went wrong on our side.',
    lead: 'The page could not be shown. Nothing you did caused this, and your own projects are not affected: they live on your computer. Try again in a moment.',
    description: 'The Straft Studio website hit a problem showing this page. Your projects are not affected. Try again, or use the links to reach the main pages.', reload: true },
  { code: 503, slug: '503', title: 'Back soon', h1: 'We’ll be right back.',
    lead: 'The site is being updated and will be available again shortly. The apps already on your computer keep working: they do not need this website.',
    description: 'The Straft Studio website is briefly unavailable while it is updated. The apps on your computer keep working. Please try again in a moment.', reload: true },
];

export default function errors(ctx) {
  const { html, MARK } = ctx;
  return ERRORS.map((e) => ({
    path: `/${e.slug}.html`, noindex: true, nav: null, title: `${e.code}: ${e.title} | Straft Studio`, description: e.description, status: e.code,
    body: html`<section class="err"><div class="wrap center">
<div class="err-mark" aria-hidden="true">${MARK}</div>
<p class="err-code" aria-label="Error ${e.code}">${e.code}</p>
<h1>${e.h1}</h1>
<p class="lead">${e.lead}</p>
<div class="hero-cta"><a class="btn btn-primary btn-lg" href="/">Go to the home page</a>${e.reload ? html`<button type="button" class="btn btn-ghost btn-lg" data-reload>Try again</button>` : html`<a class="btn btn-ghost btn-lg" href="/tutorials/" data-os-href-win="/windows/tutorials/" data-os-href-mac="/mac/tutorials/">Browse the tutorials</a>`}</div>
<ul class="err-links" aria-label="Main pages">
<li><a href="/features/"><b>Features</b><span>Everything Straft does</span></a></li>
<li><a href="/tutorials/" data-os-href-win="/windows/tutorials/" data-os-href-mac="/mac/tutorials/"><b>Tutorials</b><span>Step by step, on real screens</span></a></li>
<li><a href="/pricing/"><b>Pricing</b><span>Pay once, own forever</span></a></li>
<li><a href="/download/"><b>Download</b><span>Windows and Mac</span></a></li>
<li><a href="/faq/"><b>Questions</b><span>Short, plain answers</span></a></li>
<li><a href="/contact/"><b>Contact</b><span>Reach the team</span></a></li>
</ul>
</div></section>`,
  }));
}
