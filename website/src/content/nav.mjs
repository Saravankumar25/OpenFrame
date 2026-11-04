// Feature pages and their menu groups. Slugs are permanent URLs (/features/<slug>/).
export const FEATURE_GROUPS = [
  ['Ideate', [
    ['idea-vault', 'Idea Vault', 'Capture anything: notes, images, links, voice.'],
    ['story-board', 'Story and Beat Board', 'Acts, sequences, beats and scene cards you can drag.'],
    ['moodboards', 'Moodboards', 'Visual references that live with the film.'],
  ]],
  ['Write', [
    ['screenplay', 'Screenplay Editor', 'Proper formatting that just happens.'],
    ['drafts-revisions', 'Drafts and Revisions', 'Compare drafts. Lock the shooting script.'],
    ['import-export', 'Import and Export', 'Final Draft, Fountain, PDF, DOCX and text.'],
    ['straft-ai', 'Straft AI', 'Answers and prepares. You decide.'],
  ]],
  ['Plan', [
    ['breakdown', 'Script Breakdown', 'Tag every prop, cast member and location.'],
    ['cast-crew-locations', 'Cast, Crew and Locations', 'Everyone and everywhere, in one place.'],
    ['shot-lists-storyboards', 'Shot Lists and Storyboards', 'Plan every shot before the shoot.'],
    ['scheduling', 'Shooting Schedule', 'A stripboard that warns before problems.'],
  ]],
  ['Shoot', [
    ['call-sheets', 'Call Sheets', 'From shooting day to PDF in minutes.'],
    ['sides-daily-view', 'Sides and Daily View', 'Just the day’s pages, ready to hand out.'],
    ['reports-budget', 'Reports and Budget', 'Reports and a light budget, no accounting.'],
    ['backups-recovery', 'Projects, Backups and Recovery', 'Autosave, undo, Recently Deleted and backups.'],
  ]],
];
export const FEATURES = FEATURE_GROUPS.flatMap(([group, items]) => items.map(([slug, title, blurb]) => ({ slug, title, blurb, group })));

// "Why Straft" menu: [href, title, blurb].
export const WHY = [
  ['/local-and-private/', 'Local and private', 'Your project never has to leave your computer.'],
  ['/pricing/', 'Pay once, own forever', 'No subscription. No rental.'],
  ['/compare/', 'One app vs five', 'What you stop paying for.'],
  ['/switch/', 'Switch from other software', 'Bring your scripts. Keep your work.'],
  ['/themes/', 'Themes', 'Light, dark, built-in and your own app themes.'],
];

// Manual parts, in reading order. Manual areas (src/content/manual) name one of these.
export const PARTS = [
  ['Start', 'Getting started', 'Install, set up, create a project and find your way around.'],
  ['Ideate', 'Ideas and story', 'The Idea Vault, the Story page, the Beat Board and characters.'],
  ['Write', 'Writing', 'The screenplay editor, drafts, review and import.'],
  ['Plan', 'Production planning', 'Breakdown, catalog, people, places, shots and the schedule.'],
  ['Shoot', 'Shooting days', 'Call sheets, sides, Daily View, reports and the budget.'],
  ['Assistant', 'Straft AI', 'Set up Straft AI and use it safely.'],
  ['Project and data', 'Projects and data', 'Files, notes, backups, packages, templates and settings.'],
];

export const OS = {
  win: { key: 'win', slug: 'windows', name: 'Windows', long: 'Windows 11', mod: 'Ctrl' },
  mac: { key: 'mac', slug: 'mac', name: 'Mac', long: 'macOS on Apple silicon', mod: 'Command' },
};

export const COMPANY = 'Vighnajith Advisories Private Limited';
export const COMPANY_ADDRESS = '12/1, Ground Floor, 22nd Main Road, Bengaluru, Karnataka 560078, India';
export const SUPPORT_EMAIL = 'support@straftstudio.com';
export const CONTACT_EMAIL = 'contact@straftstudio.com';
