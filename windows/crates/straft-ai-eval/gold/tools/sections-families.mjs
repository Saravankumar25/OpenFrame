// Pairs, triples, conversational cases, multi-turn follow-ups and out-of-section negatives.
import * as D from './sections-data.mjs';
import { C, P } from './sections-core.mjs';
import { LEAVES, leafIds } from './sections-data.mjs';

const { num, each, money, dayAlt, charScenes, charDays, castOf, crewOf, scenes, bd } = D;

const dt = (d) => [D.days[d - 1].date, `${d} March`, `March ${d}`];
const iso = (s) => [s, `${+s.slice(8)} ${['', 'January', 'February', 'March'][+s.slice(5, 7)]}`, `${['', 'January', 'February', 'March'][+s.slice(5, 7)]} ${+s.slice(8)}`];
const labelOf = Object.fromEntries(LEAVES);

const SC = 'screenplay/scenes';
const BD = 'breakdown/breakdown';
const CR = 'production/cast-crew';
const CH = 'story/characters';
const BB = 'story/beat-board';
const OL = 'story/outline';
const SH = 'production/schedule';
const LO = 'production/locations';
const BU = 'production/budget';
const CS = 'call-sheets/call-sheets';
const CA = 'production/catalog';
const SL = 'production/shot-lists';
const SB = 'production/storyboards';
const DO = 'production/day-out-of-days';
const DV = 'production/daily-view';
const SI = 'production/sides';
const sc = (n) => ({ scene: n });
const dy = (n) => ({ day: n });

