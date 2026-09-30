//! Deterministic "Suggest Elements" engine (FSD §27, §97). No AI, no network:
//! plain rules over the scene text, so the same scene always yields the same
//! suggestions.
//!
//! - Character cues → Cast (and "(V.O.)" → Sound: voice-over).
//! - Known character names mentioned in action → Cast.
//! - Scene heading → Location / Set (INT/EXT and time of day are scene facts).
//! - Keyword / phrase dictionaries → Props, Vehicles, Animals, Wardrobe,
//!   Special Effects, VFX, Sound, Extras / Background (crowd words) and
//!   Hair / Makeup (blood, scar…).
//!
//! Every candidate carries the matched phrase as evidence and a plain-language
//! confidence ("Likely" / "Possible"), never a numeric score (FSD §27.7).

use openframe_domain::enums::BreakdownCategory as Cat;
use serde::Serialize;
use ts_rs::TS;

pub const LIKELY: &str = "Likely";
pub const POSSIBLE: &str = "Possible";

/// One screenplay element of the scene being analysed.
#[derive(Debug, Clone, Copy)]
pub struct ScriptElement<'a> {
    pub id: &'a str,
    pub element_type: &'a str,
    pub text: &'a str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub category: Cat,
    pub name: String,
    pub key: String,
    pub evidence: String,
    pub confidence: &'static str,
    pub element_id: Option<String>,
    /// UTF-16 offsets of the matched text within the element's text.
    pub span: Option<(i64, i64)>,
}

/// Facts parsed from the scene heading (shown with the scene, not catalog items).
#[derive(Debug, Clone, Default, PartialEq, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct BreakdownSceneFacts {
    /// "INT", "EXT" or "INT/EXT".
    pub int_ext: Option<String>,
    pub set_name: Option<String>,
    pub time_of_day: Option<String>,
}

// ------------------------------------------------------------------ names

/// Normalized identity key for matching names: case/punctuation-insensitive,
/// leading article dropped, last word singularized.
pub fn name_key(s: &str) -> String {
    let lower = s.to_lowercase().replace(['\'', '’'], "");
    let cleaned: String = lower
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    let mut words: Vec<String> = cleaned.split_whitespace().map(|w| w.to_string()).collect();
    if words.len() > 1 && matches!(words[0].as_str(), "the" | "a" | "an") {
        words.remove(0);
    }
    if let Some(last) = words.pop() {
        words.push(singular(&last));
    }
    words.join(" ")
}

fn singular(w: &str) -> String {
    let n = w.len();
    if w == "buses" {
        return "bus".into();
    }
    if n > 4 && w.ends_with("ies") {
        return format!("{}y", &w[..n - 3]);
    }
    if n > 4 && w.ends_with("ves") && !w.ends_with("eves") {
        return format!("{}fe", &w[..n - 3])
            .replace("lfe", "lf")
            .replace("afe", "af");
    }
    if n > 3 && w.ends_with("es") {
        let stem = &w[..n - 2];
        if stem.ends_with('x')
            || stem.ends_with("ch")
            || stem.ends_with("sh")
            || stem.ends_with("ss")
        {
            return stem.to_string();
        }
    }
    if n > 3 && w.ends_with('s') && !w.ends_with("ss") && !w.ends_with("us") && !w.ends_with("is") {
        return w[..n - 1].to_string();
    }
    w.to_string()
}

