// The official Straft Studio profiles, in the order they are shown. Discord is a branded redirect on this site
// (the build writes the rule into _redirects, see scripts/build.mjs); it is not a profile, so it stays out of sameAs.
import { html } from '../lib/html.mjs';
import { SOC_ICON } from '../lib/icons.mjs';

export const DISCORD_INVITE = 'https://discord.gg/AuQRHDpzP4';
export const SOCIAL = [
  { key: 'youtube', name: 'YouTube', href: 'https://www.youtube.com/@StraftStudio' },
  { key: 'facebook', name: 'Facebook', href: 'https://www.facebook.com/straftstudio/' },
  { key: 'instagram', name: 'Instagram', href: 'https://www.instagram.com/straftstudio/' },
  { key: 'x', name: 'X', href: 'https://x.com/straftstudio/' },
  { key: 'threads', name: 'Threads', href: 'https://www.threads.com/@straftstudio/' },
  { key: 'linkedin', name: 'LinkedIn', href: 'https://www.linkedin.com/company/straftstudio/' },
  { key: 'reddit', name: 'Reddit', href: 'https://www.reddit.com/user/straftstudio/' },
  { key: 'discord', name: 'Discord', href: 'https://straftstudio.com/discord', invite: true },
];
export const SAME_AS = SOCIAL.filter((s) => !s.invite).map((s) => s.href);

// A row of icon links. Each link names its platform for screen readers and opens in a new tab.
export const socialList = (cls = '') => html`<ul class="soc-l ${cls}" aria-label="Straft Studio on social media">${SOCIAL.map((s) => html`<li><a href="${s.href}" target="_blank" rel="noopener noreferrer" aria-label="Straft Studio on ${s.name}" title="${s.name}">${SOC_ICON[s.key]}</a></li>`)}</ul>`;
