// One place for the public address. Change OF_SITE_URL (or this default) when the custom domain arrives;
// canonical URLs, Open Graph URLs, the sitemap, robots.txt and llms.txt all derive from it at build time.
export const SITE_URL = (process.env.OF_SITE_URL || 'https://openframe-studio.pages.dev').replace(/\/+$/, '');
export const SITE_NAME = 'OpenFrame Studio';
export const PAGES_PROJECT = 'openframe-studio';
// Store listing links. Empty until the listings exist: the store buttons then render as buttons that do nothing yet.
export const STORE = { win: process.env.OF_STORE_WIN || '', mac: process.env.OF_STORE_MAC || '' };