// ------------------------------------------------------------------ the 15 most likely pairs
export function pairs() {
  const out = [];
  const pair = (a, b, q, t, spec = {}) => out.push(C({ fam: 'pair', picks: [a, b], q, t, ...spec }));
  const sudheeshScenes = charScenes('Sudheesh').length;

  // P1 Screenplay + Cast & Crew (the example of the spec)
  pair(P(SC), P(CR), 'How many scenes does Sudheesh play in, and describe his character?', 'ds',
    { has: [num(sudheeshScenes)], f: ['Sanoj Kurup', 'Anjali'], keep: true });
  pair(P(SC), P(CR), 'How many scenes is Young Anjali in, and what limits does her actor have?', 'ls',
    { has: [num(charScenes('Young Anjali').length)], f: ['guardian'] });
  pair(P(SC), P(CR), 'Who plays the Harbour Clerk and which scenes is he in?', 'ls',
    { f: ['Prasad Venugopalan'], has: each(charScenes('Harbour Clerk')) });
  // P2 Screenplay + Characters
  pair(P(SC, sc(41)), P(CH), 'Who is in scene 41 and what is each person\'s role?', 'ls',
    { f: ['Anjali', 'Sudheesh', 'Nandan', 'Supporting'] });
  pair(P(SC), P(CH), 'How many scenes is Babu in, and how is he connected to Mariamma?', 'rl',
    { has: [num(charScenes('Babu').length)], f: ['rival'] });
  pair(P(SC), P(CH), 'Which scenes feature the rival of Mariamma?', 'ls',
    { f: ['Babu'], has: each(charScenes('Babu')) });
  // P3 Beat Board + Screenplay
  pair(P(BB), P(SC), 'Explain the second act using the scenes.', 'se',
    { f: ['Babu', 'Salim', 'Mariamma', 'Jose'] });
  pair(P(BB), P(SC), 'Which scene is linked to the card where Babu cuts the power, and who is in it?', 'ls',
    { f: ['Dr. Leela', 'Rosily', 'Mariamma', 'Jose'], has: [['40']] });
  pair(P(BB), P(SC), 'Which scene belongs to the card where the licence is stamped, and what happens in it?', 'ex',
    { f: ['Anjali', 'Mariamma', 'Salim'], has: [['45']] });
  // P4 Screenplay + Breakdown
  pair(P(SC, sc(3)), P(BD, sc(3)), 'What happens in scene 3 and what props does it need?', 'ex',
    { f: ['Anjali', 'Suitcase'] });
  pair(P(SC, sc(17)), P(BD, sc(17)), 'Who is in scene 17 and who are the extras?', 'ls',
    { f: ['Babu', 'Mariamma', 'Jose', 'Harbour Clerk', 'Fish buyers'] });
  pair(P(SC, sc(33)), P(BD, sc(33)), 'Who is in scene 33 and what hair does each person have?', 'ls',
    { f: ['Young Anjali', 'Mariamma', 'two braids'] });
  // P5 Screenplay + Schedule
  pair(P(SC), P(SH), 'Which day is scene 12 shot, and what happens in it?', 'ex',
    { f: ['transmitter'], has: [dayAlt(5)] });
  pair(P(SC), P(SH, dy(7)), 'What is shot on Day 7 and who is in those scenes?', 'ls',
    { f: ['Anjali', 'Sudheesh', 'Nandan'], has: each([19, 41]) });
  pair(P(SC), P(SH), 'Which day is the scene where Jose confesses shot, and who else is in it?', 'ls',
    { f: ['Jose', 'Anjali'], has: [dayAlt(1)] });
  // P6 Schedule + Locations
  pair(P(SH), P(LO), 'Where are scenes 9 and 3 shot, and what is the status of those locations?', 'ls',
    { f: ['Babu\'s Boat Yard', 'Shortlisted', 'Backwater Ferry Jetty', 'Idea'] });
  pair(P(SH), P(LO), 'Which shoot days use a location that is only an idea?', 'ls', { has: each([12, 13]) });
  pair(P(SH, dy(14)), P(LO), 'On Day 14, where do we shoot and is the location confirmed?', 'ls',
    { f: ['Little Flower', 'Offshore Waters', 'Shortlisted'] });
  // P7 Schedule + Cast & Crew
  pair(P(SH), P(CR), 'On which days is Varghese scheduled while his actor is unavailable?', 'ls',
    { f: ['Abraham Kallarakkal'], has: each([13, 14]) });
  pair(P(SH), P(CR), 'Who plays the Harbour Clerk, and on which day is he needed at two locations?', 'ls',
    { f: ['Prasad Venugopalan'], has: [dayAlt(12)] });
  pair(P(SH), P(CR), 'Which actor is unavailable on the day scene 42 is shot?', 'lk', { f: ['Beena Chacko'] });
  // P8 Schedule + Call Sheets
  pair(P(SH), P(CS), 'Which shoot days have a final call sheet?', 'ls', { has: [dayAlt(1), dayAlt(2), dayAlt(3)] });
  pair(P(SH, dy(6)), P(CS, dy(6)), 'What is the crew call on Day 6 and which scenes are shot that night?', 'ls',
    { f: ['16:00'], has: each([26, 31, 34]) });
  pair(P(SH), P(CS), 'Is there a call sheet for the day scene 35 is shot?', 'lk',
    { has: [['no call sheet', 'not yet', 'does not have', 'has no']] });
  // P9 Budget + Locations
  pair(P(BU), P(LO), 'What is planned for location fees, and what fee does Babu\'s Boat Yard want?', 'ls',
    { has: [money(900000), ['25,000', '25000']] });
  pair(P(BU), P(LO), 'How many locations are not confirmed, and what was spent on location fees and permits?', 'ls',
    { has: [num(6), money(765000)] });
  pair(P(BU), P(LO), 'Which location has a permit still pending, and what is planned for location fees and permits?', 'ls',
    { f: ['Offshore Waters'], has: [money(900000)] });
  // P10 Cast & Crew + Characters
  pair(P(CR), P(CH), 'Who plays Mariamma and how is she related to Anjali?', 'rl', { f: ['Thresiamma Joseph', 'aunt'] });
  pair(P(CR), P(CH), 'Who is the antagonist and who plays him?', 'lk', { f: ['Babu', 'Suresh Pallath'] });
  pair(P(CR), P(CH), 'Which character has no actor, and what is the role?', 'lk', { f: ['Jose', 'Supporting'] });
  // P11 Breakdown + Catalog
  pair(P(BD, sc(3)), P(CA), 'Which props are used in scene 3, and in how many scenes is each used?', 'ls',
    { f: ['Suitcase', 'Leather case of cassette tapes'], has: [num(2)] });
  pair(P(BD), P(CA), 'How many prop elements are in the breakdown and how many different props are in the catalog?', 'ls',
    { has: [num(97), num(73)] });
  pair(P(BD), P(CA), 'Which scenes use the prop that appears in the most scenes?', 'ls',
    { f: ['Steel tea glasses'], has: each([2, 10, 20, 28, 44]) });
  // P12 Shot Lists + Storyboards
  pair(P(SL, sc(41)), P(SB, sc(41)), 'How many shots and storyboard panels does scene 41 have?', 'ls', { has: [num(3), num(4)] });
  pair(P(SL, sc(41)), P(SB, sc(41)), 'Which lens is the close on Anjali, and which panel shows her at the microphone?', 'ls',
    { f: ['85mm', 'microphone'] });
  pair(P(SL, sc(41)), P(SB, sc(41)), 'Describe the plan for scene 41 in shots and panels.', 'br',
    { f: ['crane', 'microphone', 'lamp'] });
  // P13 Locations + Breakdown
  pair(P(LO), P(BD), 'Which scenes are at Kadalpuram Beach and what is the permit situation there?', 'ls',
    { f: ['permit'], has: each([15, 27, 33, 40, 45]) });
  pair(P(LO), P(BD, sc(35)), 'What locations does scene 35 use and what is their status?', 'ls',
    { f: ['Little Flower Fishing Boat', 'Offshore Waters'] });
  pair(P(LO), P(BD), 'What does the roof location require, and which scenes use it?', 'ls',
    { f: ['harness'], has: each([19, 41]) });
  // P14 Day Out of Days + Cast & Crew
  pair(P(DO), P(CR), 'On which days does Salim work, and who plays him?', 'ls',
    { f: ['Nizar Hameed'], has: each(charDays('Salim')) });
  pair(P(DO), P(CR), 'Does Adithyan Raj work on a day he is unavailable?', 'lk', { f: ['Adithyan Raj'], has: [dayAlt(8)] });
  pair(P(DO), P(CR), 'How many days does the actor who plays Mariamma work?', 'ct',
    { f: ['Thresiamma Joseph'], has: [num(charDays('Mariamma').length)] });
  // P15 Daily View + Sides
  pair(P(DV, dy(7)), P(SI, dy(7)), 'What is shot on Day 7 and which pages are in the sides?', 'ls', { has: each([19, 41]) });
  pair(P(DV, dy(12)), P(SI, dy(12)), 'For Day 12, which scenes are in the sides and which props are needed?', 'ls',
    { f: ['Suitcase', 'Substation key'], has: each([9, 29, 3, 30]) });
  pair(P(DV, dy(4)), P(SI, dy(4)), 'Who speaks in the Day 4 sides and what does the day note say?', 'ls',
    { f: ['Anjali', 'Nandan', 'Salim', 'Exterior station day'] });
  return out;
}

