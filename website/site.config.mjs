// One place for the public address (the official custom domain). Override with OF_SITE_URL.
// canonical URLs, Open Graph URLs, the sitemap, robots.txt and llms.txt all derive from it at build time.
export const SITE_URL = (process.env.OF_SITE_URL || 'https://straftstudio.com').replace(/\/+$/, '');
export const SITE_NAME = 'Straft Studio';
// The Cloudflare Pages project still has this name; renaming it is a manual owner action, and changing only this text breaks the deploy.
export const PAGES_PROJECT = 'openframe-studio';
// Store listing links. Empty until the listings exist: the store buttons then render as buttons that do nothing yet.
export const STORE = { win: process.env.OF_STORE_WIN || '', mac: process.env.OF_STORE_MAC || '' };
