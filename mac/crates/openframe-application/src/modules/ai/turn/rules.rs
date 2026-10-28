//! The code rules of the answer-type read (spec 5.3) and the length class.
//!
//! Rules are English only; any other language falls through to the nearest-neighbour read. Names
//! never come from a word list here: whether a message names something comes from the resolver.

use std::sync::LazyLock;

use regex::Regex;

use super::{AnswerType, LengthClass};
use crate::modules::ai::index::{How, Resolved};

/// Filler a request may start with ("ok bro, can you ..."), so `^` rules still see the verb.
macro_rules! pre {
    () => {
        r"^(?:\W*(?:please|pls|can you|could you|would you|kindly|ok|okay|bro|hey|hi|so|now|then|and|just|go ahead and)\b\W*)*"
    };
}

macro_rules! re {
    ($p:expr) => {{
        static R: LazyLock<Regex> = LazyLock::new(|| Regex::new($p).expect("valid pattern"));
        &*R
    }};
}

/// What the rules found.
#[derive(Debug, PartialEq)]
pub(super) enum Verdict {
    /// A strong rule named the type.
    Type(AnswerType),
    /// A correction, complaint or bare title: the previous type.
    Inherit(AnswerType),
    /// Social or angry words with no request and no name.
    Chat,
    /// The rules are not sure. `angry` tells the later steps never to read a change or a count.
    Open { angry: bool },
}

/// What the message names, from the resolver. A screenplay taken from the focus or because it is
/// the only one is not named by the message.
#[derive(Debug, Default, Clone, Copy)]
pub(super) struct Mentions {
    pub scene: bool,
    /// Distinct scenes named by number.
    pub scenes: usize,
    pub file: bool,
    pub other: bool,
}

impl Mentions {
    pub fn of(r: &Resolved) -> Self {
        let named = |h: &crate::modules::ai::index::Hit| h.how != How::Pronoun;
        let mut scenes: Vec<u32> = r
            .scenes
            .iter()
            .filter(|s| s.how != How::Pronoun)
            .map(|s| s.ordinal)
            .collect();
        scenes.sort_unstable();
        scenes.dedup();
        Self {
            scene: !scenes.is_empty() || !r.not_found.is_empty(),
            scenes: scenes.len(),
            file: r.files.iter().any(named),
            other: r
                .screenplays
                .iter()
                .any(|h| matches!(h.how, How::Exact | How::Alias | How::Fuzzy(_)))
                || r.persons.iter().any(named)
                || r.characters.iter().any(named)
                || r.locations.iter().any(named)
                || r.acts.iter().any(named)
                || r.days.iter().any(named)
                || r.budget_lines.iter().any(named)
                || r.others.iter().any(|(_, h)| named(h)),
        }
    }

    pub fn any(self) -> bool {
        self.scene || self.file || self.other
    }
}

/// Lower case, straight quotes, and the complaints "you dont open the right scene" taken out: a
/// verb the person says the assistant failed to do is not a request.
fn clean(text: &str) -> String {
    let t = text.to_lowercase().replace(['’', '‘'], "'");
    drop_complaints(&t)
}