// ------------------------------------------------------------------ the five triples
export function triples() {
  const out = [];
  const tri = (picks, q, t, spec) => out.push(C({ fam: 'triple', picks, q, t, ...spec }));
  tri([P(SC, sc(41)), P(BD, sc(41)), P(SH, dy(7))], 'For scene 41, what happens, what props does it need and which day is it shot?', 'ex',
    { f: ['Anjali', 'Portable generator'], has: [dayAlt(7)] });
  tri([P(CH), P(CR), P(SC)], 'Tell me about Babu: his role, who plays him and how many scenes he is in.', 'ds',
    { f: ['Antagonist', 'Suresh Pallath'], has: [num(charScenes('Babu').length)] });
  tri([P(SH, dy(8)), P(LO), P(CR)], 'On Shoot Day 8, where do we shoot, is the location confirmed and which actors are unavailable?', 'ls',
    { f: ['Kadalpuram Beach', 'Confirmed', 'Aadhya Varma', 'Adithyan Raj'] });
  tri([P(BU), P(SH), P(CS)], 'What was planned for Lead cast, how many shoot days are there and how many call sheets are final?', 'ls',
    { has: [money(4000000), num(14), num(3)] });
  tri([P(BB), P(SC), P(CH)], 'Explain the story: who drives each act and how are they related?', 'se',
    { f: ['Anjali', 'Babu', 'Mariamma', 'Varghese'] });
  return out;
}

