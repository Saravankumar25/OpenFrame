//! What every fetcher is given: the connection, the person, the index, the message as words, and a
//! few shared reads (the draft of a screenplay, who speaks in which scene, the day of a scene).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::rc::Rc;
use std::sync::LazyLock;

use openframe_domain::{Actor, AppResult};
use regex::Regex;
use rusqlite::{Connection, OptionalExtension};

use super::guard::Guard;
use super::words::{Msg, title_case};
use super::{Fact, Input};
use crate::core::AppCore;
use crate::modules::ai::index::{EntityIndex, Kind, Resolved, ScreenplayInfo, normalize};
use crate::modules::ai::queries;
use crate::modules::ai::sections::PickedLeaf;
use crate::modules::ai::turn::ReadHow;
use crate::modules::schedule::board::{self, DayRow};

/// A scene as the fetchers name it.
#[derive(Debug, Clone)]
pub struct SceneRef {
    pub id: String,
    pub lineage_id: String,
    pub number: u32,
    pub heading: String,
    pub screenplay_id: String,
    pub screenplay_title: String,
    pub draft_id: String,
}

/// Who speaks in which scene of one draft, with names as the Story calls them.
#[derive(Debug, Default)]
pub struct Presence {
    pub by_scene: HashMap<String, Vec<String>>,
    pub by_name: BTreeMap<String, BTreeSet<u32>>,
}

pub struct FetchCtx<'a> {
    pub conn: &'a Connection,
    pub actor: &'a Actor,
    pub index: &'a EntityIndex,
    pub core: Option<&'a AppCore>,
    /// The message; a correction ("I asked you ...") carries the message it corrects too.
    pub message: String,
    pub m: Msg,
    /// The person's previous message in this conversation, for a follow-up that names nothing.
    pub previous: Option<String>,
    pub guard: Guard,
    /// The section this fetcher reads for. `None` only for the names stage, which reads no section.
    pub picked: Option<&'a PickedLeaf>,
    drafts: RefCell<HashMap<String, Option<queries::DraftRef>>>,
    presence: RefCell<HashMap<String, Rc<Presence>>>,
    days: RefCell<Option<Rc<Vec<DayRow>>>>,
}

impl<'a> FetchCtx<'a> {
    pub fn new(input: &Input<'a>) -> Self {
        let message = match (input.read.how, input.previous_message) {
            (ReadHow::Repair, Some(p)) => {
                format!("{} {}", replaced_by(p, input.message), input.message)
            }
            _ => input.message.to_string(),
        };
        Self {
            conn: input.conn,
            actor: input.actor,
            index: input.index,
            core: input.core,
            m: Msg::new(&message),
            message,
            previous: input.previous_message.map(str::to_string),
            guard: Guard::for_actor(input.actor),
            picked: None,
            drafts: RefCell::new(HashMap::new()),
            presence: RefCell::new(HashMap::new()),
            days: RefCell::new(None),
        }
    }

    /// The context of one picked section: its topic words open the fetchers, its draft is the draft
    /// every read of that screenplay uses, and its role rule can only take contact data away.
    pub fn for_leaf(input: &Input<'a>, picked: &'a PickedLeaf) -> Self {
        let mut ctx = Self::new(input);
        ctx.m = Msg::with_topic(&ctx.message, picked.leaf.topic);
        ctx.picked = Some(picked);
        ctx.guard.contacts =
            ctx.guard.contacts && picked.leaf.min_role.allows_contacts(input.actor);
        if let Some(d) = &picked.draft {
            ctx.drafts
                .borrow_mut()
                .insert(d.screenplay_id.clone(), Some(d.clone()));
        }
        ctx
    }

    /// The same section read with the words the person typed alone.
    pub fn typed_for_leaf(input: &Input<'a>, picked: &'a PickedLeaf) -> Self {
        let mut ctx = Self::for_leaf(input, picked);
        ctx.m = Msg::new(&ctx.message);
        ctx
    }

    /// The day this section was narrowed to, when the person chose one.
    pub fn picked_day(&self) -> Option<&str> {
        self.picked
            .and_then(|p| p.day.as_ref())
            .map(|d| d.id.as_str())
    }

    /// The screenplay a message without a title is about: the named one, the only one, else the
    /// longest.
    pub fn screenplay(&self, r: &Resolved) -> Option<&ScreenplayInfo> {
        r.screenplay_id()
            .and_then(|id| self.index.screenplay(id))
            .or_else(|| {
                let all = &self.index.screenplays;
                if all.len() == 1 {
                    all.first()
                } else {
                    all.iter().max_by_key(|s| s.scene_count)
                }
            })
    }

