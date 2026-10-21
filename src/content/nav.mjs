// Feature pages and their menu groups. Slugs are permanent URLs (/features/<slug>/).
export const FEATURE_GROUPS = [
  ['Ideate', [
    ['idea-vault', 'Idea Vault', 'Notes, images, links and voice memos in one place.'],
    ['story-board', 'Story and Beat Board', 'Acts, sequences, beats, scene cards and characters.'],
    ['moodboards', 'Moodboards', 'Visual references that live with the film.'],
  ]],
  ['Write', [
    ['screenplay', 'Screenplay Editor', 'Industry formatting while you type.'],
    ['drafts-revisions', 'Drafts and Revisions', 'Compare drafts, lock pages, track revisions.'],
    ['import-export', 'Import and Export', 'Final Draft, Fountain, Word, PDF and text.'],
    ['ai-assistant', 'Offline Assistant', 'Optional, local, and it only suggests.'],
  ]],
  ['Plan', [
    ['breakdown', 'Script Breakdown', 'Tag cast, props, wardrobe and more per scene.'],
    ['cast-crew-locations', 'Cast, Crew and Locations', 'People and places, linked to scenes.'],
    ['shot-lists-storyboards', 'Shot Lists and Storyboards', 'Plan every shot before the shoot.'],
    ['scheduling', 'Shooting Schedule', 'A stripboard that warns before problems.'],
  ]],
  ['Shoot', [
    ['call-sheets', 'Call Sheets', 'From shooting day to PDF.'],
    ['sides-daily-view', 'Sides and Daily View', 'Just the day’s pages, ready to hand out.'],
    ['reports-budget', 'Reports and Budget', 'Production reports and a simple budget.'],
    ['backups-recovery', 'Backups and Recovery', 'Autosave, undo, Recently Deleted and backups.'],
  ]],
];
export const FEATURES = FEATURE_GROUPS.flatMap(([group, items]) => items.map(([slug, title, blurb]) => ({ slug, title, blurb, group })));

// Tutorial learning paths, in reading order.
export const TUTORIAL_PATHS = [
  ['getting-started', 'Getting started', 'Install, set up and find your way around.'],
  ['writing', 'Writing and story', 'From the first idea to a formatted screenplay.'],
  ['visual', 'Visual planning', 'Moodboards, shot lists and storyboards.'],
  ['prep', 'Production preparation', 'Breakdown, people, places and the schedule.'],
  ['shoot', 'Production days', 'Call sheets, sides, reports and budget.'],
  ['ai', 'Offline assistant', 'Set up the optional assistant and use it safely.'],
  ['data', 'Projects and data', 'Backups, recovery, files and settings.'],
];

export const OS = {
  win: { key: 'win', slug: 'windows', name: 'Windows', long: 'Windows 11', mod: 'Ctrl' },
  mac: { key: 'mac', slug: 'mac', name: 'macOS', long: 'macOS 14 or later on Apple silicon', mod: 'Cmd' },
};
