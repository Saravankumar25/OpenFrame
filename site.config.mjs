// One place for the public address. Change OF_SITE_URL (or this default) when the custom domain arrives;
// canonical URLs, Open Graph URLs, the sitemap, robots.txt and llms.txt all derive from it at build time.
export const SITE_URL = (process.env.OF_SITE_URL || 'https://openframe-studio.pages.dev').replace(/\/+$/, '');
export const SITE_NAME = 'OpenFrame Studio';
export const PAGES_PROJECT = 'openframe-studio';
