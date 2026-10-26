//! People: who plays a role, who holds a crew job, when someone is free, and how to reach them.
//! Reads `cast_member`, `crew_member` and `availability_window`. Contact data is read only for a
//! person who may see it (`production::privacy::may_see_contacts`); the question for it is still
//! answered, with a plain statement that it is not shown.

use std::sync::LazyLock;

use openframe_domain::AppResult;
use regex::Regex;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::{Msg, clip};
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::{Resolved, normalize};
use crate::modules::ai::turn::{AnswerType, Domain, Group, TurnRead};
use crate::modules::production::availability::{self as av, AvailabilityOwner};

static EMAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[\w.+-]+@[\w-]+(?:\.[\w-]+)+").expect("valid pattern"));
static PHONE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\+?\d[\d ()-]{7,}\d").expect("valid pattern"));

/// Nouns that name a crew job.
const JOBS: &[&str] = &[
    "makeup artist",
    "gaffer",
    "grip",
    "director",
    "producer",
    "supervisor",
    "coordinator",
    "manager",
    "operator",
    "editor",
    "designer",
    "artist",
    "dop",
    "cinematographer",
    "stunt",
    "assistant",
];

/// True when the message asks about a crew job or about the crew.
pub fn asks_crew_job(m: &Msg) -> bool {
    JOBS.iter().any(|j| m.has(j)) || m.stem("crew")
}

/// Other names of a crew job.
const JOB_ALIASES: &[(&str, &[&str])] = &[
    (
        "director of photography",
        &["dop", "dp", "cinematographer", "director of photography"],
    ),
    (
        "makeup artist",
        &["makeup", "make up artist", "makeup artist"],
    ),
    ("script supervisor", &["script supervisor", "continuity"]),
    ("location manager", &["location manager"]),
];

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("people", || fetch(ctx, read, r))
}

struct Cast {
    id: String,
    actor: String,
    role: String,
    character_id: Option<String>,
    primary: bool,
    contact: String,
    notes: String,
}

struct Crew {
    id: String,
    person: String,
    role: String,
    department: String,
    contact: String,
    notes: String,
}