    /// The draft the assistant reads for a screenplay (current, else latest).
    pub fn draft(&self, screenplay_id: &str) -> Option<queries::DraftRef> {
        self.drafts
            .borrow_mut()
            .entry(screenplay_id.to_string())
            .or_insert_with(|| {
                crate::modules::ai::index::draft_for_screenplay(self.conn, screenplay_id)
                    .ok()
                    .flatten()
            })
            .clone()
    }

    pub fn scene_by_id(&self, scene_id: &str) -> Option<SceneRef> {
        let (d, s) = queries::scene_by_id(self.conn, scene_id).ok().flatten()?;
        Some(SceneRef {
            id: s.id,
            lineage_id: s.lineage_id,
            number: s.number as u32,
            heading: s.heading,
            screenplay_id: d.screenplay_id,
            screenplay_title: d.screenplay_title,
            draft_id: d.id,
        })
    }

    pub fn scene_of(&self, screenplay_id: &str, number: u32) -> Option<SceneRef> {
        let d = self.draft(screenplay_id)?;
        let s = queries::scenes(self.conn, &d.id)
            .ok()?
            .into_iter()
            .find(|s| s.number as u32 == number)?;
        Some(SceneRef {
            id: s.id,
            lineage_id: s.lineage_id,
            number,
            heading: s.heading,
            screenplay_id: d.screenplay_id.clone(),
            screenplay_title: d.screenplay_title.clone(),
            draft_id: d.id,
        })
    }

    /// The scenes a message is about: the ones it names (or the focus gave), the scenes of a card it
    /// names, and "the last scene" or "the first scene".
    pub fn focus_scenes(&self, r: &Resolved) -> Vec<SceneRef> {
        let mut out: Vec<SceneRef> = Vec::new();
        fn add(out: &mut Vec<SceneRef>, s: Option<SceneRef>) {
            if let Some(s) = s
                && !out.iter().any(|o| o.id == s.id)
            {
                out.push(s);
            }
        }
        for h in &r.scenes {
            match &h.scene_id {
                Some(id) => add(&mut out, self.scene_by_id(id)),
                None => add(&mut out, self.scene_of(&h.screenplay_id, h.ordinal)),
            }
        }
        for (kind, hit) in &r.others {
            if *kind == Kind::Card
                && let Some(id) = self.card_scene(&hit.id)
            {
                add(&mut out, self.scene_by_id(&id));
            }
        }
        // "Varghese's last scene" is his, not the screenplay's: the fetchers read it from his scenes.
        if out.is_empty()
            && r.characters.is_empty()
            && let Some(sp) = self.screenplay(r)
        {
            if self
                .m
                .any(&["last scene", "final scene", "ending scene", "closing scene"])
            {
                add(&mut out, self.scene_of(&sp.id, sp.scene_count));
            } else if self.m.any(&["first scene", "opening scene"]) {
                add(&mut out, self.scene_of(&sp.id, 1));
            }
        }
        out
    }

    pub fn card_scene(&self, card_id: &str) -> Option<String> {
        self.conn
            .query_row(
                "SELECT screenplay_scene_id FROM story_scene_card WHERE id = ?1 AND deleted_at IS NULL",
                [card_id],
                |r| r.get::<_, Option<String>>(0),
            )
            .optional()
            .ok()
            .flatten()
            .flatten()
    }

    /// Who speaks in the scenes of a draft. One read per draft per message.
    pub fn presence(&self, draft_id: &str) -> Rc<Presence> {
        if let Some(p) = self.presence.borrow().get(draft_id) {
            return p.clone();
        }
        let p = Rc::new(self.read_presence(draft_id).unwrap_or_default());
        self.presence
            .borrow_mut()
            .insert(draft_id.to_string(), p.clone());
        p
    }

