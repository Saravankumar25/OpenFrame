// The cases that belong to one leaf each. Every expected value is read from FACTS.md (through
// sections-data.mjs) or from knowledge.md for the two help leaves.
import assert from 'node:assert/strict';
import * as D from './sections-data.mjs';
import { C, P } from './sections-core.mjs';

const { num, each, money, dayAlt, scene, bdOf, charScenes, charDays, castOf, crewOf, locOf, scenes, bd, days, shots, cast, crew, locs, budget } = D;

const dt = (d) => [D.days[d - 1].date, `${d} March`, `March ${d}`];
const iso = (s) => [s, `${+s.slice(8)} ${['', 'January', 'February', 'March'][+s.slice(5, 7)]}`, `${['', 'January', 'February', 'March'][+s.slice(5, 7)]} ${+s.slice(8)}`];
const sc = (n) => ({ scene: n });
const dy = (n) => ({ day: n });
const TEASER = { screenplay: 'Low Tide Radio teaser trailer' };

export function positives() {
  const out = [];
  const mk = (leaf) => (q, t, spec = {}, more = {}) => out.push(C({ leaf, picks: [P(leaf, more)], q, t, ...spec }));

  // ---------------------------------------------------------------- Idea Vault, this project
  {
    const a = mk('idea-vault/this-project');
    a('How many items are in the project Idea Vault?', 'ct', { has: [num(4)] });
    a('What is the quote about the sea in my vault?', 'lk', { f: ['The sea gives back at low tide'] });
    a('Who is the sea quote from?', 'lk', { f: ['Varghese'] });
    a('What is the link saved in the vault?', 'lk', { has: ['shortwave-listening-guide'] });
    a('What is the caption on the monsoon palette image?', 'lk', { f: ['turmeric yellow'] });
    a('List everything in the project Idea Vault.', 'ls', { f: ['Tide song lyrics idea', 'Line about the sea', 'Shortwave listening guide', 'palette-monsoon.png'] });
    a('What is the song idea in the vault about?', 'ex', { f: ['fishermen', 'engine'] });
    a('Which vault items are notes?', 'ls', { f: ['Tide song lyrics idea'] });
    a('do i have an image saved in my ideas', 'lk', { f: ['palette-monsoon.png'] });
    a('whats the tide song idea', 'lk', { f: ['tide in, tide out'] });
    a('Is there a link saved in the project vault?', 'lk', { f: ['Shortwave listening guide'] });
    a('What kind of item is Line about the sea?', 'lk', { has: [['quote']] });
  }

  // ---------------------------------------------------------------- Idea Vault, global (low-tide-sections)
  {
    const a = mk('idea-vault/global');
    a('How many items are in the global Idea Vault?', 'ct', { has: [num(4)] });
    a('What is in the Cold open idea?', 'lk', { f: ['kettle'] });
    a('What is the Line to steal?', 'lk', { f: ['the boats that did not come back'] });
    a('Where does the Line to steal come from?', 'lk', { f: ['Notebook, 2026'] });
    a('What is the Lighting reference reel?', 'lk', { has: ['lighting-reference-reel'] });
    a('What are the title ideas in my global vault?', 'ls', { f: ['Salt Light', 'The Quiet Coast', 'Low Water'] });
    a('List what is in my global vault.', 'ls', { f: ['Cold open idea', 'Line to steal', 'Lighting reference reel', 'Title ideas'] });
    a('Which global vault item is a link?', 'lk', { f: ['Lighting reference reel'] });
    a('whats the cold open i saved globally', 'lk', { f: ['kettle', 'radio'] });
  }

  // Story > Story Page: no cases. The Story page is not offered to the assistant (see NOTES).

  // ---------------------------------------------------------------- Story, Beat Board
  {
    const a = mk('story/beat-board');
    a('How many acts are on the Beat Board?', 'ct', { has: [num(3)] });
    a('What are the acts called?', 'ls', { f: ['The Return', 'The Silence of 1999', 'The Signal'] });
    a('What is Act Two called?', 'lk', { f: ['The Silence of 1999'] });
    a('How many cards are in Act One?', 'ct', { has: [num(3)] });
    a('Which card in Act One is about the key?', 'lk', { f: ['Fr. Thomas', 'key'] });
    a('What are the cards in Act Three?', 'ls', { f: ['batteries', 'lamps', 'licence'] });
    a('Which act has the card where Mariamma confesses?', 'lk', { has: [['Act Two', 'The Silence of 1999']] });
    a('How many cards are there in total?', 'ct', { has: [num(9)] });
    a('Which cards are about Babu?', 'ls', { f: ['pressures', 'cuts the power'] });
    a('What is the first card of the story?', 'lk', { f: ['station locked', 'land'] });
    a('What is the last card on the board?', 'lk', { f: ['licence', 'mast'] });
    a('Which scene heading is linked to the card where Babu cuts the power?', 'lk', { f: ['KADALPURAM BEACH - DUSK'] });
    a('Which act has Salim\'s log?', 'lk', { has: [['Act Two', 'The Silence of 1999']] });
    a('whats act 2 about', 'ex', { f: ['Babu', 'Mariamma', 'Jose'] });
    a('Brief me on Act Two.', 'br', { f: ['Babu', 'Salim', 'Mariamma', 'Jose'] });
    a('Explain the story as the board lays it out, act by act.', 'se', { f: ['The Return', 'The Silence of 1999', 'The Signal', 'Babu'] });
    a('What does Act One set up?', 'ex', { f: ['Fr. Thomas', 'transmitter'] });
    a('Which act ends with the licence?', 'lk', { has: [['Act Three', 'The Signal']] });
  }

  // ---------------------------------------------------------------- Story, Outline
  {
    const a = mk('story/outline');
    a('Give me the outline of the story in order.', 'ls', { f: ['The Return', 'The Silence of 1999', 'The Signal'] });
    a('How many acts does the outline have?', 'ct', { has: [num(3)] });
    a('In the outline, what comes right after the card where Fr. Thomas gives Anjali the key?', 'lk', { f: ['transmitter'] });
    a('What card comes just before the licence card?', 'lk', { f: ['signal', 'roof'] });
    a('What is the middle card of Act Two in the outline?', 'lk', { f: ['last tape', 'Salim'] });
    a('List the cards of Act Three in order.', 'ls', { f: ['batteries', 'signal', 'licence'] });
    a('What is the last act called in the outline?', 'lk', { f: ['The Signal'] });
    a('Walk me through the outline as a story.', 'se', { f: ['The Return', 'The Silence of 1999', 'The Signal', 'Anjali'] });
    a('In the outline, which act holds the first broadcast?', 'lk', { has: [['Act Two', 'The Silence of 1999']] });
    a('how many cards in total in the outline', 'ct', { has: [num(9)] });
  }

  // ---------------------------------------------------------------- Story, Characters
  {
    const a = mk('story/characters');
    a('How many characters are there?', 'ct', { has: [num(13)] });
    a('What is Babu\'s role?', 'lk', { f: ['Antagonist'] });
    a('Who is the antagonist?', 'lk', { f: ['Babu'] });
    a('Which characters are leads?', 'ls', { f: ['Anjali', 'Mariamma'] });
    a('How is Varghese related to Mariamma?', 'rl', { f: ['brother'] });
    a('Who is Anjali\'s father?', 'lk', { f: ['Varghese'] });
    a('Who raised Anjali?', 'lk', { f: ['Mariamma'] });
    a('Who is Young Anjali?', 'ex', { f: ['Anjali', '1999'] });
    a('What does Mariamma call Young Anjali?', 'lk', { f: ['Kunju'] });
    a('What is the other name for Fr. Thomas?', 'lk', { f: ['Achan'] });
    a('Who is ANJU in the script?', 'lk', { f: ['Anjali'] });
    a('Who is the rival of Mariamma?', 'lk', { f: ['Babu'] });
    a('Who works for Babu?', 'lk', { f: ['Harbour Clerk'] });
    a('What is Dr. Leela to Mariamma?', 'lk', { f: ['doctor'] });
    a('Who does Nandan learn from?', 'lk', { f: ['Salim'] });
    a('How is Jose connected to Varghese?', 'rl', { f: ['Little Flower'] });
    a('Who keeps the station key?', 'lk', { f: ['Fr. Thomas'] });
    a('Which character has the Minor role?', 'lk', { f: ['Dr. Leela'] });
    a('How many relationships are recorded?', 'ct', { has: [num(12)] });
    a('What is Sudheesh to Anjali?', 'lk', { f: ['friend', 'engineer'] });
    a('Who is friends with Mariamma?', 'lk', { f: ['Rosily'] });
    a('Which character has the most scenes?', 'lk', { f: ['Anjali'], has: [num(charScenes('Anjali').length)] });
    a('How many scenes does Fr. Thomas have?', 'ct', { has: [num(charScenes('Fr. Thomas').length)] });
    a('How many scenes does Jose have?', 'ct', { has: [num(charScenes('Jose').length)] });
    a('who is varkey', 'lk', { f: ['Varghese'] });
    a('Describe Babu.', 'ds', { f: ['Antagonist', 'Mariamma'] });
    a('Describe Mariamma.', 'ds', { f: ['Varghese', 'Anjali', 'Babu'] });
    a('Describe Anjali as a character.', 'ds', { f: ['Varghese', 'Mariamma'] });
    a('How are Anjali, Mariamma and Varghese related?', 'rl2', { f: ['father', 'aunt', 'brother'] });
    a('How are Babu, Mariamma and the Harbour Clerk connected?', 'rl2', { f: ['rival', 'Harbour Clerk'] });
  }

  // ---------------------------------------------------------------- Story, Timeline (low-tide-sections)
  {
    const a = mk('story/timeline');
    a('Which scenes are on DAY 2?', 'ls', { has: each([6, 7, 8, 9, 10]) });
    a('How many Story Days are there?', 'ct', { has: [num(5)] });
    a('Which scenes are in 1999?', 'ls', { has: each([33, 34, 35, 36]) });
    a('What Story Day is scene 11 on?', 'lk', { f: ['DAY 3'] });
    a('How many scenes have no Story Day?', 'ct', { has: [num(22)] });
    a('What is the time note on scene 3?', 'lk', { f: ['Late morning'] });
    a('Which scenes have a time note?', 'ls', { has: each([3, 14, 45]) });
    a('How many scenes are in DAY 4?', 'ct', { has: [num(5)] });
    a('Is scene 20 on a Story Day?', 'lk', { has: [['Unassigned', 'no Story Day', 'not assigned', 'not on a Story Day']] });
    a('What is the time note for scene 45?', 'lk', { f: ['Dawn, a week later'] });
    a('which story day has scene 34', 'lk', { f: ['1999'] });
    a('Which scenes are on DAY 3?', 'ls', { has: each([11, 12, 13, 14]) });
    a('What time of day is scene 14?', 'lk', { f: ['After midnight'] });
    a('Which Story Days have five scenes?', 'ls', { f: ['DAY 1', 'DAY 2', 'DAY 4'] });
  }

  // Story > Episodes: no cases. The project is a feature film and the leaf is offered only on episodic projects.

  // ---------------------------------------------------------------- Screenplay, Scenes
  {
    const a = mk('screenplay/scenes');
    const interior = scenes.filter((s) => s.heading.startsWith('INT.')).length;
    const exterior = scenes.filter((s) => s.heading.startsWith('EXT.')).length;
    const night = scenes.filter((s) => /NIGHT/.test(s.heading)).length;
    assert.equal(interior + exterior, 45);
    const office = scenes.filter((s) => /HARBOUR OFFICE/.test(s.heading)).map((s) => s.n);
    a('How many scenes does LOW TIDE RADIO have?', 'ct', { has: [num(45)] });
    a('What is the heading of scene 35?', 'lk', { f: ['OPEN SEA - LITTLE FLOWER'] });
    a('Who is in scene 3?', 'ls', { f: ['Harbour Clerk', 'Anjali'] }, sc(3));
    a('What happens in scene 12?', 'ex', { f: ['Sudheesh', 'Anjali', 'transmitter'] }, sc(12));
    a('Which scenes is Salim in?', 'ls', { has: each(charScenes('Salim')) });
    a('How many scenes is Anjali in?', 'ct', { has: [num(28)] });
    a('How many interior scenes are there?', 'ct', { has: [num(interior)] });
    a('How many exterior scenes are there?', 'ct', { has: [num(exterior)] });
    a('How many night scenes are there?', 'ct', { has: [num(night)] });
    a('Which scene has Jose confess that he cut the net?', 'lk', { has: [['37']], f: ['Jose'] });
    a('Which scenes take place in 1999?', 'ls', { has: each([33, 34, 35, 36]) });
    a('In which scene does the Harbour Clerk stamp the licence?', 'lk', { has: [['43']] });
    a('Brief me on scene 41.', 'br', { f: ['Anjali', 'Sudheesh', 'Nandan', 'lamps'] }, sc(41));
    a('Brief me on scene 3.', 'br', { f: ['Anjali', 'ferry', 'Harbour Clerk'] }, sc(3));
    a('Describe scene 35.', 'ds', { f: ['Varghese', 'Jose', 'storm'] }, sc(35));
    a('Brief me on scene 32.', 'br', { f: ['Mariamma', 'Varghese', 'storm'] }, sc(32));
    a('Explain the story of LOW TIDE RADIO from beginning to end.', 'se', { f: ['Anjali', 'Babu', 'Mariamma'] });
    a('Which characters speak in the teaser?', 'ls', { f: ['Anjali', 'Mariamma', 'Sudheesh', 'Nandan'] }, TEASER);
    a('How many scenes does the teaser have?', 'ct', { has: [num(3)] }, TEASER);
    a('What happens in scene 46?', 'lk', { abst: ['46', '45 scenes', 'screenplay'] });
    a('What is the heading of scene 41?', 'lk', { f: ['COAST RADIO STATION - ROOF - NIGHT'] });
    a('Who is in scene 22?', 'ls', { f: ['Fr. Thomas', 'Mariamma', 'Rosily'] }, sc(22));
    a('In which scene does Anjali first meet Sudheesh?', 'lk', { has: [['10']] });
    a('What does Salim give Anjali in scene 30?', 'lk', { f: ['call log'] }, sc(30));
    a('Which scene has them play the sealed cassette?', 'lk', { has: [['31']] });
    a('Which scenes are in the Harbour Office?', 'ls', { has: each(office) });
    a('Who speaks in scene 45?', 'ls', { f: ['Anjali', 'Mariamma', 'Salim'] }, sc(45));
    a('Describe scene 1.', 'ds', { f: ['Mariamma', 'Jose', 'Harbour Clerk'] }, sc(1));
    a('Brief me on scene 24: who is there and what happens?', 'br', { f: ['Babu', 'Harbour Clerk', 'fine'] }, sc(24));
    a('what happens at the start of the film', 'ex', { f: ['Mariamma'] });
    a('Which is the first scene with Dr. Leela?', 'lk', { has: [['16']] });
  }

  // ---------------------------------------------------------------- Breakdown
  {
    const a = mk('breakdown/breakdown');
    const noSound = bd.filter((s) => s.sound.length === 0).length;
    assert.equal(noSound, 18);
    a('What props does scene 3 need?', 'ls', { f: ['Suitcase', 'Leather case of cassette tapes'] }, sc(3));
    a('What does Mariamma wear in scene 1?', 'lk', { f: ['faded blue mundu'] }, sc(1));
    a('What vehicle is in scene 9?', 'lk', { f: ['Pickup truck'] }, sc(9));
    a('Which scene has the rain machine?', 'lk', { has: [['41']] });
    a('What are the special effects in scene 35?', 'lk', { f: ['Rain rig', 'wave spray'] }, sc(35));
    a('Which scenes have a VFX element?', 'ls', { has: each([13, 35, 41]) });
    a('How many prop elements are in the breakdown?', 'ct', { has: [num(97)] });
    a('How many different props are there?', 'ct', { has: [num(73)] });
    a('How many vehicle elements are in the breakdown?', 'ct', { has: [num(7)] });
    a('Which scenes have animals?', 'ls', { has: each([1, 17, 37, 45]) });
    a('Who are the extras in scene 17?', 'lk', { f: ['Fish buyers'] }, sc(17));
    a('What is the sound for scene 26?', 'lk', { f: ['Live radio broadcast', 'valve hum'] }, sc(26));
    a('Which scenes have no wardrobe?', 'ls', { has: each([12, 13, 14, 25, 29, 39]) });
    a('What hair and makeup does scene 33 need?', 'ls', { f: ['two braids', 'younger look'] }, sc(33));
    a('How many special effects elements are there?', 'ct', { has: [num(3)] });
    a('What does Anjali wear in scene 41?', 'lk', { f: ['yellow raincoat'] }, sc(41));
    a('What is Sudheesh\'s wardrobe in scene 41?', 'lk', { f: ['black raincoat'] }, sc(41));
    a('Which scenes use the ferry boat?', 'ls', { has: each([3, 30]) });
    a('How many hair and makeup elements are in the breakdown?', 'ct', { has: [num(12)] });
    a('List the props in scene 41.', 'ls', { f: ['Portable generator', 'Signal lamp', 'Microphone', 'Car batteries'] }, sc(41));
    a('Which scenes use a rubber stamp?', 'ls', { has: each(D.sceneHas('props', 'Rubber stamp')) });
    a('Which scenes use steel tea glasses?', 'ls', { has: each(D.sceneHas('props', 'Steel tea glasses')) });
    a('How many wardrobe elements are in the breakdown?', 'ct', { has: [num(102)] });
    a('what animals are in scene 45', 'lk', { f: ['Gulls'] }, sc(45));
    a('How many extras elements are there?', 'ct', { has: [num(11)] });
    a('How many scenes have no sound element?', 'ct', { has: [num(noSound)] });
    a('What props does scene 12 have?', 'ls', { f: ['Vacuum tubes', 'Soldering iron'] }, sc(12));
    a('What is the sound in scene 35?', 'lk', { f: ['Storm and radio crackle'] }, sc(35));
    a('What wardrobe does Varghese have in scene 35?', 'lk', { f: ['checked lungi', 'oilskin jacket'] }, sc(35));
    a('Brief me on everything scene 41 needs.', 'br', { f: ['Portable generator', 'yellow raincoat', 'Rain machine'] }, sc(41));
    a('How many cast elements are in the breakdown?', 'ct', { has: [num(120)] });
  }

  // ---------------------------------------------------------------- Production, Overview
  {
    const a = mk('production/overview');
    a('Which draft is the Production Source?', 'lk', { f: ['LOW TIDE RADIO'] });
    a('How many locations are still unresolved?', 'ct', { has: [num(5)] });
    a('Which characters do not have an actor yet?', 'ls', { f: ['Jose'] });
    a('How many characters does the production count?', 'ct', { has: [num(13)] });
    a('How many locations does the production have?', 'ct', { has: [num(16)] });
    a('What do I need to do next to prepare the shoot?', 'lp', { f: ['Jose'] });
    a('Is the teaser the Production Source?', 'lk', { f: ['LOW TIDE RADIO'] });
  }

  // ---------------------------------------------------------------- Production, Catalog
  {
    const a = mk('production/catalog');
    a('How many different props are in the catalog?', 'ct', { has: [num(73)] });
    a('Which scenes use the Brass lamp?', 'ls', { has: each(D.sceneHas('props', 'Brass lamp')) });
    a('Which prop is used in the most scenes?', 'lk', { f: ['Steel tea glasses'], has: [num(5)] });
    a('How many scenes use steel tea glasses?', 'ct', { has: [num(5)] });
    a('Where is the Suitcase used?', 'ls', { has: each(D.sceneHas('props', 'Suitcase')) });
    a('Which vehicles are in the catalog?', 'ls', { f: ['Fishing boats', 'Ferry boat', 'Pickup truck', 'Auto rickshaw', 'Inspector\'s jeep', 'Little Flower'] });
    a('How many different vehicles are there?', 'ct', { has: [num(6)] });
    a('Which scenes use the Sealed cassette tape?', 'ls', { has: each(D.sceneHas('props', 'Sealed cassette tape')) });
    a('Which props are used in more than two scenes?', 'ls', { f: ['Steel tea glasses', 'Rubber stamp'] });
    a('Which scenes use the rubber stamp?', 'ls', { has: each(D.sceneHas('props', 'Rubber stamp')) });
  }

  // ---------------------------------------------------------------- Production, Locations
  {
    const a = mk('production/locations');
    a('How many locations does the production have?', 'ct', { has: [num(16)] });
    a('How many locations are confirmed?', 'ct', { has: [num(10)] });
    a('What is the status of Kadalpuram Talkies?', 'lk', { f: ['Rejected'] });
    a('Why was Kadalpuram Talkies rejected?', 'lk', { f: ['unsafe'] });
    a('What is the address of Chaaya Tea Shop?', 'lk', { f: ['Market Junction'] });
    a('Who is the contact for the Harbour Office?', 'lk', { f: ['Harbour master', '+91 90000 30006'] });
    a('Which locations are only an idea so far?', 'ls', { f: ['Lighthouse Road', 'Backwater Ferry Jetty'] });
    a('Which locations are shortlisted?', 'ls', { f: ['Offshore Waters', 'Babu\'s Boat Yard', 'Government Dispensary'] });
    a('What location fee does Babu\'s Boat Yard want?', 'lk', { has: [['25,000', '25000']] });
    a('When is the Coast Radio Station Roof unavailable?', 'lk', { f: ['Roof repair'], has: [dt(6)] });
    a('When is Offshore Waters unavailable?', 'lk', { f: ['Fisheries ban'], has: [dt(9), dt(13)] });
    a('When is the Little Flower fishing boat available?', 'lk', { has: [dt(14)] });
    a('How many cars can park at the Kadalpuram Fish Landing Centre?', 'lk', { has: [['Eight', '8']] });
    a('When can we shoot at the Government Dispensary?', 'lk', { f: ['Sunday afternoons'] });
    a('When does the bell ring at St. Antony\'s Church?', 'lk', { f: ['6 pm', '7 pm'] });
    a('What power do we need at the Coast Radio Station?', 'lk', { f: ['Generator'] });
    a('Which location needs a harness?', 'lk', { f: ['Coast Radio Station Roof'] });
    a('Who is the contact for the Coast Radio Station?', 'lk', { f: ['Fr. Thomas', '+91 90000 30004'] });
    a('Which locations list Fr. Thomas as the contact?', 'ls', { f: ['Coast Radio Station', 'St. Antony\'s Church'] });
    a('Which locations have an unavailable date?', 'ls', { f: ['Coast Radio Station Roof', 'Offshore Waters', 'St. Antony\'s Church'] });
    a('Is the beach permit confirmed?', 'lk', { has: [['confirmed']] });
    a('Which location is open from 4 am?', 'lk', { f: ['Kadalpuram Fish Landing Centre'] });
    a('What are the Harbour Office hours?', 'lk', { f: ['10 am', '4 pm'] });
    a('Describe the Coast Radio Station location.', 'ds', { f: ['Station Road', 'Fr. Thomas', 'Generator'] });
    a('Describe Kadalpuram Fish Landing Centre.', 'ds', { f: ['Landing Road', 'Port authority letter', 'Harbour Clerk office'] });
    a('What toilets are at Kadalpuram Beach?', 'lk', { f: ['Portable toilets'] });
    a('Who is the contact for Anjali\'s Rented Room?', 'lk', { f: ['Bank manager'] });
    a('what is the noise problem at the fish landing centre', 'lk', { f: ['Boat engines'] });
    a('Describe Offshore Waters.', 'ds', { f: ['Shortlisted', 'permit', 'safety boat'] });
  }

  // ---------------------------------------------------------------- Production, Cast & Crew
  {
    const a = mk('production/cast-crew');
    const productionCrew = crew.filter((c) => c.dept === 'Production').length;
    assert.equal(productionCrew, 3);
    a('Who plays Anjali?', 'lk', { f: ['Devika Ramanathan'] });
    a('Who plays Babu?', 'lk', { f: ['Suresh Pallath'] });
    a('Who plays Jose?', 'lk', { abst: ['cast'] });
    a('How many cast members are there?', 'ct', { has: [num(12)] });
    a('How many crew members are there?', 'ct', { has: [num(14)] });
    a('What is Devika Ramanathan\'s phone number?', 'lk', { f: [castOf('Anjali').phone] });
    a('What is Nizar Hameed\'s email?', 'lk', { f: [castOf('Salim').email] });
    a('Who is the director?', 'lk', { f: ['Vinod Chandrahas'] });
    a('Who is the producer?', 'lk', { f: ['Mini Sebastian'] });
    a('Who is the director of photography?', 'lk', { f: ['Harikrishnan Embran'] });
    a('Which department is Remya Sukumaran in?', 'lk', { has: [['Hair / Makeup', 'Hair/Makeup', 'Hair and Makeup']] });
    a('Which actors are child performers?', 'ls', { f: ['Aadhya Varma', 'Adithyan Raj'] });
    a('When is Aadhya Varma unavailable?', 'lk', { f: ['School exams'], has: [dt(8)] });
    a('Who has a bad back?', 'lk', { f: ['Sanoj Kurup'] });
    a('Who brings his own ham radio set?', 'lk', { f: ['Nizar Hameed'] });
    a('What other job does Prasad Venugopalan have on the unit?', 'lk', { f: ['Production Assistant'] });
    a('Which crew member is unavailable on 2027-03-07?', 'lk', { f: ['Harikrishnan Embran'] });
    a('Why is Remya Sukumaran unavailable on 2027-03-10?', 'lk', { f: ['another film'] });
    a('Which actor has his own pickup truck?', 'lk', { f: ['Suresh Pallath'] });
    a('Who plays Fr. Thomas?', 'lk', { f: ['Joy Mathen'] });
    a('Who plays Young Anjali?', 'lk', { f: ['Aadhya Varma'], no: ['Devika Ramanathan'] });
    a('Who plays Dr. Leela?', 'lk', { f: ['Geetha Kaimal'] });
    a('Who plays Varghese?', 'lk', { f: ['Abraham Kallarakkal'] });
    a('When is Abraham Kallarakkal unavailable?', 'lk', { f: ['Stage tour'] });
    a('How many crew members are in the Production department?', 'ct', { has: [num(productionCrew)] });
    a('Who is the sound recordist?', 'lk', { f: ['Anwar Basheer'] });
    a('Who is the gaffer?', 'lk', { f: ['Shaji Varkey'] });
    a('Who is the location manager and how do I reach him?', 'lk', { f: ['Biju Antony', crewOf('Biju Antony').phone] });
    a('Tell me about Beena Chacko.', 'ds', { f: ['Rosily', '11 pm', 'Temple festival'] });
    a('Who plays Sudheesh and what does he need on set?', 'lk', { f: ['Sanoj Kurup', 'chair'] });
    a('Who is the script supervisor?', 'lk', { f: ['Divya Mathai'] });
    a('Who is the costume designer?', 'lk', { f: ['Sherin Abdulla'] });
    a('Which actors are unavailable on 2027-03-08?', 'ls', { f: ['Aadhya Varma', 'Adithyan Raj'] });
  }

  // ---------------------------------------------------------------- Production, Moodboards
  {
    const a = mk('production/moodboards');
    a('How many moodboards does the project have?', 'ct', { has: [num(1)] });
    a('What is the moodboard called?', 'lk', { f: ['Monsoon and salt'] });
    a('What colours are on the Monsoon and salt board?', 'lk', { f: ['wet slate', 'rust', 'turmeric yellow'] });
    a('What is the sound note on the moodboard?', 'lk', { f: ['mast wires singing numbers'] });
    a('What texture does the board call for?', 'lk', { f: ['salt-cracked paint'] });
    a('What light does the board ask for?', 'lk', { f: ['dawn at low tide', 'long shadows'] });
    a('How many notes are on the board?', 'ct', { has: [num(4)] });
    a('How many images are on the moodboard?', 'ct', { has: [['0', 'no images', 'none', 'zero']] });
    a('Describe the look of the film from the moodboard.', 'ds', { f: ['slate', 'turmeric', 'low tide'] });
    a('whats on the mood board', 'ls', { f: ['Colour', 'Sound', 'Texture', 'Light'] });
  }

  // ---------------------------------------------------------------- Production, Storyboards
  {
    const a = mk('production/storyboards');
    a('How many storyboards are there?', 'ct', { has: [num(1)] });
    a('Which scene is the Roof transmission storyboard for?', 'lk', { has: [['41']] });
    a('How many panels does the storyboard have?', 'ct', { has: [num(4)] }, sc(41));
    a('What is in panel 1?', 'lk', { f: ['lamps appear on the dark water'] }, sc(41));
    a('What happens in panel 2?', 'lk', { f: ['microphone', 'tarpaulin'] }, sc(41));
    a('Who watches the battery meter?', 'lk', { f: ['Sudheesh'] }, sc(41));
    a('What does the last panel show?', 'lk', { f: ['Nandan', 'lamp', 'boat'] }, sc(41));
    a('Do the panels have drawn images yet?', 'lk', { has: [['empty', 'no image', 'not yet']] }, sc(41));
    a('Walk me through the Roof transmission storyboard.', 'br', { f: ['lamps', 'microphone', 'battery', 'boat'] }, sc(41));
    a('What is panel 3?', 'lk', { f: ['battery meter'] }, sc(41));
  }

  // ---------------------------------------------------------------- Production, Shot Lists
  {
    const a = mk('production/shot-lists');
    const closeUps = shots.filter((s) => s.size === 'Close-up').length;
    const bySceneCount = (n) => shots.filter((s) => s.scene === n).length;
    assert.equal(bySceneCount(35), 3);
    assert.equal(bySceneCount(41), 3);
    a('How many shots are in the shot list?', 'ct', { has: [num(11)] });
    a('How many shots does scene 35 have?', 'ct', { has: [num(3)] }, sc(35));
    a('How many shots does scene 41 have?', 'ct', { has: [num(3)] }, sc(41));
    a('What lens is used for the insert of the valves?', 'lk', { f: ['100mm'] });
    a('Which shot uses a crane?', 'lk', { f: ['crane up', 'roof'] });
    a('Which shots are handheld?', 'ls', { f: ['Little Flower', 'Varghese', 'Jose'] });
    a('What angle is the wide of the Little Flower in the swell?', 'lk', { f: ['High angle'] });
    a('What lens is the close on Anjali at the microphone?', 'lk', { f: ['85mm'] });
    a('Which shot uses the 18mm lens?', 'lk', { f: ['roof', 'lamps'] });
    a('What movement is the two shot of Sudheesh and Anjali over the transmitter?', 'lk', { f: ['Slow push in'] });
    a('How many close-ups are there?', 'ct', { has: [num(closeUps)] });
    a('Which shots are low angle?', 'ls', { f: ['padlocked gate', 'bailing'] });
    a('How many shots does scene 12 have?', 'ct', { has: [num(2)] }, sc(12));
    a('List the shots for scene 1.', 'ls', { f: ['Wide of the landing', 'Mariamma\'s hands'] }, sc(1));
    a('What size is the shot of Jose bailing?', 'lk', { f: ['Close-up'] });
    a('Which scenes have three shots?', 'ls', { has: each([35, 41]) });
    a('Brief me on the shots planned for scene 35.', 'br', { f: ['Little Flower', 'Varghese', 'Jose', 'handheld'] }, sc(35));
    a('What is the shot for scene 6?', 'lk', { f: ['padlocked gate', 'Tilt up'] }, sc(6));
    a('how many shots for scene 1', 'ct', { has: [num(2)] }, sc(1));
  }

  // ---------------------------------------------------------------- Production, Schedule
  {
    const a = mk('production/schedule');
    a('How many shoot days are there?', 'ct', { has: [num(14)] });
    a('Which scenes are on Shoot Day 4?', 'ls', { has: each([6, 13, 24, 42]) }, dy(4));
    a('Which day is scene 3 shot?', 'lk', { has: [dayAlt(12)] });
    a('Which day is scene 35 shot?', 'lk', { has: [dayAlt(14)] });
    a('Where do we shoot on Day 7?', 'lk', { f: ['Coast Radio Station Roof'] }, dy(7));
    a('What is the note on Day 4?', 'lk', { f: ['Exterior station day'] }, dy(4));
    a('What is the note on Day 14?', 'lk', { f: ['Sea day', 'life jackets'] }, dy(14));
    a('How many open warnings does the schedule have?', 'ct', { has: [num(11)] });
    a('Which actors are unavailable on Shoot Day 8?', 'ls', { f: ['Aadhya Varma', 'Adithyan Raj'] }, dy(8));
    a('What is the warning for Day 13?', 'lk', { f: ['Abraham Kallarakkal', 'unavailable'] }, dy(13));
    a('On which days is Anjali needed at two locations?', 'ls', { f: ['Anjali'], has: each([10, 12, 13]) });
    a('How many scenes are on Day 2?', 'ct', { has: [num(5)] }, dy(2));
    a('How many scenes are not scheduled?', 'ct', { has: [['0', 'no scene', 'none', 'zero', 'all']] });
    a('Which day has the most scenes?', 'lk', { has: [dayAlt(2)] });
    a('What date is Shoot Day 9?', 'lk', { has: [dt(9)] }, dy(9));
    a('Which day clashes with Babu\'s availability?', 'lk', { f: ['Babu', 'Film promotion event'], has: [dayAlt(5)] });
    a('Which days shoot at Kadalpuram Beach?', 'ls', { has: each([8, 9]) });
    a('Which days are at the Coast Radio Station?', 'ls', { has: each([4, 5, 6]) });
    a('What is the first shoot date?', 'lk', { has: [dt(1)] });
    a('Who is needed at two locations on Shoot Day 12?', 'ls', { f: ['Anjali', 'Harbour Clerk'] }, dy(12));
    a('What is scheduled on Shoot Day 14?', 'lk', { f: ['Little Flower'], has: [['35']] }, dy(14));
    a('which day is the sea day', 'lk', { has: [dayAlt(14)] });
    a('What is the warning on Day 5?', 'lk', { f: ['Babu', 'unavailable'] }, dy(5));
    a('Which day do we shoot scene 21?', 'lk', { has: [dayAlt(13)] });
    a('Brief me on Shoot Day 12.', 'br', { f: ['Babu\'s Boat Yard', 'Backwater Ferry Jetty', 'Anjali'] }, dy(12));
    a('What clashes are there on Shoot Day 10?', 'ls', { f: ['Anjali', 'Mariamma'] }, dy(10));
    a('Which scenes are shot on Day 11?', 'ls', { has: each([8, 25, 43]) }, dy(11));
    a('On which day is Mariamma needed at two locations?', 'lk', { f: ['Mariamma'], has: [dayAlt(10)] });
    a('Which location do we shoot at on Day 1?', 'lk', { f: ['Kadalpuram Fish Landing Centre'] }, dy(1));
  }

  // ---------------------------------------------------------------- Production, Day Out of Days
  {
    const a = mk('production/day-out-of-days');
    const when = (c) => charDays(c);
    a('On which days does Salim work?', 'ls', { has: each(when('Salim')) });
    a('How many days does Mariamma work?', 'ct', { has: [num(when('Mariamma').length)] });
    a('What is the first day Nandan works?', 'lk', { has: [dayAlt(when('Nandan')[0])] });
    a('On which day does Nandan finish?', 'lk', { has: [dayAlt(when('Nandan').at(-1))] });
    a('Who works on Day 14?', 'ls', { f: ['Varghese', 'Jose'] }, dy(14));
    a('Who works on Day 7?', 'ls', { f: ['Anjali', 'Sudheesh', 'Nandan'] }, dy(7));
    a('Does Young Anjali work on Day 8?', 'lk', { has: [['yes', 'Yes']] }, dy(8));
    a('How many days does Anjali work?', 'ct', { has: [num(when('Anjali').length)] });
    a('On which day does Fr. Thomas work?', 'lk', { has: [dayAlt(when('Fr. Thomas')[0])] });
    a('Which cast member works the most days?', 'lk', { f: ['Anjali'], has: [num(when('Anjali').length)] });
    a('How many days does Dr. Leela work?', 'ct', { has: [num(when('Dr. Leela').length)] });
    a('Who works on Day 3?', 'ls', { f: ['Mariamma', 'Anjali', 'Young Anjali'] }, dy(3));
    a('When does Varghese first work?', 'lk', { has: [dayAlt(when('Varghese')[0])] });
    a('On which days does Babu work?', 'ls', { has: each(when('Babu')) });
    a('How many days does the Harbour Clerk work?', 'ct', { has: [num(when('Harbour Clerk').length)] });
  }

  // ---------------------------------------------------------------- Production, Daily View
  {
    const a = mk('production/daily-view');
    const props7 = [...bdOf(19).props, ...bdOf(41).props];
    a('What is shot on Day 4?', 'ls', { has: each([6, 13, 24, 42]) }, dy(4));
    a('What date is Shoot Day 12 and where do we shoot?', 'lk', { f: ['Babu\'s Boat Yard', 'Backwater Ferry Jetty'], has: [dt(12)] }, dy(12));
    a('Who is needed on Day 14?', 'ls', { f: ['Varghese', 'Jose'] }, dy(14));
    a('What props are needed on Day 7?', 'ls', { f: props7.slice(0, 4) }, dy(7));
    a('What is the note for Day 14?', 'lk', { f: ['Sea day'] }, dy(14));
    a('What special effects are needed on Day 7?', 'lk', { f: ['Rain machine'] }, dy(7));
    a('Which vehicles are needed on Day 14?', 'lk', { f: ['Little Flower'] }, dy(14));
    a('Who is needed on Day 3?', 'ls', { f: ['Mariamma', 'Anjali', 'Young Anjali'] }, dy(3));
    a('How many scenes are on Day 2?', 'ct', { has: [num(5)] }, dy(2));
    a('What is the Day 4 note?', 'lk', { f: ['Makeup artist', 'early call'] }, dy(4));
    a('Which location is used on Day 9?', 'lk', { f: ['Kadalpuram Beach'] }, dy(9));
    a('Brief me on Day 14.', 'br', { f: ['Little Flower', 'Varghese', 'Jose', 'Sea day'] }, dy(14));
    a('Brief me on Day 7.', 'br', { f: ['Coast Radio Station Roof', 'Anjali', 'Sudheesh', 'Nandan'] }, dy(7));
    a('What vehicles are needed on Day 12?', 'ls', { f: ['Pickup truck', 'Ferry boat'] }, dy(12));
  }

  // ---------------------------------------------------------------- Production, Sides
  {
    const a = mk('production/sides');
    a('Which scenes are in the sides for Day 7?', 'ls', { has: each([19, 41]) }, dy(7));
    a('How many scenes are in the sides for Day 2?', 'ct', { has: [num(5)] }, dy(2));
    a('Which cast are in the sides for Day 11?', 'ls', { f: ['Harbour Clerk', 'Anjali', 'Babu'] }, dy(11));
    a('What happens on the roof in the Day 7 sides?', 'ex', { f: ['Anjali', 'Sudheesh', 'Nandan'] }, dy(7));
    a('What does Anjali say into the microphone on the roof?', 'lk', { f: ['answer with a light'] }, { day: 7, scene: 41 });
    a('What does the Harbour Clerk say about the licence fee?', 'lk', { f: ['one rupee'] }, { day: 11, scene: 43 });
    a('In the Day 14 sides, what happens to the boat?', 'ex', { f: ['Little Flower', 'engine'] }, dy(14));
    a('Who speaks in the sides for Day 1?', 'ls', { f: ['Mariamma', 'Jose', 'Harbour Clerk', 'Babu', 'Anjali'] }, dy(1));
    a('Which scenes are in the Day 10 sides?', 'ls', { has: each([7, 22, 38, 16]) }, dy(10));
    a('What does the Harbour Clerk ask Anjali at the jetty?', 'lk', { f: ['Name, purpose, how long'] }, { day: 12, scene: 3 });
  }

  // ---------------------------------------------------------------- Production, Reports
  {
    const a = mk('production/reports');
    a('In the Prop Report, which scenes use the Suitcase?', 'ls', { has: each([3, 4]) });
    a('In the Cast Scene Report, how many scenes is Anjali in?', 'ct', { has: [num(28)] });
    a('In the Location Report, how many scenes are at the Coast Radio Station?', 'ct', { has: [num(scenes.filter((s) => s.loc === 'Coast Radio Station').length)] });
    a('In the Schedule Report, which scenes are on Shoot Day 1?', 'ls', { has: each([1, 17, 37]) });
    a('In the Scene Report, what is the heading of scene 20?', 'lk', { f: ['CHAAYA TEA SHOP - EVENING'] });
    a('In the Breakdown Completeness report, which scenes have no wardrobe?', 'ls', { has: each([12, 13, 14, 25, 29, 39]) });
    a('How many scenes have no sound in the Breakdown Completeness report?', 'ct', { has: [num(18)] });
    a('In the Cast Scene Report, how many scenes is Salim in?', 'ct', { has: [num(charScenes('Salim').length)] });
    a('Which location is used by the most scenes in the Location Report?', 'lk', { f: ['Coast Radio Station'], has: [num(11)] });
    a('In the Prop Report, which scenes use a rubber stamp?', 'ls', { has: each([8, 25, 43]) });
    a('Which scenes does the Schedule Report put on Day 2?', 'ls', { has: each([2, 10, 20, 28, 44]) });
    a('cast scene report for Young Anjali', 'ls', { has: each([33, 34, 36]) });
  }

  // ---------------------------------------------------------------- Production, Budget
  {
    const a = mk('production/budget');
    const line = (name) => budget.find((l) => l.line === name);
    const cat = (c) => budget.filter((l) => l.cat === c).reduce((n, l) => n + l.planned, 0);
    const d = (n) => money(n).slice(0, 3);
    const planned = (name, q, t = 'lk') => a(q, t, { has: [money(line(name).planned)] });
    a('What is the planned total of all budget lines?', 'lk', { has: [money(16460000)] });
    a('What is the actual total?', 'lk', { has: [money(13887000)] });
    a('What is the difference between planned and actual?', 'lk', { has: [money(223000)] });
    planned('Lead cast', 'What is planned for Lead cast?');
    a('What was the actual for Lead cast?', 'lk', { has: [money(3850000)] });
    a('Are we under or over budget on Camera?', 'lk', { has: [['over']], f: [] });
    a('Which budget lines have no actual yet?', 'ls', { f: ['Lighting and grip', 'Edit and sound mix', 'Contingency'] });
    a('How many budget lines are there?', 'ct', { has: [num(11)] });
    a('Which line has the largest planned amount?', 'lk', { f: ['Lead cast'] });
    a('Which lines are over budget?', 'ls', { f: ['Supporting cast', 'Camera', 'Boat hire and sea safety'] });
    a('What is the currency of the budget?', 'lk', { has: [['INR', 'rupee']] });
    planned('Unit catering', 'What is planned for Unit catering?');
    planned('Contingency', 'What is the contingency?');
    a('How much was spent on Unit catering?', 'lk', { has: [money(line('Unit catering').actual)] });
    planned('Camera', 'What is the planned amount for Camera?');
    a('What is the actual for Camera?', 'lk', { has: [money(line('Camera').actual)] });
    planned('Edit and sound mix', 'What is the planned amount for Edit and sound mix?');
    a('How much is planned for Boat hire and sea safety?', 'lk', { has: [money(line('Boat hire and sea safety').planned)] });
    a('What is the difference on Lead cast?', 'lk', { has: [money(150000)] });
    planned('Direction and production crew', 'What is the planned amount for Direction and production crew?');
    planned('Location fees and permits', 'What is planned for Location fees and permits?');
    planned('Art department and props', 'How much is planned for the Art department and props?');
    a('what was the actual for the supporting cast', 'lk', { has: [money(line('Supporting cast').actual)] });
    planned('Lighting and grip', 'What is the planned amount for Lighting and grip?');
    const castTotal = cat('Cast');
    const equipTotal = cat('Equipment');
    a('What is the total planned for the Cast category?', 'lk', { has: [money(castTotal)], d: money(castTotal) });
    a('What is the total planned for Equipment?', 'lk', { has: [money(equipTotal)], d: money(equipTotal) });
    void d;
  }

  // ---------------------------------------------------------------- Call Sheets
  {
    const a = mk('call-sheets/call-sheets');
    a('How many call sheets are there?', 'ct', { has: [num(6)] });
    a('How many call sheets are final?', 'ct', { has: [num(3)] });
    a('What is the status of the call sheet for Shoot Day 4?', 'lk', { f: ['Draft'] }, dy(4));
    a('What time is the crew called on Shoot Day 3?', 'lk', { f: ['06:30'] }, dy(3));
    a('What time is the crew called on Shoot Day 4?', 'lk', { f: ['05:45'] }, dy(4));
    a('What time is the makeup artist called on Shoot Day 4?', 'lk', { f: ['05:30', 'Remya Sukumaran'] }, dy(4));
    a('Which shoot days have no call sheet?', 'lk', { f: ['7', '14'] });
    a('Which call sheet is for a night shoot?', 'lk', { f: ['Shoot Day 6', '16:00'] });
    a('When is the director of photography called on Shoot Day 4?', 'lk', { f: ['Harikrishnan Embran', '06:00'] }, dy(4));
    a('Which call sheets are drafts?', 'ls', { has: each([4, 5, 6]) });
    a('Which call sheets are final?', 'ls', { has: each([1, 2, 3]) });
    a('What date is the Shoot Day 3 call sheet?', 'lk', { has: [dt(3)] }, dy(3));
    a('Who is the first assistant director on the Shoot Day 4 call sheet?', 'lk', { f: ['Sajan Palat', '05:45'] }, dy(4));
    a('What is the title of the Shoot Day 4 call sheet?', 'lk', { f: ['Call Sheet: Day 4'] }, dy(4));
    a('Is the Shoot Day 6 call sheet final?', 'lk', { has: [['draft', 'Draft', 'not final']] }, dy(6));
    a('What time is the crew called on Shoot Day 1?', 'lk', { f: ['06:00'] }, dy(1));
    a('Brief me on the Day 4 call sheet.', 'br', { f: ['Remya Sukumaran', '05:30', 'Harikrishnan Embran', 'Draft'] }, dy(4));
    a('What time is the camera assistant called on Shoot Day 4?', 'lk', { f: ['Jithin Das', '06:00'] }, dy(4));
    a('What time is the sound recordist called on Shoot Day 4?', 'lk', { f: ['Anwar Basheer', '06:00'] }, dy(4));
  }

  // ---------------------------------------------------------------- Files
  {
    const a = mk('files/files');
    a('How many files are in the project?', 'ct', { has: [num(4)] });
    a('What are the folders called?', 'ls', { f: ['Permits', 'Reference'] });
    a('What does the beach permit letter say?', 'lk', { f: ['1 to 14 March 2027', '6 am to 9 pm'] });
    a('What is in the harbour inspection checklist?', 'ls', { f: ['life jackets', 'safety boat', 'radio check', 'weather log'] });
    a('How tall is the coast station mast?', 'lk', { f: ['42 metres'] });
    a('When is low water in March 2027?', 'lk', { f: ['1, 8 and 14 March'] });
    a('Which files are in the Permits folder?', 'ls', { f: ['beach-permit-letter.txt', 'harbour-inspection-checklist.txt'] });
    a('What lamps does the mast have?', 'lk', { f: ['red obstruction lamps'] });
    a('Which side of the mast does the feed run on?', 'lk', { f: ['east face'] });
    a('what is the tide table file called', 'lk', { f: ['tide-table-march-2027.txt'] });
  }

  // ---------------------------------------------------------------- Notes & Tasks, Notes
  {
    const a = mk('notes-tasks/notes');
    const priv = ['PRIVATE-OTHER-1', 'PRIVATE-OTHER-2'];
    a('What does the Sea safety plan say?', 'lk', { f: ['safety boat', 'life jacket'] });
    a('At what wind speed do we call off the sea day?', 'lk', { f: ['20 knots'] });
    a('Which note is pinned?', 'lk', { f: ['Sea safety plan'] });
    a('How long should room tone be?', 'lk', { f: ['sixty seconds'] });
    a('What Malayalam words are in the dialect notes?', 'ls', { f: ['chechi', 'chetta', 'kutty'] });
    a('What is the status of the offshore permit according to the notes?', 'lk', { f: ['pending'] });
    a('Summarise the Reminders for the unit note.', 'ex', { f: ['salt-proof bags', 'walkie'], no: priv });
    a('What is in my private note?', 'lk', { has: [['PRIVATE-LOCAL-1']], no: priv });
    a('Show me every private note in the project.', 'ls', { has: [['PRIVATE-LOCAL-1']], no: priv });
    a('Is the beach permit confirmed according to the notes?', 'lk', { has: [['confirmed']] });
    a('How are radio voices recorded?', 'lk', { f: ['clean', 'post'] });
    a('What should we bring on the sea day?', 'lk', { f: ['salt-proof bags'] });
    a('What spelling rule do the dialect notes give?', 'lk', { f: ['same way in every scene'] });
  }

  // ---------------------------------------------------------------- Notes & Tasks, Tasks
  {
    const a = mk('notes-tasks/tasks');
    a('How many tasks are there?', 'ct', { has: [num(8)] });
    a('How many tasks are still open?', 'ct', { has: [num(6)] });
    a('Which tasks are done?', 'ls', { f: ['Pay the deposit', 'Book hotel rooms'] });
    a('Which open task is due first?', 'lk', { f: ['Recce Lighthouse Road'] });
    a('What is due on 2027-03-05?', 'lk', { f: ['Order car batteries'] });
    a('Which task has no due date?', 'lk', { f: ['Record wild tracks'] });
    a('When is the offshore permit task due?', 'lk', { has: [iso('2027-02-15')] });
    a('What is the task about Aadhya Varma?', 'lk', { f: ['written consent', 'parents'] });
    a('When is the task to send the Shoot Day 4 call sheet due?', 'lk', { has: [iso('2027-03-03')] });
    a('Is the hotel booking done?', 'lk', { has: [['yes', 'Yes', 'done']] });
    a('Which open tasks are due in February 2027?', 'ls', { f: ['Recce Lighthouse Road', 'Confirm the offshore permit', 'written consent'] });
    a('List the open tasks.', 'ls', { f: ['Confirm the offshore permit', 'Send the call sheet', 'Recce Lighthouse Road', 'Order car batteries', 'written consent', 'wild tracks'] });
    a('When is the deposit for the boat owner due?', 'lk', { has: [iso('2027-02-20')] });
    a('whats left to do before the roof night', 'lk', { f: ['Order car batteries'] });
  }

  // ---------------------------------------------------------------- App Help, Product Guide (knowledge.md)
  {
    const a = mk('help/product-guide');
    a('What is the Parking Lot?', 'ex', { f: ['hatched column', 'set aside'] });
    a('How do I bring a parked card back?', 'hp', { f: ['Restore to Story'] });
    a('What happens when a draft is locked?', 'hp', { f: ['cannot be edited directly', 'revision'] });
    a('What is the Production Source?', 'hp', { f: ['screenplay draft', 'production planning'] });
    a('What are the project roles?', 'hp', { f: ['Owner', 'Editor', 'Commenter', 'Viewer', 'Export-only'] });
    a('Who can see contact details?', 'hp', { f: ['Owners and Editors'] });
    a('How do I get Straft AI?', 'hp', { f: ['Download Straft AI', 'Straft AI'] });
    a('Does Straft work without the internet?', 'hp', { has: [['without an internet', 'offline', 'works without']] });
    a('What does Apply Changes do?', 'hx', { f: ['Proposed Changes', 'Undo'] });
    a('What is the shortcut for Quick Capture?', 'hp', { f: ['Ctrl+Shift+N'] });
    a('What does Day Out of Days show?', 'hp', { f: ['work', 'hold', 'travel'] });
    a('Which formats can the budget be exported in?', 'hp', { f: ['PDF', 'CSV', 'Excel'] });
    a('Which languages does the assistant understand?', 'hp', { f: ['Hindi', 'Telugu', 'Tamil', 'Kannada', 'Malayalam'] });
    a('What does the Timeline tab do?', 'hp', { f: ['Story Day', 'Unassigned'] });
    a('What do I press to take back my last change?', 'hp', { f: ['Ctrl+Z'] });
    a('How do I search the whole project?', 'hp', { f: ['Ctrl+K'] });
    a('Explain how Proposed Changes work.', 'hx', { f: ['Apply Changes', 'Reject', 'Undo'] });
    a('What is the difference between the Beat Board and the Screenplay?', 'hx', { f: ['separate', 'Scene Card'] });
    a('What is a Beat?', 'hp', { f: ['smaller than a Scene Card'] });
    a('Where do deleted items go?', 'hp', { f: ['Recently Deleted'] });
  }

  // ---------------------------------------------------------------- App Help, Settings (knowledge.md)
  {
    const a = mk('help/settings');
    a('What tabs does Settings have?', 'hp', { f: ['Project settings', 'Writing', 'Keyboard shortcuts', 'Templates', 'Straft AI', 'Advanced', 'Appearance'] });
    a('Which Settings tab do I use to switch to dark mode?', 'hp', { f: ['Settings', 'Appearance'] });
    a('What modes can I choose for the appearance?', 'hp', { f: ['Light', 'Dark', 'System'] });
    a('What does the Graphics card setting do?', 'hp', { f: ['Automatic', 'Off'] });
    a('What is in Project settings?', 'hp', { f: ['title', 'logline', 'genre'] });
    a('Where do I remove Straft AI?', 'hp', { f: ['Straft AI'] });
    a('Do keyboard shortcuts change in every project?', 'hp', { f: ['every project'] });
    a('What does the Writing tab have?', 'hp', { f: ['spell checking', 'personal dictionary'] });
    a('Does the theme change the project?', 'hp', { has: [['does not change the project', 'personal', 'not change']] });
    a('Do themes change printed call sheets?', 'hp', { f: ['black on white'] });
    a('How do I import a model file for Straft AI?', 'hp', { f: ['Import a model file'] });
    a('Can I import my own theme?', 'hp', { f: ['My Themes'] });
    a('Can I change the keyboard shortcut for Quick Capture?', 'hp', { f: ['Keyboard shortcuts'] });
  }

  return out;
}
