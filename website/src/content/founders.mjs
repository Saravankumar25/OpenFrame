// Founder profiles for the About page and structured data. Owner rules: exact names, Saravan Kumar M K first, Badrinath
// Reddy A second, each credited with their team (2026-11-04), flowing paragraphs (no side headings, no bullet points), each
// paragraph the same number of characters as its counterpart so the two cards line up (tests/static.test.mjs checks it),
// facts from the project record only, and nothing about how the product is built (no technologies, internal counts or test
// results).
export const FOUNDERS = [
  { name: 'Saravan Kumar M K', role: 'Technical Co-Founding Engineer', link: 'https://saravankumar25.github.io', text: [
    'Saravan Kumar M K led the technical development of Straft Studio with the engineering team, turning the product design into working software, screen by screen, on both Windows and on the Mac.',
    'Together, the team built the single engine shared by both apps, the saving and recovery that protect your work, and Straft AI, which runs on your own computer and changes nothing until you press Apply.',
  ] },
  { name: 'Badrinath Reddy A', role: 'Product Co-Founding Engineer', text: [
    'Badrinath Reddy A led the product development of Straft Studio with the product team, shaping the idea, the design and the direction, and how a film moves from first idea to final call sheet.',
    'Together, the team chose who Straft Studio serves first, independent and regional-language filmmakers, and set the promises it keeps: your work stays on your computer, and you buy it once, not monthly.',
  ] },
];