    fn read_presence(&self, draft_id: &str) -> AppResult<Presence> {
        let numbers: HashMap<String, u32> = queries::scenes(self.conn, draft_id)?
            .into_iter()
            .map(|s| (s.id, s.number as u32))
            .collect();
        let mut st = self.conn.prepare(
            "SELECT e.text, s.id FROM screenplay_element e
             JOIN screenplay_scene s ON s.id = e.scene_id
             WHERE s.draft_id = ?1 AND s.deleted_at IS NULL AND e.element_type = 'character'
             ORDER BY s.position, e.position",
        )?;
        let rows: Vec<(String, String)> = st
            .query_map([draft_id], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<_, _>>()?;
        let mut out = Presence::default();
        let mut canon: HashMap<String, String> = HashMap::new();
        for (text, scene) in rows {
            let cue = queries::normalize_cue(&text);
            if cue.is_empty() {
                continue;
            }
            let name = canon
                .entry(cue.clone())
                .or_insert_with(|| self.story_name(&cue))
                .clone();
            let names = out.by_scene.entry(scene.clone()).or_default();
            if !names.contains(&name) {
                names.push(name.clone());
            }
            if let Some(n) = numbers.get(&scene) {
                out.by_name.entry(name).or_default().insert(*n);
            }
        }
        Ok(out)
    }

    /// The name the Story gives a dialogue cue ("ANJU" is Anjali), else the cue as a name.
    pub fn story_name(&self, cue: &str) -> String {
        if let Some(hits) = self.index.phrase(&normalize(cue)) {
            let found = hits
                .iter()
                .filter_map(|(i, _)| self.index.entities.get(*i))
                .find(|e| e.kind == Kind::Character);
            if let Some(e) = found {
                return e.name.clone();
            }
        }
        title_case(cue)
    }

    /// Number and heading of scenes by id (the number is the place in the draft).
    pub fn scene_numbers(&self, ids: &[String]) -> HashMap<String, (u32, String)> {
        if ids.is_empty() {
            return HashMap::new();
        }
        let marks = vec!["?"; ids.len()].join(",");
        let sql = format!(
            "SELECT id, heading, n FROM (
                 SELECT id, heading, row_number() OVER (PARTITION BY draft_id ORDER BY position, id) AS n
                 FROM screenplay_scene WHERE deleted_at IS NULL) WHERE id IN ({marks})"
        );
        let read = || -> AppResult<HashMap<String, (u32, String)>> {
            let mut st = self.conn.prepare(&sql)?;
            let rows = st.query_map(rusqlite::params_from_iter(ids.iter()), |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    (r.get::<_, i64>(2)? as u32, r.get::<_, String>(1)?),
                ))
            })?;
            Ok(rows.collect::<Result<_, _>>()?)
        };
        read().unwrap_or_default()
    }

    /// The shooting days of the current schedule, numbered; empty when there is no schedule.
    pub fn days(&self) -> Rc<Vec<DayRow>> {
        if let Some(d) = self.days.borrow().as_ref() {
            return d.clone();
        }
        let days = board::current_schedule(self.conn)
            .ok()
            .flatten()
            .and_then(|s| board::load_days(self.conn, &s.id).ok())
            .unwrap_or_default();
        let days = Rc::new(days);
        *self.days.borrow_mut() = Some(days.clone());
        days
    }
}

static DAY_WORDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\b(?:shoot\s+)?day\s+#?\d{1,3}\b").expect("valid pattern"));
static SCENE_WORDS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)\bscenes?\s+#?\d{1,4}\b").expect("valid pattern"));

/// The earlier message a follow-up is read with: "and day 5" after "what is on day 4" reads day 5 alone,
/// and the same for a scene.
fn replaced_by(previous: &str, message: &str) -> String {
    let mut p = previous.to_string();
    for words in [&*DAY_WORDS, &*SCENE_WORDS] {
        if words.is_match(message) {
            p = words.replace_all(&p, " ").into_owned();
        }
    }
    p
}

/// Run a read that may fail; a failure adds no facts and is logged, never shown.
pub fn safe(name: &str, f: impl FnOnce() -> AppResult<Vec<Fact>>) -> Vec<Fact> {
    f().unwrap_or_else(|e| {
        tracing::warn!(
            code = e.code_str(),
            fetcher = name,
            "a dossier fetcher could not read"
        );
        Vec::new()
    })
}

#[cfg(test)]
mod tests {
    use super::replaced_by;

    #[test]
    fn a_day_or_scene_in_a_follow_up_replaces_the_earlier_one() {
        assert_eq!(
            replaced_by("what is on day 4", "and day 5").trim(),
            "what is on"
        );
        assert_eq!(
            replaced_by("props for scene 3", "and the vehicles"),
            "props for scene 3"
        );
        assert_eq!(
            replaced_by("props for scene 3", "and scene 9").trim(),
            "props for"
        );
    }
}