/// "POLICE STATION" → "Police Station", "ARJUN'S HOUSE" → "Arjun's House".
pub fn title_case(s: &str) -> String {
    s.split_whitespace()
        .map(|w| {
            if w.chars().any(|c| c.is_ascii_digit()) {
                return w.to_string();
            }
            let mut out = String::with_capacity(w.len());
            let mut first = true;
            for c in w.chars() {
                if first && c.is_alphabetic() {
                    out.extend(c.to_uppercase());
                    first = false;
                } else if c == '-' || c == '/' {
                    out.push(c);
                    first = true;
                } else {
                    out.extend(c.to_lowercase());
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------- heading

const TIME_WORDS: &[&str] = &[
    "DAY",
    "NIGHT",
    "MORNING",
    "EVENING",
    "AFTERNOON",
    "DAWN",
    "DUSK",
    "SUNSET",
    "SUNRISE",
    "CONTINUOUS",
    "LATER",
    "MOMENTS LATER",
    "SAME",
    "SAME TIME",
    "NOON",
    "MIDNIGHT",
    "MAGIC HOUR",
    "TWILIGHT",
    "PRE-DAWN",
    "EARLY MORNING",
    "LATE NIGHT",
    "FLASHBACK",
];

pub fn parse_heading(heading: &str) -> BreakdownSceneFacts {
    let h = heading.trim();
    if h.is_empty() {
        return BreakdownSceneFacts::default();
    }
    let upper = h.to_uppercase();
    let prefixes: &[(&str, &str)] = &[
        ("INT./EXT.", "INT/EXT"),
        ("INT/EXT.", "INT/EXT"),
        ("INT./EXT", "INT/EXT"),
        ("INT/EXT", "INT/EXT"),
        ("EXT./INT.", "INT/EXT"),
        ("EXT/INT.", "INT/EXT"),
        ("EXT/INT", "INT/EXT"),
        ("I/E.", "INT/EXT"),
        ("I/E", "INT/EXT"),
        ("INT.", "INT"),
        ("EXT.", "EXT"),
        ("EST.", "EXT"),
        ("INT ", "INT"),
        ("EXT ", "EXT"),
    ];
    let mut int_ext = None;
    let mut rest = h;
    for (p, label) in prefixes {
        if upper.starts_with(p) {
            int_ext = Some(label.to_string());
            rest = &h[p.len()..];
            break;
        }
    }
    // Segments separated by dashes (" - ", " — ", " – ", "--").
    let normalized = rest
        .replace(" — ", " - ")
        .replace(" – ", " - ")
        .replace("—", " - ")
        .replace("--", " - ");
    let mut segments: Vec<String> = normalized
        .split(" - ")
        .map(|s| s.trim().trim_matches('.').trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let mut time = None;
    if segments.len() > 1 {
        let last = segments
            .last()
            .map(|s| s.to_uppercase())
            .unwrap_or_default();
        if TIME_WORDS
            .iter()
            .any(|t| last == *t || last.starts_with(&format!("{t} ")) || last.contains(t))
        {
            time = segments.pop().map(|s| s.to_uppercase());
        }
    } else if let Some(only) = segments.first() {
        let u = only.to_uppercase();
        if int_ext.is_none() && TIME_WORDS.contains(&u.as_str()) {
            time = Some(u);
            segments.clear();
        }
    }
    let set = segments.join(" - ");
    BreakdownSceneFacts {
        int_ext,
        set_name: if set.trim().is_empty() {
            None
        } else {
            Some(title_case(&set))
        },
        time_of_day: time,
    }
}

// ------------------------------------------------------------ dictionaries

/// (pattern words, canonical name or "" for the pattern itself, likely?)
type Dict = &'static [(&'static str, &'static str, bool)];

const PROPS: Dict = &[
    ("gun", "Gun", true),
    ("pistol", "", true),
    ("revolver", "", true),
    ("rifle", "", true),
    ("shotgun", "", true),
    ("knife", "", true),
    ("dagger", "", true),
    ("sword", "", true),
    ("machete", "", true),
    ("phone", "Phone", false),
    ("mobile phone", "Mobile Phone", true),
    ("cellphone", "Phone", true),
    ("smartphone", "Phone", true),
    ("laptop", "", true),
    ("computer", "", false),
    ("folder", "", true),
    ("file", "", false),
    ("envelope", "", true),
    ("letter", "", false),
    ("briefcase", "", true),
    ("suitcase", "", true),
    ("backpack", "", true),
    ("bag", "", false),
    ("wallet", "", true),
    ("purse", "", true),
    ("keys", "Keys", true),
    ("key", "", false),
    ("camera", "", true),
    ("photograph", "", true),
    ("photo", "Photograph", false),
    ("newspaper", "", true),
    ("book", "", false),
    ("notebook", "", true),
    ("diary", "", true),
    ("map", "", true),
    ("torch", "", true),
    ("flashlight", "", true),
    ("lantern", "", true),
    ("candle", "", true),
    ("cigarette", "", true),
    ("lighter", "", true),
    ("matchbox", "", true),
    ("bottle", "", true),
    ("glass", "", false),
    ("cup", "", false),
    ("mug", "", true),
    ("umbrella", "", true),
    ("wristwatch", "Watch", true),
    ("ring", "", false),
    ("necklace", "", true),
    ("handcuffs", "Handcuffs", true),
    ("badge", "", true),
    ("radio", "", false),
    ("walkie-talkie", "Walkie-Talkie", true),
    ("walkie talkie", "Walkie-Talkie", true),
    ("tablet", "", false),
    ("pen", "", false),
    ("document", "", false),
    ("documents", "Documents", false),
    ("cash", "", true),
    ("money", "Cash", false),
    ("coin", "", false),
    ("ticket", "", true),
    ("passport", "", true),
    ("rope", "", true),
    ("chain", "", false),
    ("hammer", "", true),
    ("axe", "", true),
    ("shovel", "", true),
    ("crowbar", "", true),
    ("syringe", "", true),
    ("pills", "Pills", true),
    ("medicine", "", false),
    ("tray", "", false),
    ("basket", "", false),
    ("crate", "", true),
    ("guitar", "", true),
    ("violin", "", true),
    ("drum", "", false),
    ("ball", "", false),
    ("cricket bat", "Cricket Bat", true),
    ("doll", "", true),
    ("toy", "", false),
    ("bouquet", "", true),
    ("flowers", "Flowers", false),
    ("painting", "", false),
    ("mirror", "", false),
    ("clock", "", false),
    ("television", "", false),
    ("tv", "Television", false),
    ("binoculars", "Binoculars", true),
    ("lamp", "", false),
    ("teacup", "", true),
    ("flask", "", true),
    ("cane", "", false),
    ("walking stick", "", true),
    ("helmet", "", false),
];

const VEHICLES: Dict = &[
    ("car", "", true),
    ("taxi", "", true),
    ("cab", "Taxi", false),
    ("bus", "", true),
    ("truck", "", true),
    ("lorry", "", true),
    ("van", "", true),
    ("jeep", "", true),
    ("motorcycle", "", true),
    ("motorbike", "Motorcycle", true),
    ("bike", "", false),
    ("bicycle", "", true),
    ("scooter", "", true),
    ("auto-rickshaw", "Auto-Rickshaw", true),
    ("autorickshaw", "Auto-Rickshaw", true),
    ("rickshaw", "", true),
    ("ambulance", "", true),
    ("police car", "Police Car", true),
    ("police jeep", "Police Jeep", true),
    ("train", "", true),
    ("boat", "", true),
    ("ship", "", true),
    ("helicopter", "", true),
    ("plane", "", false),
    ("airplane", "", true),
    ("tractor", "", true),
    ("tram", "", true),
    ("suv", "SUV", true),
    ("sedan", "", true),
    ("limousine", "", true),
    ("pickup truck", "Pickup Truck", true),
    ("fire engine", "Fire Engine", true),
];

const ANIMALS: Dict = &[
    ("dog", "", true),
    ("puppy", "", true),
    ("cat", "", true),
    ("kitten", "", true),
    ("horse", "", true),
    ("cow", "", true),
    ("goat", "", true),
    ("sheep", "", true),
    ("bird", "", false),
    ("crow", "", true),
    ("pigeon", "", true),
    ("parrot", "", true),
    ("chicken", "", false),
    ("rooster", "", true),
    ("hen", "", true),
    ("snake", "", true),
    ("fish", "", false),
    ("rat", "", true),
    ("mouse", "", false),
    ("monkey", "", true),
    ("elephant", "", true),
    ("buffalo", "", true),
    ("donkey", "", true),
    ("camel", "", true),
    ("duck", "", false),
    ("owl", "", true),
    ("stray dog", "Stray Dog", true),
];

const WARDROBE: Dict = &[
    ("coat", "", true),
    ("raincoat", "", true),
    ("overcoat", "", true),
    ("jacket", "", true),
    ("uniform", "", true),
    ("suit", "", false),
    ("tie", "", false),
    ("dress", "", false),
    ("saree", "", true),
    ("sari", "Saree", true),
    ("shirt", "", false),
    ("t-shirt", "T-Shirt", true),
    ("jeans", "Jeans", true),
    ("trousers", "Trousers", true),
    ("skirt", "", true),
    ("hat", "", true),
    ("cap", "", false),
    ("scarf", "", true),
    ("shawl", "", true),
    ("gloves", "Gloves", true),
    ("boots", "Boots", true),
    ("shoes", "Shoes", false),
    ("sandals", "Sandals", true),
    ("sunglasses", "Sunglasses", true),
    ("spectacles", "Spectacles", true),
    ("apron", "", true),
    ("mask", "", false),
    ("hoodie", "", true),
    ("veil", "", true),
    ("turban", "", true),
    ("costume", "", true),
    ("wedding dress", "Wedding Dress", true),
    ("pajamas", "Pajamas", true),
    ("lab coat", "Lab Coat", true),
    ("kurta", "", true),
    ("dhoti", "", true),
    ("lungi", "", true),
];

const SPECIAL_EFFECTS: Dict = &[
    ("rain", "Rain", true),
    ("raining", "Rain", true),
    ("downpour", "Rain", true),
    ("drizzle", "Rain", true),
    ("snow", "", true),
    ("fog", "", true),
    ("mist", "", true),
    ("smoke", "", true),
    ("fire", "", false),
    ("flames", "Fire", true),
    ("blaze", "Fire", true),
    ("explosion", "", true),
    ("explodes", "Explosion", true),
    ("sparks", "Sparks", true),
    ("wind", "", false),
    ("gust", "Wind", false),
    ("shatters", "Breaking Glass", true),
    ("breaking glass", "Breaking Glass", true),
    ("squib", "", true),
    ("dust storm", "Dust Storm", true),
    ("steam", "", false),
];

const VFX: Dict = &[
    ("hologram", "", true),
    ("spaceship", "", true),
    ("monster", "", false),
    ("ghost", "", false),
    ("portal", "", true),
    ("teleports", "Teleport", true),
    ("levitates", "Levitation", true),
    ("levitating", "Levitation", true),
    ("transforms", "Transformation", false),
    ("morphs", "Morph", true),
    ("disintegrates", "Disintegration", true),
    ("green screen", "Green Screen", true),
    ("cgi", "CGI", true),
    ("screen replacement", "Screen Replacement", true),
    ("glowing", "Glow Effect", false),
    ("invisible", "Invisibility", false),
    ("time freezes", "Time Freeze", true),
];

const SOUND: Dict = &[
    ("gunshot", "", true),
    ("gunfire", "Gunshot", true),
    ("siren", "", true),
    ("thunder", "", true),
    ("phone rings", "Phone Ringing", true),
    ("ringing", "Ringing", false),
    ("music", "", false),
    ("song", "", false),
    ("sings", "Singing", true),
    ("singing", "Singing", true),
    ("scream", "", true),
    ("screams", "Scream", true),
    ("knock", "Knocking", true),
    ("knocks", "Knocking", true),
    ("knocking", "Knocking", true),
    ("doorbell", "", true),
    ("whistle", "", false),
    ("honks", "Horn", true),
    ("honking", "Horn", true),
    ("horn", "", false),
    ("alarm", "", true),
    ("footsteps", "Footsteps", true),
    ("explosion", "", true),
    ("echo", "", false),
    ("train whistle", "Train Whistle", true),
    ("radio plays", "Radio Playback", true),
    ("announcement", "", false),
];

/// Crowd words → Extras / Background.
const EXTRAS: Dict = &[
    ("crowd", "Crowd", true),
    ("crowds", "Crowd", true),
    ("passersby", "Passersby", true),
    ("passers-by", "Passersby", true),
    ("passengers", "Passengers", true),
    ("commuters", "Commuters", true),
    ("pedestrians", "Pedestrians", true),
    ("onlookers", "Onlookers", true),
    ("bystanders", "Bystanders", true),
    ("customers", "Customers", true),
    ("shoppers", "Shoppers", true),
    ("students", "Students", true),
    ("villagers", "Villagers", true),
    ("soldiers", "Soldiers", true),
    ("guards", "Guards", true),
    ("policemen", "Policemen", true),
    ("constables", "Constables", true),
    ("police officers", "Police Officers", true),
    ("nurses", "Nurses", true),
    ("waiters", "Waiters", true),
    ("dancers", "Dancers", true),
    ("audience", "Audience", true),
    ("spectators", "Spectators", true),
    ("mourners", "Mourners", true),
    ("protesters", "Protesters", true),
    ("workers", "Workers", true),
    ("patrons", "Patrons", true),
    ("guests", "Guests", true),
    ("vendors", "Vendors", true),
    ("travellers", "Travellers", true),
    ("travelers", "Travellers", true),
    ("people", "Background People", false),
    ("families", "Families", false),
    ("children play", "Children", true),
    ("porters", "Porters", true),
];

const HAIR_MAKEUP: Dict = &[
    ("blood", "Blood", true),
    ("bloody", "Blood", true),
    ("bloodied", "Blood", true),
    ("bleeding", "Blood", true),
    ("bleeds", "Blood", true),
    ("scar", "Scar", true),
    ("scarred", "Scar", true),
    ("bruise", "Bruise", true),
    ("bruised", "Bruise", true),
    ("wound", "Wound", true),
    ("wounded", "Wound", true),
    ("black eye", "Black Eye", true),
    ("tattoo", "Tattoo", true),
    ("tattooed", "Tattoo", true),
    ("sweat", "Sweat", false),
    ("sweating", "Sweat", false),
    ("wig", "Wig", true),
    ("beard", "Beard", false),
    ("moustache", "Moustache", false),
    ("mustache", "Moustache", false),
    ("makeup", "Makeup", false),
    ("make-up", "Makeup", false),
    ("lipstick", "Lipstick", true),
    ("grey hair", "Grey Hair", true),
    ("gray hair", "Grey Hair", true),
    ("bald", "Bald", false),
    ("soot", "Soot", true),
    ("burns", "Burns", false),
    ("burn marks", "Burns", true),
    ("wrinkled", "Aged Makeup", false),
    ("stubble", "Stubble", false),
    ("pale", "Pale Makeup", false),
    ("dirt-streaked", "Dirt", true),
    ("mud-streaked", "Mud", true),
    ("tear-streaked", "Tear Streaks", true),
];

/// Descriptors kept in front of a matched Prop/Wardrobe/Vehicle noun ("red folder").
const DESCRIPTORS: &[&str] = &[
    "red", "black", "white", "blue", "green", "yellow", "grey", "gray", "brown", "silver", "gold",
    "golden", "pink", "orange", "purple", "old", "new", "wet", "broken", "bloody", "torn", "rusty",
    "leather", "wooden", "plastic", "paper", "metal", "antique", "vintage", "dirty", "muddy",
    "battered", "shiny", "cracked", "empty",
];

fn dictionaries() -> [(Cat, Dict); 9] {
    [
        (Cat::Extras, EXTRAS),
        (Cat::Props, PROPS),
        (Cat::Wardrobe, WARDROBE),
        (Cat::Vehicles, VEHICLES),
        (Cat::HairMakeup, HAIR_MAKEUP),
        (Cat::SpecialEffects, SPECIAL_EFFECTS),
        (Cat::Vfx, VFX),
        (Cat::Sound, SOUND),
        (Cat::Animals, ANIMALS),
    ]
}

// --------------------------------------------------------------- tokenize

#[derive(Debug)]
struct Tok {
    start: usize,
    end: usize,
    lower: String,
    sentence: usize,
    upper_case: bool,
}

fn tokenize(text: &str) -> Vec<Tok> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut toks = Vec::new();
    let mut sentence = 0usize;
    let mut i = 0usize;
    while i < chars.len() {
        let (b, c) = chars[i];
        if c.is_alphanumeric() {
            let start = b;
            let mut j = i + 1;
            while j < chars.len() {
                let cj = chars[j].1;
                let joiner = matches!(cj, '\'' | '’' | '-')
                    && chars
                        .get(j + 1)
                        .map(|x| x.1.is_alphanumeric())
                        .unwrap_or(false);
                if cj.is_alphanumeric() || joiner {
                    j += 1;
                } else {
                    break;
                }
            }
            let end = chars.get(j).map(|x| x.0).unwrap_or(text.len());
            let raw = &text[start..end];
            toks.push(Tok {
                start,
                end,
                lower: raw.to_lowercase().replace('’', "'"),
                sentence,
                upper_case: raw.chars().any(|c| c.is_alphabetic())
                    && !raw.chars().any(|c| c.is_lowercase()),
            });
            i = j;
        } else {
            if matches!(c, '.' | '!' | '?' | ';' | '\n') {
                sentence += 1;
            }
            i += 1;
        }
    }
    toks
}

fn word_matches(tok: &str, pat: &str) -> bool {
    let tok = tok.strip_suffix("'s").unwrap_or(tok);
    if tok == pat {
        return true;
    }
    if tok.len() > pat.len() + 3
        || !tok.starts_with(&pat[..pat.len().saturating_sub(2).max(1).min(pat.len())])
    {
        return false;
    }
    tok == format!("{pat}s")
        || tok == format!("{pat}es")
        || (pat.ends_with('y') && tok == format!("{}ies", &pat[..pat.len() - 1]))
        || (pat.ends_with("fe") && tok == format!("{}ves", &pat[..pat.len() - 2]))
        || (pat.ends_with('f') && tok == format!("{}ves", &pat[..pat.len() - 1]))
}

fn utf16_offset(text: &str, byte: usize) -> i64 {
    text[..byte].encode_utf16().count() as i64
}

/// Up to two words before and two after the match, within the sentence,
/// with closing punctuation kept: “carrying a pistol.”
fn evidence(text: &str, toks: &[Tok], first: usize, last: usize) -> String {
    let sentence = toks[first].sentence;
    let mut a = first;
    while a > 0 && first - (a - 1) <= 2 && toks[a - 1].sentence == sentence {
        a -= 1;
    }
    let mut z = last;
    while z + 1 < toks.len() && (z + 1) - last <= 2 && toks[z + 1].sentence == sentence {
        z += 1;
    }
    let mut end = toks[z].end;
    if let Some(c) = text[end..]
        .chars()
        .next()
        .filter(|c| matches!(c, '.' | '!' | '?'))
    {
        end += c.len_utf8();
    }
    text[toks[a].start..end]
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

// ---------------------------------------------------------------- engine

/// Analyse one scene. `known_characters` are upper-case character names known
/// in the production source (cues elsewhere, Story characters).
pub fn suggest(
    heading: &str,
    elements: &[ScriptElement<'_>],
    known_characters: &[String],
) -> Vec<Candidate> {
    let mut out: Vec<Candidate> = Vec::new();
    let push = |c: Candidate, out: &mut Vec<Candidate>| {
        if c.key.is_empty()
            || out
                .iter()
                .any(|o| o.category == c.category && o.key == c.key)
        {
            return;
        }
        out.push(c);
    };

    // Cast from character cues (and voice-over → Sound).
    let mut cue_names: Vec<String> = Vec::new();
    for e in elements.iter().filter(|e| e.element_type == "character") {
        let Some(name) = super::script::cue_name(e.text) else {
            continue;
        };
        let display = title_case(&name);
        let span = Some((0, e.text.encode_utf16().count() as i64));
        push(
            Candidate {
                category: Cat::Cast,
                key: name_key(&display),
                name: display.clone(),
                evidence: e.text.trim().to_string(),
                confidence: LIKELY,
                element_id: Some(e.id.to_string()),
                span,
            },
            &mut out,
        );
        if super::script::cue_is_voice_over(e.text) {
            let n = format!("Voice-over — {display}");
            push(
                Candidate {
                    category: Cat::Sound,
                    key: name_key(&n),
                    name: n,
                    evidence: e.text.trim().to_string(),
                    confidence: LIKELY,
                    element_id: Some(e.id.to_string()),
                    span,
                },
                &mut out,
            );
        }
        cue_names.push(name);
    }

    // Location / Set from the heading.
    let heading_el = elements.iter().find(|e| e.element_type == "scene_heading");
    let facts = parse_heading(heading);
    if let Some(set) = &facts.set_name {
        push(
            Candidate {
                category: Cat::Location,
                key: name_key(set),
                name: set.clone(),
                evidence: heading.trim().to_string(),
                confidence: LIKELY,
                element_id: heading_el.map(|e| e.id.to_string()),
                span: None,
            },
            &mut out,
        );
    }

    // Action / shot text.
    let known: Vec<(String, Vec<String>)> = known_characters
        .iter()
        .filter(|n| !cue_names.contains(n))
        .map(|n| {
            (
                n.clone(),
                n.to_lowercase()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect::<Vec<_>>(),
            )
        })
        .filter(|(_, w)| !w.is_empty())
        .collect();
    let dicts = dictionaries();
    for e in elements
        .iter()
        .filter(|e| matches!(e.element_type, "action" | "shot"))
    {
        let text = e.text;
        let toks = tokenize(text);
        for i in 0..toks.len() {
            // Known characters mentioned in action.
            for (name, words) in &known {
                if i + words.len() <= toks.len()
                    && words.iter().enumerate().all(|(k, w)| {
                        toks[i + k]
                            .lower
                            .strip_suffix("'s")
                            .unwrap_or(&toks[i + k].lower)
                            == w
                    })
                {
                    let last = i + words.len() - 1;
                    let display = title_case(name);
                    push(
                        Candidate {
                            category: Cat::Cast,
                            key: name_key(&display),
                            name: display,
                            evidence: evidence(text, &toks, i, last),
                            confidence: if toks[i].upper_case { LIKELY } else { POSSIBLE },
                            element_id: Some(e.id.to_string()),
                            span: Some((
                                utf16_offset(text, toks[i].start),
                                utf16_offset(text, toks[last].end),
                            )),
                        },
                        &mut out,
                    );
                }
            }
            // Dictionaries.
            for (cat, dict) in dicts.iter() {
                for (pattern, canonical, likely) in dict.iter() {
                    let words: Vec<&str> = pattern.split(' ').collect();
                    if i + words.len() > toks.len() {
                        continue;
                    }
                    let last = i + words.len() - 1;
                    let ok = words.iter().enumerate().all(|(k, w)| {
                        let t = &toks[i + k];
                        t.sentence == toks[i].sentence
                            && if i + k == last {
                                word_matches(&t.lower, w)
                            } else {
                                t.lower == *w
                            }
                    });
                    if !ok {
                        continue;
                    }
                    let base = if canonical.is_empty() {
                        title_case(pattern)
                    } else {
                        canonical.to_string()
                    };
                    let mut first = i;
                    let mut name = base;
                    if matches!(cat, Cat::Props | Cat::Wardrobe | Cat::Vehicles)
                        && i > 0
                        && toks[i - 1].sentence == toks[i].sentence
                        && DESCRIPTORS.contains(&toks[i - 1].lower.as_str())
                        && !name.to_lowercase().starts_with(&toks[i - 1].lower)
                    {
                        first = i - 1;
                        name = format!("{} {}", title_case(&toks[i - 1].lower), name);
                    }
                    push(
                        Candidate {
                            category: *cat,
                            key: name_key(&name),
                            name,
                            evidence: evidence(text, &toks, first, last),
                            confidence: if *likely { LIKELY } else { POSSIBLE },
                            element_id: Some(e.id.to_string()),
                            span: Some((
                                utf16_offset(text, toks[first].start),
                                utf16_offset(text, toks[last].end),
                            )),
                        },
                        &mut out,
                    );
                }
            }
        }
    }

    // A qualified name ("Red Folder") supersedes the bare noun ("Folder") in the same category.
    let snapshot = out.clone();
    out.retain(|c| {
        !snapshot.iter().any(|o| {
            o.category == c.category && o.key != c.key && o.key.ends_with(&format!(" {}", c.key))
        })
    });
    let order = |c: &Cat| Cat::ALL.iter().position(|x| x == c).unwrap_or(usize::MAX);
    out.sort_by_key(|c| order(&c.category));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn el<'a>(id: &'a str, t: &'a str, text: &'a str) -> ScriptElement<'a> {
        ScriptElement {
            id,
            element_type: t,
            text,
        }
    }

    #[test]
    fn headings_parse_into_facts() {
        let f = parse_heading("INT. POLICE STATION — NIGHT");
        assert_eq!(f.int_ext.as_deref(), Some("INT"));
        assert_eq!(f.set_name.as_deref(), Some("Police Station"));
        assert_eq!(f.time_of_day.as_deref(), Some("NIGHT"));
        let f = parse_heading("EXT./INT. ARJUN'S HOUSE - KITCHEN - DAY");
        assert_eq!(f.int_ext.as_deref(), Some("INT/EXT"));
        assert_eq!(f.set_name.as_deref(), Some("Arjun's House - Kitchen"));
        assert_eq!(f.time_of_day.as_deref(), Some("DAY"));
        let f = parse_heading("EXT. BUS STOP (NH-16)");
        assert_eq!(f.set_name.as_deref(), Some("Bus Stop (NH-16)"));
        assert_eq!(f.time_of_day, None);
    }

    #[test]
    fn keys_normalize_case_plural_and_articles() {
        assert_eq!(name_key("The Red Folders"), name_key("red folder"));
        assert_eq!(name_key("Pistols"), "pistol");
        assert_eq!(name_key("Arjun's House"), "arjuns house");
        assert_eq!(name_key("Boxes"), "box");
        assert_eq!(name_key("Knives"), "knife");
        assert_eq!(name_key("Glass"), "glass");
    }

    #[test]
    fn the_example_scene_yields_the_expected_suggestions() {
        let els = [
            el("h", "scene_heading", "INT. POLICE STATION — NIGHT"),
            el(
                "a1",
                "action",
                "Arjun enters carrying a pistol. Rain drips from his coat.",
            ),
            el(
                "a2",
                "action",
                "He places the RED FOLDER on the desk. A crowd gathers outside.",
            ),
            el("c1", "character", "MEERA"),
            el(
                "d1",
                "dialogue",
                "You said you would never come back with a knife.",
            ),
            el("c2", "character", "ARJUN (V.O.)"),
            el(
                "a3",
                "action",
                "Blood on his sleeve. A siren wails. His dog barks at the jeep.",
            ),
        ];
        let s = suggest(
            "INT. POLICE STATION — NIGHT",
            &els,
            &["ARJUN".into(), "MEERA".into(), "RAVI".into()],
        );
        let find = |cat: Cat, name: &str| {
            s.iter()
                .find(|c| c.category == cat && c.name == name)
                .cloned()
        };
        let pistol = find(Cat::Props, "Pistol").expect("pistol");
        assert_eq!(pistol.evidence, "carrying a pistol.");
        assert_eq!(pistol.confidence, LIKELY);
        assert_eq!(pistol.element_id.as_deref(), Some("a1"));
        let (a, b) = pistol.span.unwrap();
        assert_eq!(
            &"Arjun enters carrying a pistol. Rain drips from his coat."[a as usize..b as usize],
            "pistol"
        );
        assert!(
            find(Cat::Props, "Red Folder").is_some(),
            "descriptor kept: {s:?}"
        );
        assert!(find(Cat::Props, "Folder").is_none(), "bare noun superseded");
        assert!(find(Cat::Cast, "Meera").is_some());
        assert!(
            find(Cat::Cast, "Arjun").is_some(),
            "cue with V.O. extension"
        );
        assert!(find(Cat::Sound, "Voice-over — Arjun").is_some());
        assert!(find(Cat::Location, "Police Station").is_some());
        assert!(find(Cat::SpecialEffects, "Rain").is_some());
        assert!(find(Cat::Wardrobe, "Coat").is_some());
        assert!(find(Cat::Extras, "Crowd").is_some());
        assert!(find(Cat::HairMakeup, "Blood").is_some());
        assert!(find(Cat::Sound, "Siren").is_some());
        assert!(find(Cat::Animals, "Dog").is_some());
        assert!(find(Cat::Vehicles, "Jeep").is_some());
        assert!(
            find(Cat::Props, "Knife").is_none(),
            "dialogue is not scanned"
        );
        assert!(find(Cat::Cast, "Ravi").is_none());
        // Deterministic and grouped in category order.
        assert_eq!(
            s,
            suggest(
                "INT. POLICE STATION — NIGHT",
                &els,
                &["ARJUN".into(), "MEERA".into(), "RAVI".into()]
            )
        );
        let idx: Vec<usize> = s
            .iter()
            .map(|c| Cat::ALL.iter().position(|x| *x == c.category).unwrap())
            .collect();
        assert!(idx.windows(2).all(|w| w[0] <= w[1]));
    }

    #[test]
    fn known_character_mentioned_only_in_action() {
        let els = [el("a", "action", "RAVI (30s) waits by the car.")];
        let s = suggest("EXT. STREET - DAY", &els, &["RAVI".into()]);
        let ravi = s.iter().find(|c| c.category == Cat::Cast).unwrap();
        assert_eq!(ravi.name, "Ravi");
        assert_eq!(ravi.confidence, LIKELY, "introduced in caps");
        assert!(
            s.iter()
                .any(|c| c.category == Cat::Vehicles && c.name == "Car")
        );
        assert!(
            s.iter()
                .any(|c| c.category == Cat::Location && c.name == "Street")
        );
    }

    #[test]
    fn plurals_and_word_boundaries() {
        let els = [el(
            "a",
            "action",
            "Two pistols lie on the carpet. Scarves everywhere; a scary cartoon.",
        )];
        let s = suggest("", &els, &[]);
        assert!(s.iter().any(|c| c.name == "Pistol"));
        assert!(!s.iter().any(|c| c.name == "Car"), "carpet is not a car");
        assert!(!s.iter().any(|c| c.name == "Scar"), "scary is not a scar");
    }
}
