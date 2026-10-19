//! Explicit references before a delete (stable identity rule, Production depth contract
//! section 9). The delete confirmation names what still points at a catalog item, location,
//! cast member or crew member, and what happens to those links. Deleting is recoverable
//! (Recently Deleted keeps items 30 days) and never rewrites the screenplay.

use openframe_domain::{Actor, AppError, AppResult, Capability};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{ProductionSceneRef, SourceScenes, active_source, catalog_usage, sort_refs};
use crate::core::AppCore;
use crate::registry::{Registry, hidden as h};

pub fn register(r: &mut Registry) {
    use crate::registry::OperationMetadata as M;
    r.module("Production");
    r.query("production.references", references).meta(
        M::read(
            "What still refers to a catalog item, location, cast or crew member before a delete.",
        )
        .hidden(h::UI_FLOW),
    );
}

openframe_domain::string_enum!(
    /// Kinds of production records with explicit references.
    ProductionRecordKind {
        CatalogItem = "catalog_item",
        Location = "location",
        CastMember = "cast_member",
        CrewMember = "crew_member",
    }
);

#[derive(Debug, Deserialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProductionReferencesArgs {
    pub kind: ProductionRecordKind,
    pub id: String,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export)]
#[serde(rename_all = "camelCase")]
pub struct ProductionReferences {
    pub name: String,
    /// Scenes of the Production Source (and removed scenes) that use the record.
    pub scenes: Vec<ProductionSceneRef>,
    /// Other records linked to it, e.g. "Catalog item: Old Railway Station".
    pub links: Vec<String>,
    /// Plain words on what deleting does to those references.
    pub consequence: String,
}

fn name_of(c: &Connection, kind: ProductionRecordKind, id: &str) -> AppResult<String> {
    let sql = match kind {
        ProductionRecordKind::CatalogItem => {
            "SELECT name FROM catalog_item WHERE id = ?1 AND deleted_at IS NULL"
        }
        ProductionRecordKind::Location => {
            "SELECT name FROM location WHERE id = ?1 AND deleted_at IS NULL"
        }
        ProductionRecordKind::CastMember => {
            "SELECT person_name FROM cast_member WHERE id = ?1 AND deleted_at IS NULL"
        }
        ProductionRecordKind::CrewMember => {
            "SELECT person_name FROM crew_member WHERE id = ?1 AND deleted_at IS NULL"
        }
    };
    c.query_row(sql, [id], |r| r.get(0))
        .optional()?
        .ok_or_else(|| AppError::not_found("record"))
}

fn strings(c: &Connection, sql: &str, id: &str) -> AppResult<Vec<String>> {
    let mut stmt = c.prepare(sql)?;
    let rows = stmt
        .query_map([id], |r| r.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(rows)
}

fn scene_word(n: usize) -> String {
    if n == 1 {
        "1 scene".into()
    } else {
        format!("{n} scenes")
    }
}

pub(crate) fn build(
    c: &Connection,
    root: &std::path::Path,
    kind: ProductionRecordKind,
    id: &str,
) -> AppResult<ProductionReferences> {
    let name = name_of(c, kind, id)?;
    let source = active_source(c)?;
    let scenes_in = SourceScenes::load(c, source.as_ref().map(|s| s.draft_id.as_str()))?;
    let mut links = Vec::new();
    let (scenes, consequence) = match kind {
        ProductionRecordKind::CatalogItem => {
            let scenes = catalog_usage(c, &scenes_in)?.remove(id).unwrap_or_default();
            for a in strings(
                c,
                "SELECT alias FROM catalog_alias WHERE catalog_item_id = ?1 ORDER BY alias",
                id,
            )? {
                links.push(format!("Also known as: {a}"));
            }
            for l in strings(
                c,
                "SELECT l.name FROM location l JOIN catalog_item ci ON ci.location_id = l.id
                 WHERE ci.id = ?1 AND l.deleted_at IS NULL",
                id,
            )? {
                links.push(format!("Location: {l}"));
            }
            let consequence = if scenes.is_empty() {
                "No scene uses it. It moves to Recently Deleted for 30 days.".to_string()
            } else {
                format!(
                    "It moves to Recently Deleted for 30 days. The breakdown of {} keeps its entry, marked “Catalog item removed”, until you restore it.",
                    scene_word(scenes.len())
                )
            };
            (scenes, consequence)
        }
        ProductionRecordKind::Location => {
            let mut scenes = super::locations::location_usage(c, &scenes_in)?
                .remove(id)
                .unwrap_or_default();
            sort_refs(&mut scenes);
            for n in strings(
                c,
                "SELECT name FROM catalog_item WHERE location_id = ?1 AND deleted_at IS NULL ORDER BY name",
                id,
            )? {
                links.push(format!("Catalog item: {n}"));
            }
            for n in strings(
                c,
                "SELECT name FROM location WHERE replacement_location_id = ?1 AND deleted_at IS NULL ORDER BY name",
                id,
            )? {
                links.push(format!("Replacement for: {n}"));
            }
            let consequence = format!(
                "It moves to Recently Deleted for 30 days with its photos and availability. {} keep their breakdown; the screenplay is not changed.",
                if scenes.is_empty() {
                    "Scenes".to_string()
                } else {
                    format!("The {}", scene_word(scenes.len()))
                }
            );
            (scenes, consequence)
        }
        ProductionRecordKind::CastMember => {
            let dir = super::people::directory(c, root, true)?;
            let m = dir
                .members
                .into_iter()
                .find(|m| m.id == id)
                .ok_or_else(|| AppError::not_found("cast member"))?;
            if let Some(ch) = &m.character_name {
                links.push(format!(
                    "Plays {ch}{}",
                    if m.is_primary { "" } else { " (alternate)" }
                ));
            }
            let consequence = format!(
                "{name} moves to Recently Deleted for 30 days. {} becomes “Needs casting” again; call sheets already issued keep their copy.",
                m.character_name.as_deref().unwrap_or("The character")
            );
            (m.scenes, consequence)
        }
        ProductionRecordKind::CrewMember => (
            vec![],
            format!(
                "{name} moves to Recently Deleted for 30 days. Call sheets already issued keep their copy."
            ),
        ),
    };
    Ok(ProductionReferences {
        name,
        scenes,
        links,
        consequence,
    })
}

fn references(
    core: &AppCore,
    actor: &Actor,
    args: ProductionReferencesArgs,
) -> AppResult<ProductionReferences> {
    actor.require(Capability::View, "view production records")?;
    let s = core.project()?;
    let root = s.layout.root().to_path_buf();
    s.store.read(|c| build(c, &root, args.kind, &args.id))
}