/// Removes "you [still] dont X Y", "you do not X" and "why cant you X Y": the negation, then up to
/// four words of the action that was not done, ending before the next "you" or after a sentence end.
fn drop_complaints(t: &str) -> String {
    let toks: Vec<&str> = t.split_whitespace().collect();
    fn bare(w: &str) -> &str {
        w.trim_matches(|c: char| !c.is_alphanumeric() && c != '\'')
    }
    let is_neg = |w: &str| {
        matches!(
            bare(w),
            "don't"
                | "dont"
                | "didn't"
                | "didnt"
                | "can't"
                | "cant"
                | "cannot"
                | "never"
                | "won't"
                | "wont"
                | "couldn't"
                | "couldnt"
        )
    };
    let is_you = |w: &str| matches!(bare(w), "you" | "u");
    let mut drop = vec![false; toks.len()];
    let mut i = 0;
    while i < toks.len() {
        let mut after_negation = None;
        if is_you(toks[i]) {
            let mut j = i + 1;
            if j < toks.len() && matches!(bare(toks[j]), "still" | "even" | "just") {
                j += 1;
            }
            if j < toks.len() && is_neg(toks[j]) {
                after_negation = Some(j + 1);
            } else if j + 1 < toks.len()
                && matches!(bare(toks[j]), "do" | "did" | "can")
                && bare(toks[j + 1]) == "not"
            {
                after_negation = Some(j + 2);
            }
        } else if bare(toks[i]) == "why"
            && i + 2 < toks.len()
            && is_neg(toks[i + 1])
            && is_you(toks[i + 2])
        {
            after_negation = Some(i + 3);
        }
        let Some(mut k) = after_negation else {
            i += 1;
            continue;
        };
        let mut taken = 0;
        while k < toks.len() && taken < 4 && !is_you(toks[k]) && bare(toks[k]) != "why" {
            let ends = toks[k].ends_with([',', '.', ';', '!', '?']);
            k += 1;
            taken += 1;
            if ends {
                break;
            }
        }
        drop[i..k].iter_mut().for_each(|d| *d = true);
        i = k;
    }
    toks.iter()
        .zip(&drop)
        .filter(|(_, d)| !**d)
        .map(|(t, _)| *t)
        .collect::<Vec<_>>()
        .join(" ")
}

fn words(t: &str) -> usize {
    t.split_whitespace().count()
}

pub(super) fn decide(text: &str, resolved: &Resolved, previous: Option<AnswerType>) -> Verdict {
    let m = clean(text);
    let men = Mentions::of(resolved);
    let repair = re!(
        r"\b(?:i\s+(?:just\s+|literally\s+|already\s+|clearly\s+)?(?:asked|said|told|mean|meant|wanted|was asking|am asking|am talking|was talking)|i\s+(?:didn't|did not)\s+ask|that is the question|that's the question|not what i|how many times|over \d+ times|you\s+(?:have\s+|had\s+|just\s+)?(?:incorrectly\s+|again\s+|still\s+|wrongly\s+)?(?:opened|gave|showed|listed|answered|said|told|returned|picked)|wrong (?:one|scene|answer|file|screenplay)|not that one)\b"
    )
    .is_match(&m);
    // Two scenes in a complaint ("you opened 41, I asked for 42") are not a comparison.
    let named = if repair {
        Mentions {
            scenes: men.scenes.min(1),
            ..men
        }
    } else {
        men
    };
    if repair {
        // "I asked for X" states what was wanted: read what follows the phrase.
        if let Some(t) = wanted_tail(&m).and_then(|tail| strong(tail, named)) {
            return Verdict::Type(t);
        }
    }
    if let Some(t) = strong(&m, named) {
        return Verdict::Type(t);
    }
    // Anger and thanks that ask for nothing and name nothing are chat, even beside "I asked".
    if social(&m) && !request(&m) && !men.any() {
        return Verdict::Chat;
    }
    if repair {
        return previous.map_or(Verdict::Open { angry: angry(&m) }, Verdict::Inherit);
    }
    // A bare title or scene that answers the last question ("LOW TIDE RADIO", "the feature").
    if men.any()
        && words(&m) <= 8
        && !re!(r"\b(?:what|who|how|which|when|where|why)\b|\?").is_match(&m)
        && let Some(p) = previous
    {
        return Verdict::Inherit(p);
    }
    Verdict::Open { angry: angry(&m) }
}

/// What follows the first "I asked", "I wanted", "talking about" and the like; failing that, what
/// follows the last "but" or "instead" ("I did not ask for that, but explaining scene 35").
fn wanted_tail(m: &str) -> Option<&str> {
    re!(r"\b(?:asked|wanted|meant|asking|talking about|said|told)\b")
        .find(m)
        .or_else(|| re!(r"\b(?:but|instead|rather)\b").find_iter(m).last())
        .map(|f| &m[f.end()..])
}

