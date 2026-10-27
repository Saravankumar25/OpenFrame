//! Settings: the project's own settings and who works on it, how the app looks on this computer, and
//! spell checking. Never a licence, a password or a path.
//! Reads `project`, `project_member`, the appearance state and the spelling settings.

use openframe_domain::AppResult;

use crate::modules::ai::dossier::ctx::{FetchCtx, safe};
use crate::modules::ai::dossier::words::clip;
use crate::modules::ai::dossier::{Fact, P_HIGH, P_MID};
use crate::modules::ai::index::Resolved;
use crate::modules::ai::turn::TurnRead;
use crate::modules::appearance::{AppearanceMode, state, theme};

pub fn run(ctx: &FetchCtx<'_>, read: &TurnRead, r: &Resolved) -> Vec<Fact> {
    safe("settings", || fetch(ctx, read, r))
}

fn fetch(ctx: &FetchCtx<'_>, _read: &TurnRead, _r: &Resolved) -> AppResult<Vec<Fact>> {
    let m = &ctx.m;
    let mut out = Vec::new();
    let project_words = m.any(&[
        "project",
        "title",
        "status",
        "logline",
        "genre",
        "language",
        "creator",
        "type",
        "members",
        "roles",
        "settings",
        "who is on the project",
    ]);
    if project_words {
        let row: Option<(String, String, String, String, String, String, String)> = ctx
            .conn
            .query_row(
                "SELECT title, project_type, status, COALESCE(language, ''), COALESCE(genre, ''),
                        COALESCE(creator, ''), COALESCE(logline, '') FROM project LIMIT 1",
                [],
                |r| {
                    Ok((
                        r.get(0)?,
                        r.get(1)?,
                        r.get(2)?,
                        r.get(3)?,
                        r.get(4)?,
                        r.get(5)?,
                        r.get(6)?,
                    ))
                },
            )
            .ok();
        if let Some((title, kind, status, language, genre, creator, logline)) = row {
            out.push(
                Fact::new("Project", "")
                    .f("title", title)
                    .f("type", kind)
                    .f("status", status)
                    .f("language", language)
                    .f("genre", genre)
                    .f("creator", creator)
                    .f("logline", clip(&logline, 400))
                    .prio(P_HIGH),
            );
        }
        if m.any(&["members", "roles", "role", "who", "team", "people"]) {
            let mut st = ctx.conn.prepare(
                "SELECT id, display_name, role FROM project_member WHERE deleted_at IS NULL ORDER BY display_name LIMIT 50",
            )?;
            let rows: Vec<(String, String, String)> = st
                .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
                .collect::<Result<_, _>>()?;
            for (id, name, role) in rows {
                out.push(
                    Fact::new("Project member", id)
                        .f("name", name)
                        .f("role", role)
                        .prio(P_MID),
                );
            }
        }
    }
    if let Some(core) = ctx.core
        && m.any(&[
            "theme",
            "themes",
            "dark",
            "light",
            "appearance",
            "mode",
            "colour",
            "color",
        ])
        && let Ok(s) = state(core)
    {
        let mode = match s.mode {
            AppearanceMode::Light => "Light",
            AppearanceMode::Dark => "Dark",
            AppearanceMode::System => "System (follows Windows or macOS)",
        };
        let name = theme::builtin(&s.theme_id)
            .map(|t| t.name.clone())
            .or_else(|| {
                s.my_themes
                    .iter()
                    .find(|t| t.id == s.theme_id)
                    .map(|t| t.name.clone())
            })
            .unwrap_or(s.theme_id);
        let built_in: Vec<&str> = theme::builtin_themes()
            .iter()
            .map(|t| t.name.as_str())
            .collect();
        out.push(
            Fact::new("Appearance", "")
                .f("mode", mode)
                .f("theme", name)
                .f("built-in themes", built_in.join(", "))
                .prio(P_HIGH),
        );
    }
    if m.any(&["spell", "spelling", "spellcheck", "proofing", "dictionary"]) {
        let p = crate::modules::writing::proofing::read_settings(ctx.conn)?;
        out.push(
            Fact::new("Spell checking", "")
                .f("on", if p.enabled { "yes" } else { "no" })
                .f("main language", p.main_language)
                .f("second language", p.second_language.unwrap_or_default())
                .prio(P_HIGH),
        );
    }
    Ok(out)
}