// ------------------------------------------------------------------ conversational answers
export function conversational() {
  const out = [];
  const conv = (picks, q, t, spec) => out.push(C({ fam: 'conv', picks, q, t, ...spec }));
  // scene brief
  conv([P(SC, sc(41))], 'Give me a briefing on scene 41: who is there and what happens.', 'br', { f: ['Anjali', 'Sudheesh', 'Nandan', 'lamps'] });
  conv([P(SC, sc(35))], 'Brief me on scene 35.', 'br', { f: ['Varghese', 'Jose', 'Little Flower', 'engine'] });
  conv([P(SC, sc(32))], 'Walk me through scene 32.', 'br', { f: ['Mariamma', 'Anjali', 'net', 'storm'] });
  conv([P(SC, sc(12)), P(BD, sc(12))], 'Brief me on scene 12 including what it needs.', 'br', { f: ['Sudheesh', 'Anjali', 'transmitter', 'Vacuum tubes'] });
  conv([P(SC, sc(45))], 'What is scene 45 about? Give me a proper picture of it.', 'br', { f: ['Anjali', 'Mariamma', 'Salim', 'low tide'] });
  conv([P(SC, sc(8))], 'Brief me on scene 8.', 'br', { f: ['Harbour Clerk', 'Babu', 'lease', 'cheque'] });
  // character description
  conv([P(CH)], 'Describe Babu as a character.', 'ds', { f: ['Antagonist', 'Mariamma', 'land'] });
  conv([P(CH)], 'Tell me about Mariamma.', 'ds', { f: ['Varghese', 'Anjali', 'Babu', 'Rosily'] });
  conv([P(SC), P(CR)], 'Describe Salim.', 'ds', { f: ['Nizar Hameed', 'Nandan', 'ham'] });
  conv([P(CH), P(SC)], 'Describe Jose and what he does in the film.', 'ds', { f: ['Varghese', 'Little Flower', 'net'] });
  conv([P(CH)], 'Who is Young Anjali? Describe her.', 'ds', { f: ['Anjali', '1999', 'Mariamma'] });
  conv([P(SC), P(CR)], 'Describe Nandan.', 'ds', { f: ['Adithyan Raj', 'Salim'] });
  // relationship
  conv([P(CH)], 'How are Anjali, Mariamma and Varghese connected?', 'rl2', { f: ['father', 'aunt', 'brother'] });
  conv([P(CH)], 'What is the link between Babu, Mariamma and the Harbour Clerk?', 'rl2', { f: ['rival', 'works for'] });
  conv([P(CH)], 'How are Nandan and Salim connected?', 'rl2', { f: ['listener', 'student'] });
  conv([P(CH)], 'How are Jose and Varghese related, and what happened to them in 1999?', 'rl2', { f: ['crewmate', 'Little Flower'] });
  conv([P(CH)], 'How are Anjali and Sudheesh related?', 'rl2', { f: ['friend', 'engineer'] });
  conv([P(CH)], 'How is Young Anjali related to Anjali and Mariamma?', 'rl2', { f: ['younger self', 'Mariamma'] });
  // story explanation
  conv([P(BB), P(SC)], 'Explain what happens in the second act.', 'se', { f: ['Babu', 'Salim', 'Mariamma', 'Jose'] });
  conv([P(BB), P(SC)], 'Explain Act One.', 'se', { f: ['Anjali', 'Fr. Thomas', 'transmitter'] });
  conv([P(BB)], 'Explain the whole story act by act.', 'se', { f: ['The Return', 'The Silence of 1999', 'The Signal'] });
  conv([P(SC)], 'Why does Mariamma not tell Anjali how Varghese died? Explain.', 'se', { f: ['Mariamma', 'net', 'storm'] });
  conv([P(SC)], 'Who cut the cable and why? Explain what the scenes show.', 'se', { f: ['cable', 'Babu'] });
  conv([P(BB), P(SC)], 'Explain Act Three.', 'se', { f: ['batteries', 'roof', 'lamps', 'licence'] });
  return out;
}

