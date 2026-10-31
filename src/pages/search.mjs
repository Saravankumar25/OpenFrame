// /search/: full results with filters. Everything after the markup is in src/assets/js/search.js (the state lives in the URL:
// ?q=&os=&area=&type=&part=). The page is not indexed; it is only the full view of the search in the header.
const SEARCH = '<svg viewBox="0 0 24 24" aria-hidden="true" focusable="false" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round"><circle cx="11" cy="11" r="7"/><path d="M20 20l-3.5-3.5"/></svg>';

export default function search({ html, raw }) {
  const chip = (f, v, label) => html`<button type="button" data-f="${f}" data-v="${v}" aria-pressed="false">${label}</button>`;
  return [{
    path: '/search/', nav: 'search', noindex: true,
    title: 'Search | OpenFrame Studio',
    description: 'Search every OpenFrame Studio tutorial, feature and answer, and narrow the results by platform, part, area and type.',
    body: html`<div class="sp" id="sp" role="search"><section class="phero"><div class="wrap"><span class="eyebrow">Search</span><h1>Find the help you need.</h1>
<p class="lead">Tutorials, features and answers in one place. A mistyped word still finds what you meant.</p>
<div class="sp-bar">${raw(SEARCH)}<input class="sp-q" id="sp-q" type="search" autocomplete="off" autocapitalize="off" spellcheck="false" enterkeyhint="search" aria-label="Search" placeholder="Search tutorials, features and answers"></div>
<p class="sp-tips">Put a phrase in quotes to match it exactly. Put a minus sign before a word to leave it out.</p></div></section>
<section class="sec-s pt30"><div class="wrap">
<noscript><p class="sp-note">Search needs JavaScript. You can still browse the <a href="/windows/tutorials/">Windows tutorials</a> and the <a href="/mac/tutorials/">Mac tutorials</a>.</p></noscript>
<div class="sp-filters">
<div class="sp-f"><span class="sp-fl" id="spl-os">Platform</span><span class="seg" role="group" aria-labelledby="spl-os">${chip('os', 'cur', html`Current<span data-os-show="win"> (Windows)</span><span data-os-show="mac"> (Mac)</span>`)}${chip('os', 'win', 'Windows')}${chip('os', 'mac', 'Mac')}${chip('os', 'all', 'Both')}</span></div>
<div class="sp-f"><span class="sp-fl" id="spl-type">Type</span><span class="seg" role="group" aria-labelledby="spl-type">${chip('type', '', 'All')}${chip('type', 'tutorial', 'Tutorials')}${chip('type', 'feature', 'Features')}${chip('type', 'answer', 'Answers')}</span></div>
<div class="sp-f"><label class="sp-fl" for="sp-part">Part</label><select id="sp-part"><option value="">All parts</option></select></div>
<div class="sp-f"><label class="sp-fl" for="sp-area">Area</label><select id="sp-area"><option value="">All areas</option></select></div>
<button type="button" class="sp-reset" id="sp-reset" hidden>Clear filters</button>
</div>
<div class="sp-out" id="sp-out"></div>
<div class="sr" role="status" aria-live="polite" id="sp-live"></div>
</div></section></div>`,
  }];
}