fn social(m: &str) -> bool {
    re!(
        r"\b(?:fuck\w*|shit\w*|hell|damn|crap|stupid|idiot\w*|useless|worst|hopeless|garbage|rubbish|uninstall\w*|thanks?|thank you|thx|hello|hi|hey|good (?:morning|evening|afternoon|night)|cool|great|nice|awesome|perfect|okay|ok|bye|goodbye|sorry|never ?mind|whatever|wow|annoying|waste of time)\b"
    )
    .is_match(m)
}

fn angry(m: &str) -> bool {
    re!(r"\b(?:fuck\w*|shit\w*|hell|damn|crap|stupid|idiot\w*|useless|worst|hopeless|garbage|rubbish|uninstall\w*)\b")
        .is_match(m)
}

/// Words that make a message a request even beside social words.
fn request(m: &str) -> bool {
    re!(
        r"\b(?:explain\w*|list|find|search|how many|how much|what about|how about|contact|phone|email|budget|schedule|scene \d+|who plays|when is|where|cast|crew|props?|locations?|characters?|acts?|story|script|screenplay|call ?sheet|cost|files?|draft|add|create|delete|remove|rename)\b"
    )
    .is_match(m)
}

/// The strong rules, in order. `m` is cleaned text.
pub(super) fn strong(m: &str, men: Mentions) -> Option<AnswerType> {
    use AnswerType::*;
    let counting = re!(r"\bhow many\b(?: times)?")
        .find_iter(m)
        .any(|f| !f.as_str().ends_with("times"));
    if !counting
        && re!(
            r"\b(?:relationships?|related|relation|relations|bonds?|connected|connection|connections|rivalry|rivals|get along|know each other|ties|linked|friends or enemies)\b"
        )
        .is_match(m)
        && !re!(r"\binternet\b").is_match(m)
    {
        return Some(Relation);
    }
    if counting
        || re!(r"\bnumber of\b|\bcount\b|\bcounting\b|\btotal (?:number|count)\b").is_match(m)
    {
        return Some(Count);
    }
    if re!(
        r"\b(?:vs\.?|versus|compare[ds]?|comparison|difference|differences|differ|differs|different from|over budget|under budget|over or under|ahead or behind|cheaper|more expensive)\b|\bplanned\b[^.?!]*\bactual\b|\bactual\b[^.?!]*\bplanned\b|\bmore than (?:we |i )?(?:planned|thought|expected|estimated)\b|\b(?:longer|shorter) than\b"
    )
    .is_match(m)
    {
        return Some(Compare);
    }
    // "open" as a state, not a request: "what tasks are open", "still open", "open tasks".
    let open_state = re!(
        r"\b(?:is|are|was|were|be|still|remain|remains|stay|stays)\s+(?:still\s+)?open(?:ed)?\b"
    )
    .is_match(m)
        || re!(concat!(
            pre!(),
            r"open\s+(?:tasks|items|issues|warnings|questions|problems|points|threads|comments|todos?)\b"
        ))
        .is_match(m);
    let screen = re!(
        r"\b(?:screen|page|tab|board|vault|settings|workspace|view|section|panel|dashboard|calendar)\b"
    )
    .is_match(m);
    let nav_verb = re!(concat!(
        pre!(),
        r"(?:open|go to|goto|take me to|jump to|switch to|bring up|pull up|navigate to|head to)\b"
    ))
    .is_match(m);
    let show_screen = re!(concat!(pre!(), r"show me the\b")).is_match(m);
    let show_item = re!(concat!(pre!(), r"show me\b")).is_match(m) && (men.scene || men.file);
    let target = men.any()
        || re!(
            r"\b(?:scene \d+|budget|schedule|settings|call ?sheets?|cast|crew|production|story ?board|beat ?board|idea vault|vault|characters?|locations?|files?|notes?|drafts?|screen|page|tab|board|map|list|tasks?|dashboard|calendar|breakdown|screenplay|script|story|scenes?|acts?|day \d+|exports?)\b"
        )
        .is_match(m);
    if (nav_verb && !open_state && target) || (show_screen && screen) || show_item {
        return Some(Navigate);
    }
    if !open_state
        && (re!(
            r"\bopen(?:ed|ing)?\b[^.?!]*\b(?:scene\s*\d+|\S+\.(?:png|jpe?g|gif|pdf|docx?|txt|mp[34]|fdx|fountain))"
        )
        .is_match(m)
            // "scene 42 of the teaser, open it": open as the verb of a clause beside a name.
            || (men.any()
                && re!(r"(?:^|[,;.:]\s*|\b(?:please|pls|then|and|now|can you|could you|just|also)\s+)open\b")
                    .is_match(m)))
    {
        return Some(Navigate);
    }
    if open_state && re!(r"\b(?:what|which|who|show|list|any|are there|give)\b").is_match(m) {
        return Some(List);
    }
    if re!(
        r"\bhow (?:do|can|could|should|would) (?:i|we) (?:export|import|print|save|share|invite|change|set|add|create|delete|undo|redo|find|open|use|turn|switch|install|update|back ?up|restore|rename|move|duplicate|sign|log|reset|download|connect|start|stop|attach|upload|enable|disable|get)\b|\bhow to\b|\bwhere (?:do|can|could|would) (?:i|we) (?:find|see|get|change|set|go|look)\b|\bwhere (?:is|are) the [\w ]{0,30}(?:button|menu|setting|settings|option|backups?)\b|\bwhere (?:are|is) (?:my|the) [\w ]{0,20}(?:kept|stored|saved)\b|\bcan (?:i|we) (?:use|import|export|work|share|print|change|set|invite|undo|turn|switch|download|install)\b|\bis there a way\b|\bwhat does the [\w ]{0,25} (?:button|screen|board|tab) do\b|\bwhat (?:is|are|does) the (?:beat board|story board|idea vault|parking lot|quick capture|production source|project roles)\b|\b(?:the ai|the assistant|openframe|this app|the app)\b[^.?!]*\b(?:internet|offline|privacy|private|upload\w*|sent|cloud|online)\b|\b(?:is|are) my (?:project|data|files|script)\b[^.?!]*\b(?:sent|uploaded|shared|private|safe)\b|\bdoes (?:the ai|the assistant|openframe|the app) need\b"
    )
    .is_match(m)
    {
        return Some(Help);
    }
    if re!(concat!(
        pre!(),
        r"(?:describe|summari[sz]e|recap|break down|tell me (?:about|more about)|talk me through|walk me through)\b"
    ))
    .is_match(m)
        || re!(
            r"\bexplain\w*\b|\bexplanation\b|\bgist of\b|\bsummary of\b|\boverview of\b|\bwhat happens\b|\bwhat(?:'s| is) (?:going on|happening)\b|\bwhat(?:'s| is| are) (?:the )?[\w' ]{1,30} like\b|\b(?:in brief|briefly)\b[^.?!]*\b(?:story|acts?|plot)\b"
        )
        .is_match(m)
    {
        return Some(Explain);
    }
    if re!(r"\b(?:what|who)(?:'s| is) (?:in|on)\b|\bwho are (?:the|all|those|these|they)\b|\bwhat (?:are|were) (?:all )?the \w{3,}s\b").is_match(m)
        || plural_after_which(m)
    {
        return Some(List);
    }
    let mood = re!(
        r"\b(?:more|less|better|stronger|weaker|funnier|darker|tense|interesting|compelling)\b"
    )
    .is_match(m);
    if re!(
        r"\b(?:ideas?|suggest\w*|brainstorm\w*|alternatives?|what if|improv\w*|rewrite|rework|punch up|twist|stronger|any thoughts|thoughts on|what would you|options for)\b|\bhow (?:could|can|would|might|should) (?:i|we|you) (?:make|improve|fix|strengthen|rewrite|raise|build)\b|\bmake (?:it|this|that|scene \d+|the \w+) (?:more|less|better|stronger)\b"
    )
    .is_match(m)
    {
        return Some(Brainstorm);
    }
    if re!(concat!(
        pre!(),
        r"(?:(?:i want you to|i want to|i'd like to|let's)\s+)?(?:add|create|make|delete|remove|rename|change|move|set|update|assign|mark|insert|duplicate|merge|split|put|swap|replace|edit|reschedule|attach|archive|restore)\b"
    ))
    .is_match(m)
        && !mood
    {
        return Some(Change);
    }
    if re!(
        r"\bwho (?:plays|is playing|played|will play|is cast as|is the|is our|are the|was the)\b|\bwho's (?:playing|the)\b|\b(?:phone|contact|email|e-mail|mobile|whatsapp)\b|\b(?:have|has|is there|are there) (?:a|any) (?:clash|clashes|conflicts?)\b|\bwhen (?:is|are|do|does|did|will|can)\b|\bwhat time\b|\bwhich (?:shoot )?day\b|\bhow much\b|\btotal (?:planned|actual|spent|spend|budget|cost|amount|so far)\b|\bwhat (?:does|do|will|would) (?:it|that|this|they|he|she) cost\b|\bwhat(?:'s| is) the (?:status|location|phone|email|address|age|name|title|date|cost|price|fee|rate|time|duration|runtime|budget|total|spend|call time)\b|\bwhere (?:is|are|do we|will we) (?:the |we )?(?:shoot|shooting)\b"
    )
    .is_match(m)
    {
        return Some(Lookup);
    }
    if re!(concat!(pre!(), r"(?:list|enumerate)\b")).is_match(m)
        || re!(
            r"\b(?:list|enumerate)\s+(?:all|every|the|each)\b|\b(?:show|give|tell) me (?:all|every|each)\b|\bname (?:all|every|each)\b|\bwhich (?:scenes|characters|props|locations|days|actors|people|files|cards|acts|crew|cast|vehicles|items|tasks)\b|\bwhat (?:are|were) (?:all )?the (?:scenes|characters|props|locations|files|tasks|cards|days)\b"
        )
        .is_match(m)
    {
        return Some(List);
    }
    // Two scenes named in one question ask how they relate or compare.
    if men.scenes >= 2 {
        return Some(Compare);
    }
    None
}

