// One place for the public address. Canonical URLs, Open Graph URLs, the sitemap, robots.txt and llms.txt all derive from it
// at build time. Override with OF_SITE_URL. Moving the site later means changing this one line (docs/OPERATIONS.md section 8).
export const SITE_URL = (process.env.OF_SITE_URL || 'https://straftstudio.com').replace(/\/+$/, '');
export const SITE_NAME = 'Straft Studio';
export const PAGES_PROJECT = 'straft-studio';
// Store listing links. Empty until the listings exist: the store buttons then render as buttons that do nothing yet.
export const STORE = { win: process.env.OF_STORE_WIN || '', mac: process.env.OF_STORE_MAC || '' };