// ------------------------------------------------------------------ multi-turn follow-ups with the kept selection
export function turns() {
  const out = [];
  /** A conversation: every turn is a case whose `before` holds the turns that came first. */
  const seq = (picks, list) => {
    const asked = [];
    for (const [q, t, spec] of list) {
      out.push(C({ fam: 'turn', picks, q, t, before: [...asked], ...spec }));
      asked.push(q);
    }
  };
  seq([P(SC), P(CR)], [
    ['How many scenes does Nandan play in?', 'ct', { has: [num(charScenes('Nandan').length)] }],
    ['and who plays him?', 'lk', { f: ['Adithyan Raj'] }],
    ['is he free for the whole shoot?', 'lk', { f: ['School exams'] }],
  ]);
  seq([P(SH)], [
    ['Which day is scene 3 shot?', 'lk', { has: [dayAlt(12)] }],
    ['what else is shot that day?', 'ls', { has: each([9, 29, 30]) }],
    ['any warnings that day?', 'ls', { f: ['Anjali', 'Harbour Clerk'] }],
  ]);
  seq([P(BU)], [
    ['What is the actual total?', 'lk', { has: [money(13887000)] }],
    ['and the planned?', 'lk', { has: [money(16460000)] }],
    ['so what is the difference?', 'lk', { has: [money(223000)] }],
  ]);
  seq([P(CH)], [
    ['Who is Mariamma?', 'ex', { f: ['Varghese', 'Anjali'] }],
    ['who is her brother?', 'lk', { f: ['Varghese'] }],
    ['and his daughter?', 'lk', { f: ['Anjali'] }],
  ]);
  seq([P(BD, sc(41))], [
    ['What props does scene 41 need?', 'ls', { f: ['Portable generator', 'Signal lamp'] }],
    ['and the wardrobe?', 'ls', { f: ['yellow raincoat', 'black raincoat'] }],
    ['any special effects?', 'lk', { f: ['Rain machine'] }],
  ]);
  seq([P(LO), P(SH)], [
    ['Which locations are only an idea?', 'ls', { f: ['Lighthouse Road', 'Backwater Ferry Jetty'] }],
    ['which scenes use them?', 'ls', { has: each([3, 30, 21]) }],
    ['and on which days are those?', 'ls', { has: each([12, 13]) }],
  ]);
  // The selection changes between turns: the first turn gets a pointer, the person adds the leaf.
  out.push(C({ fam: 'turn', picks: [P(BU)], q: 'Who plays Anjali?', t: 'lk', ptr: CR, no: ['Devika Ramanathan'] }));
  out.push(C({ fam: 'turn', picks: [P(BU), P(CR)], sb: [P(BU)], before: ['Who plays Anjali?'], q: 'ok, now who plays Anjali?', t: 'lk', f: ['Devika Ramanathan'] }));
  out.push(C({ fam: 'turn', picks: [P(SC, sc(3))], q: 'What props does scene 3 need?', t: 'ls', ptr: BD, no: ['Suitcase'] }));
  out.push(C({ fam: 'turn', picks: [P(SC, sc(3)), P(BD, sc(3))], sb: [P(SC, sc(3))], before: ['What props does scene 3 need?'], q: 'now tell me the props for scene 3', t: 'ls', f: ['Suitcase', 'Leather case of cassette tapes'] }));
  out.push(C({ fam: 'turn', picks: [P(SH)], q: 'What does the Sea safety plan say?', t: 'lk', ptr: 'notes-tasks/notes', no: ['20 knots'] }));
  out.push(C({ fam: 'turn', picks: [P(SH), P('notes-tasks/notes')], sb: [P(SH)], before: ['What does the Sea safety plan say?'], q: 'I added notes. What does the Sea safety plan say?', t: 'lk', f: ['safety boat', '20 knots'] }));
  seq([P(CS)], [
    ['Which call sheets are final?', 'ls', { has: each([1, 2, 3]) }],
    ['and which are drafts?', 'ls', { has: each([4, 5, 6]) }],
    ['what time is the crew called on the first draft?', 'lk', { f: ['05:45'] }],
  ]);
  seq([P(CR)], [
    ['Who is the director?', 'lk', { f: ['Vinod Chandrahas'] }],
    ['and the producer?', 'lk', { f: ['Mini Sebastian'] }],
    ['give me their emails', 'ls', { f: [crewOf('Vinod Chandrahas').email, crewOf('Mini Sebastian').email] }],
  ]);
  seq([P('notes-tasks/tasks')], [
    ['Which tasks are still open?', 'ls', { f: ['Confirm the offshore permit', 'Recce Lighthouse Road'] }],
    ['which of those is due first?', 'lk', { f: ['Recce Lighthouse Road'] }],
    ['and when is it due?', 'lk', { has: [iso('2027-02-10')] }],
  ]);
  seq([P('help/product-guide')], [
    ['How do I get the AI?', 'hp', { f: ['Download Straft AI'] }],
    ['can I use it offline after that?', 'hp', { has: [['without the internet', 'offline', 'works without']] }],
    ['how do I remove it?', 'hp', { f: ['Remove Straft AI'] }],
  ]);
  seq([P('idea-vault/this-project')], [
    ['What quotes are in my vault?', 'ls', { f: ['Line about the sea'] }],
    ['who said it?', 'lk', { f: ['Varghese'] }],
  ]);
  seq([P(BB)], [
    ['How many acts are there?', 'ct', { has: [num(3)] }],
    ['what is the second one called?', 'lk', { f: ['The Silence of 1999'] }],
    ['what happens in it?', 'ex', { f: ['Babu', 'Mariamma', 'Jose'] }],
  ]);
  seq([P(SL, sc(35))], [
    ['How many shots does scene 35 have?', 'ct', { has: [num(3)] }],
    ['which of them is the wide?', 'lk', { f: ['Little Flower', '24mm'] }],
    ['and what lens is the one with Jose?', 'lk', { f: ['35mm'] }],
  ]);
  seq([P(DV, dy(7))], [
    ['What is shot on Day 7?', 'ls', { has: each([19, 41]) }],
    ['who is needed?', 'ls', { f: ['Anjali', 'Sudheesh', 'Nandan'] }],
    ['what special effects?', 'lk', { f: ['Rain machine'] }],
  ]);
  seq([P('story/timeline')], [
    ['Which scenes are in 1999?', 'ls', { has: each([33, 34, 35, 36]) }],
    ['what Story Day is scene 11 on?', 'lk', { f: ['DAY 3'] }],
  ]);
  seq([P(DO)], [
    ['On which days does Salim work?', 'ls', { has: each(charDays('Salim')) }],
    ['and Nandan?', 'ls', { has: each(charDays('Nandan')) }],
  ]);
  return out;
}