/// "which budget lines", "which locations are": a plural noun right after "which" asks for several.
fn plural_after_which(m: &str) -> bool {
    re!(r"\bwhich\s+(?:\w+\s+){0,2}?(\w{3,}s)\b")
        .captures_iter(m)
        .any(|c| {
            !matches!(
                &c[1],
                "has" | "was" | "does" | "this" | "its" | "his" | "yes" | "always" | "across"
            )
        })
}

/// A type for a message nothing could read: a question is taken as a lookup, the rest as chat.
pub(super) fn default_type(text: &str) -> AnswerType {
    let m = text.to_lowercase();
    if m.contains('?') || re!(r"^(?:what|who|how|which|when|where|why)\b").is_match(m.trim()) {
        AnswerType::Lookup
    } else {
        AnswerType::Chat
    }
}

/// The length class of the answer: from the type, moved by words in the message.
pub(super) fn length_class(text: &str, t: AnswerType) -> LengthClass {
    let base = t.length_class();
    let m = text.to_lowercase();
    if re!(r"\b(?:one line|one-line|one liner|one-liner|one sentence|one-sentence|in a sentence|in a line|one word|just the number|tl;?dr)\b")
        .is_match(&m)
    {
        return if t == AnswerType::Chat {
            base
        } else {
            LengthClass::OneSentence
        };
    }
    let shorten = re!(
        r"\b(?:briefly|in brief|brief|short answer|keep it short|shortly|quickly|quick answer|in a few words|concise|concisely|in short)\b"
    )
    .is_match(&m);
    let lengthen = re!(
        r"\b(?:in detail|detailed|in depth|in-depth|at length|long paragraph|long answer|long explanation|elaborate|thorough|thoroughly|step by step|full explanation|complete explanation)\b"
    )
    .is_match(&m);
    match (shorten, lengthen) {
        (true, false) => base.shorter(),
        (false, true) => base.longer(),
        _ => base,
    }
}
