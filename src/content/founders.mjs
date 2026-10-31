// Founder profiles for the About page and structured data. Owner rules (2026-10-31): exact names, Saravan Kumar M K
// first, Badrinath Reddy A second, flowing paragraphs (no side headings, no bullet points), each paragraph the same
// number of characters as its counterpart so the two cards line up (tests/static.test.mjs checks it), facts from the
// project record only, and nothing about how the product is built (no technologies, internal counts or test results).
export const FOUNDERS = [
  { name: 'Saravan Kumar M K', role: 'Technical Founding Engineer', link: 'https://saravankumar25.github.io', text: [
    'Saravan Kumar M K is responsible for the end-to-end technical implementation of OpenFrame Studio. The product design described how a film should move from idea to call sheet; the work was to engineer that design into real software, screen by screen, until every workflow ran on both Windows and the Mac.',
    'Saravan built the single engine that holds every project and is shared by the Windows app and the native Mac app, so a project behaves exactly the same on both. Your work saves as you go, survives a crash, and can be undone, restored from Recently Deleted or carried away as a backup, while scripts from Final Draft, Fountain and Word come in with a preview before anything is changed for good.',
    'Saravan also built OpenFrame AI to run entirely on your own computer, and made sure it earns its place: it works without the internet, shows where every answer comes from and changes nothing until you press Apply. Every screen of both apps is tested from end to end, so what you see on this site is exactly what the app does when you use it.',
  ] },
  { name: 'Badrinath Reddy A', role: 'Product Founding Engineer', text: [
    'Badrinath Reddy A is responsible for the product idea, the ideation, the product design and the product direction of OpenFrame Studio. It all began with a simple question: why should a small film team pay for, learn and keep on copying between five separate tools when one app could hold the whole film?',
    'From that idea came the journey the product follows: ideas collected in the Idea Vault, shaped on the Beat Board, written as a screenplay, then broken down, scheduled and turned into call sheets, with each step reusing what the step before it made. Badrinath designed how each workspace is laid out and how a single scene heading flows on through the breakdown, the schedule and the call sheet.',
    'Badrinath chose who OpenFrame serves first: writer-directors, student filmmakers, short-film makers and regional-language filmmakers writing in Telugu, Hindi, Tamil, Malayalam, Kannada and English. Badrinath also set the promises it keeps: your work stays on your computer, nothing changes without your say, and you buy it once, not monthly.',
  ] },
];