// ------------------------------------------------------------------ out-of-section negatives
// Each bank question lives in one leaf (home). `avoid` lists the leaves that could also answer it.
const BANK = [
  { q: 'Who is in Scene 4?', home: SC, no: ['Mariamma'], t: 'ls', avoid: [BD, SI, DV, DO, 'production/reports', CS, SH] },
  { q: 'What is the planned amount for Camera?', home: BU, no: ['1800000', '1,800,000', '18,00,000'], t: 'lk', avoid: [] },
  { q: 'What is planned for Lead cast?', home: BU, no: ['4000000', '4,000,000', '40,00,000'], t: 'lk', avoid: [] },
  { q: 'Who plays Mariamma?', home: CR, no: ['Thresiamma'], t: 'lk', avoid: [CH, CS] },
  { q: 'Who is the director?', home: CR, no: ['Vinod Chandrahas'], t: 'lk', avoid: [CS] },
  { q: 'Which day is scene 12 shot?', home: SH, no: ['Day 5', '2027-03-05'], t: 'lk', avoid: [DV, SI, DO, 'production/reports', CS] },
  { q: 'What props does scene 3 need?', home: BD, no: ['Suitcase'], t: 'ls', avoid: [CA, 'production/reports', DV] },
  { q: 'What is the status of Kadalpuram Talkies?', home: LO, no: ['Rejected'], t: 'lk', avoid: ['production/overview', 'production/reports'] },
  { q: 'Which locations are shortlisted?', home: LO, no: ['Government Dispensary'], t: 'ls', avoid: ['production/overview'] },
  { q: 'What lens is used for the insert of the valves?', home: SL, no: ['100mm'], t: 'lk', avoid: [] },
  { q: 'How is Varghese related to Mariamma?', home: CH, no: ['brother'], t: 'rl', avoid: [] },
  { q: 'What is Act Two called?', home: BB, no: ['Silence of 1999'], t: 'lk', avoid: [OL] },
  { q: 'What time is the makeup artist called on Shoot Day 4?', home: CS, no: ['05:30'], t: 'lk', avoid: [] },
  { q: 'Which tasks are still open?', home: 'notes-tasks/tasks', no: ['Recce Lighthouse Road'], t: 'ls', avoid: [] },
  { q: 'What does the beach permit letter say?', home: 'files/files', no: ['6 am to 9 pm'], t: 'lk', avoid: [] },
  { q: 'At what wind speed do we call off the sea day?', home: 'notes-tasks/notes', no: ['20 knots'], t: 'lk', avoid: [] },
  { q: 'What is the quote about the sea in my vault?', home: 'idea-vault/this-project', no: ['Everything but time'], t: 'lk', avoid: [] },
  { q: 'What does the second panel of the Roof transmission storyboard show?', home: SB, no: ['tarpaulin'], t: 'lk', avoid: [] },
  { q: 'What sound note is on the Monsoon and salt moodboard?', home: 'production/moodboards', no: ['mast wires singing'], t: 'lk', avoid: [] },
  { q: 'How do I download Straft AI?', home: 'help/product-guide', no: ['Download Straft AI'], t: 'hp', avoid: ['help/settings'] },
  { q: 'How do I pick another colour theme?', home: 'help/settings', no: ['Appearance'], t: 'hp', avoid: ['help/product-guide'] },
  { q: 'What Story Day is scene 11 on?', home: 'story/timeline', no: ['DAY 3'], t: 'lk', avoid: [] },
  { q: 'What is in the Cold open idea?', home: 'idea-vault/global', no: ['kettle'], t: 'lk', avoid: [] },
  { q: 'On which days does Salim work?', home: DO, no: ['2, 4, 6, 9, 12'], t: 'ls', avoid: [SH, DV, SI, 'production/reports', CS] },
  { q: 'What does the Harbour Clerk say about the licence fee?', home: SC, no: ['one rupee'], t: 'lk', avoid: [SI] },
];
// Leaves a person can pick on this fixture: all except Episodes.
const PICKABLE = leafIds.filter((l) => l !== 'story/episodes');