fn cast_rows(ctx: &FetchCtx<'_>) -> AppResult<Vec<Cast>> {
    let mut st = ctx.conn.prepare(
        "SELECT m.id, m.person_name, COALESCE(ch.name, m.character_name, ''), m.character_id, m.is_primary,
                COALESCE(m.contact, ''), COALESCE(m.availability_notes, '')
         FROM cast_member m LEFT JOIN story_character ch ON ch.id = m.character_id AND ch.deleted_at IS NULL
         WHERE m.deleted_at IS NULL AND m.archived = 0 ORDER BY 3, m.person_name LIMIT 300",
    )?;
    let rows = st
        .query_map([], |r| {
            Ok(Cast {
                id: r.get(0)?,
                actor: r.get(1)?,
                role: r.get(2)?,
                character_id: r.get(3)?,
                primary: r.get(4)?,
                contact: r.get(5)?,
                notes: r.get(6)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

fn crew_rows(ctx: &FetchCtx<'_>) -> AppResult<Vec<Crew>> {
    let mut st = ctx.conn.prepare(
        "SELECT id, person_name, role, COALESCE(department, ''), COALESCE(contact, ''), COALESCE(availability_notes, '')
         FROM crew_member WHERE deleted_at IS NULL AND archived = 0 ORDER BY department, role, person_name LIMIT 300",
    )?;
    let rows = st
        .query_map([], |r| {
            Ok(Crew {
                id: r.get(0)?,
                person: r.get(1)?,
                role: r.get(2)?,
                department: r.get(3)?,
                contact: r.get(4)?,
                notes: r.get(5)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

fn windows(ctx: &FetchCtx<'_>, owner: AvailabilityOwner, id: &str) -> AppResult<String> {
    let w = av::windows_for(ctx.conn, owner, id)?;
    Ok(if w.is_empty() {
        String::new()
    } else {
        av::summary(&w)
    })
}

fn fetch(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let t = read.answer_type;
    let narrative = matches!(read.group, Group::Narrative | Group::Brainstorm);
    let asks_contact = m.any(&[
        "contact", "contacts", "phone", "email", "e mail", "mail", "mobile", "number", "reach",
        "details",
    ]) && m.any(&[
        "contact",
        "contacts",
        "phone",
        "email",
        "e mail",
        "mobile",
        "reach",
        "contact details",
    ]);
    let asks_avail = m.any(&[
        "available",
        "availability",
        "unavailable",
        "free",
        "busy",
        "clash",
        "clashes",
        "exam",
        "exams",
        "dates",
        "date",
        "off",
    ]);
    let asks_cast = m.any(&[
        "plays",
        "play",
        "played",
        "playing",
        "actor",
        "actors",
        "actress",
        "cast",
        "performer",
        "portrays",
        "who is playing",
    ]);
    let asks_crew_role = asks_crew_job(m);
    // A name with nothing else asked, in words the topic list does not know: who plays it.
    let implicit_cast = t == AnswerType::Lookup
        && !r.characters.is_empty()
        && !asks_crew_role
        && read
            .domains
            .iter()
            .all(|d| matches!(d, Domain::Story | Domain::Screenplay | Domain::People));
    let mut out: Vec<Fact> = Vec::new();
    let cast = cast_rows(ctx)?;
    let crew = crew_rows(ctx)?;

    if asks_contact && !ctx.guard.contacts {
        out.push(Fact::missing(
            "Contact details are not shown for your role.",
        ));
    }

    // ---- cast
    let mut shown: Vec<&Cast> = Vec::new();
    for h in &r.persons {
        if let Some(c) = cast.iter().find(|c| c.id == h.id) {
            shown.push(c);
        }
    }
    let wants_roles = asks_cast || asks_avail || asks_contact || narrative || implicit_cast;
    if wants_roles {
        for h in &r.characters {
            let rows: Vec<&Cast> = cast
                .iter()
                .filter(|c| c.character_id.as_deref() == Some(h.id.as_str()))
                .collect();
            if rows.is_empty() && (asks_cast || asks_avail || asks_contact || implicit_cast) {
                out.push(Fact::absent(
                    format!("a cast member for {}", h.name),
                    "Cast and Crew",
                ));
            }
            for c in rows {
                if !shown.iter().any(|s| s.id == c.id) {
                    shown.push(c);
                }
            }
        }
    }
    // "who plays them": the cast of a whole scene's speaking characters.
    for c in shown {
        let mut f = Fact::new("Cast", &c.id)
            .f("role", &c.role)
            .f("actor", &c.actor)
            .prio(P_HIGH);
        if !c.primary {
            f = f.f("alternate", "yes");
        }
        if matches!(
            t,
            AnswerType::Lookup | AnswerType::List | AnswerType::Compare
        ) && (asks_cast || (!asks_avail && !implicit_cast))
        {
            f = f.require(&c.actor);
        }
        if asks_avail {
            f = f.f(
                "availability",
                windows(ctx, AvailabilityOwner::Cast, &c.id)?,
            );
            f = f.f("availability note", clip(&c.notes, 160));
            f = f.require(&c.actor);
        }
        if asks_contact && ctx.guard.contacts {
            f = with_contact(f, &c.contact);
        }
        out.push(f);
    }
    // The whole cast, when a list of it is asked for.
    if r.persons.is_empty()
        && r.characters.is_empty()
        && asks_cast
        && !asks_crew_role
        && !m.any(&[
            "problem",
            "problems",
            "clash",
            "clashes",
            "conflict",
            "conflicts",
            "issue",
            "issues",
        ])
        && matches!(
            t,
            AnswerType::List | AnswerType::Lookup | AnswerType::Count | AnswerType::Explain
        )
    {
        if cast.is_empty() {
            out.push(Fact::absent("any cast", "Cast and Crew"));
        } else if t == AnswerType::Count {
            out.push(
                Fact::new("Count", "")
                    .f("what", "cast members")
                    .f("value", cast.len().to_string())
                    .require(cast.len().to_string()),
            );
        } else {
            for c in cast.iter().take(40) {
                out.push(
                    Fact::new("Cast", &c.id)
                        .f("role", &c.role)
                        .f("actor", &c.actor)
                        .prio(P_MID),
                );
            }
        }
    }

    // ---- crew
    let mut crew_shown: Vec<&Crew> = Vec::new();
    for h in &r.persons {
        if let Some(c) = crew.iter().find(|c| c.id == h.id) {
            crew_shown.push(c);
        }
    }
    let role_hits: Vec<&Crew> = crew
        .iter()
        .filter(|c| {
            let role = normalize(&c.role);
            if !role.is_empty() && m.has(&role) {
                return true;
            }
            JOB_ALIASES
                .iter()
                .any(|(canon, names)| role == *canon && names.iter().any(|n| m.has(n)))
        })
        .collect();
    for c in role_hits {
        if !crew_shown.iter().any(|s| s.id == c.id) {
            crew_shown.push(c);
        }
    }
    let job_asked = JOBS.iter().any(|j| m.has(j));
    if crew_shown.is_empty()
        && (job_asked || m.stem("crew"))
        && matches!(
            t,
            AnswerType::Lookup
                | AnswerType::List
                | AnswerType::Count
                | AnswerType::Compare
                | AnswerType::Explain
        )
        && r.persons.is_empty()
    {
        if crew.is_empty() {
            out.push(Fact::absent("any crew", "Cast and Crew"));
        } else if m.stem("crew") && !job_asked {
            for c in crew.iter().take(40) {
                out.push(
                    Fact::new("Crew", &c.id)
                        .f("role", &c.role)
                        .f("person", &c.person)
                        .f("department", &c.department)
                        .prio(P_MID),
                );
            }
        } else if r.characters.is_empty() && !asks_cast {
            let job = JOBS
                .iter()
                .filter(|j| m.has(j))
                .max_by_key(|j| j.len())
                .map_or("a crew member with that job".to_string(), |j| {
                    format!("a {j} for this project")
                });
            out.push(Fact::absent(job, "Cast and Crew"));
        }
    }
    for c in crew_shown {
        let mut f = Fact::new("Crew", &c.id)
            .f("role", &c.role)
            .f("person", &c.person)
            .f("department", &c.department)
            .prio(P_HIGH)
            .require(&c.person);
        if asks_avail {
            f = f.f(
                "availability",
                windows(ctx, AvailabilityOwner::Crew, &c.id)?,
            );
            f = f.f("availability note", clip(&c.notes, 160));
        }
        if asks_contact && ctx.guard.contacts {
            f = with_contact(f, &c.contact);
        }
        out.push(f);
    }
    Ok(out)
}

/// The contact line, and the phone and email in it as values the answer must state.
fn with_contact(f: Fact, contact: &str) -> Fact {
    let mut f = f.f("contact", contact);
    for e in EMAIL.find_iter(contact) {
        f = f.require(e.as_str());
    }
    for p in PHONE.find_iter(contact) {
        f = f.require(p.as_str().trim());
    }
    f
}