export function negatives() {
  const out = [];
  // At least three single-leaf negatives for every leaf.
  PICKABLE.forEach((leaf, i) => {
    const pool = BANK.filter((b) => b.home !== leaf && !b.avoid.includes(leaf));
    const take = new Set();
    for (let k = 0; take.size < 3; k++) take.add((i * 3 + k * 5) % pool.length);
    for (const idx of take) {
      const b = pool[idx];
      out.push(C({ fam: 'neg', picks: [P(leaf)], q: b.q, t: b.t, ptr: b.home, no: b.no }));
    }
  });
  // Two and three leaves picked: the pointer says to remove one at the cap (the label is code-built).
  const multi = [
    [[SC, BD], BANK[1]], [[BU, LO], BANK[0]], [[CH, BB], BANK[12]], [[SH, CS], BANK[1]],
    [[SL, SB], BANK[6]], [[DV, SI], BANK[10]], [[LO, CR], BANK[5]], [[CA, 'production/reports'], BANK[3]],
    [[SC, BD, SH], BANK[1]], [[BU, LO, CR], BANK[5]], [[CH, BB, OL], BANK[12]], [[CA, 'production/reports', 'production/overview'], BANK[3]],
    [['files/files', 'notes-tasks/notes', 'notes-tasks/tasks'], BANK[10]], [[SH, DV, SI], BANK[3]],
  ];
  for (const [picks, b] of multi) {
    out.push(C({ fam: 'neg', picks: picks.map((l) => P(l)), q: b.q, t: b.t, ptr: b.home, no: b.no }));
  }
  // Part of the question fits the picked leaves, part does not: answer the part, then point.
  const part = (picks, q, spec) => out.push(C({ fam: 'neg', picks, q, t: 'ls', ...spec }));
  part([P(BU)], 'What is the actual total, and who plays Anjali?', { has: [money(13887000)], ptr: CR, no: ['Devika Ramanathan'] });
  part([P(SH)], 'Which day is scene 3 shot, and what does Anjali wear in it?', { has: [dayAlt(12)], ptr: BD, no: ['canvas shoes'] });
  part([P(SC, sc(12))], 'Who is in scene 12, and what is planned for Lighting and grip?', { f: ['Sudheesh', 'Anjali'], ptr: BU, no: ['650000', '650,000', '6,50,000'] });
  part([P(CR)], 'Who is the director, and what does Act One cover?', { f: ['Vinod Chandrahas'], ptr: BB, no: ['Fr. Thomas gives'] });
  part([P(CH)], 'What is Babu\'s role, and who plays him?', { f: ['Antagonist'], ptr: CR, no: ['Suresh Pallath'] });
  part([P(SL, sc(41))], 'How many shots does scene 41 have, and what are its storyboard panels?', { has: [num(3)], ptr: SB, no: ['battery meter'] });
  part([P(BU)], 'What is planned for Camera, and how many shoot days are there?', { has: [money(1800000)], ptr: SH, no: ['2027-03-14'] });
  part([P(CS, dy(4))], 'What time is the makeup artist called on Shoot Day 4, and what is planned for catering?', { f: ['05:30'], ptr: BU, no: ['560000', '560,000', '5,60,000'] });
  // Nothing found anywhere: "I could not find X. I looked in A and B."
  const none = (picks, q, abst) => out.push(C({ fam: 'neg', picks, q, t: 'lk', abst }));
  none([P(BU)], 'What is the budget for the helicopter?', ['Budget']);
  none([P(SC)], 'Who is in the scene with the lighthouse keeper?', ['Screenplay', 'Scenes']);
  none([P(CR)], 'Who is the stunt coordinator?', ['Cast & Crew', 'Cast', 'Crew']);
  none([P(LO)], 'What is the status of the Mumbai studio?', ['Locations', 'Location']);
  none([P('notes-tasks/tasks')], 'Which task is about the drone permit?', ['Tasks']);
  none([P(CH)], 'Who is the lighthouse keeper character?', ['Characters']);
  none([P(SH)], 'What is on Shoot Day 15?', ['Schedule', '15', '14']);
  return out;
}
