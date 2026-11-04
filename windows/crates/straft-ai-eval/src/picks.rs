//! Turn the picks of a gold case (leaf ids with a screenplay title, a scene ordinal and a shooting
//! day number) into the payload `ai.ask` takes (ids), by reading the tree the panel reads
//! (`ai.sections`). A case that names something the project does not hold fails here, loudly, so a
//! typo in a gold file never turns into a quiet "all scenes".

use serde_json::{Value, json};
use straft_application::core::AppCore;

use crate::CaseSection;

fn text<'a>(v: &'a Value, key: &str) -> &'a str {
    v[key].as_str().unwrap_or_default()
}

/// The `sections` array of an `ai.ask` for these picks. `feature_scenes` are the scene ids of the
/// screenplay a case reads by default (the feature), used for a scene ordinal on a leaf that does
/// not name its screenplay.
pub fn resolve(
    core: &AppCore,
    sections: &[CaseSection],
    feature_scenes: &[String],
) -> Result<Vec<Value>, String> {
    let tree = core
        .dispatch(&core.local_actor(), "ai.sections", json!({}))
        .map_err(|e| format!("ai.sections: {} {}", e.code_str(), e.message))?;
    let picker_of = |leaf: &str| -> String {
        tree["branches"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|b| b["leaves"].as_array().into_iter().flatten())
            .find(|l| text(l, "id") == leaf)
            .map(|l| text(l, "picker").to_string())
            .unwrap_or_default()
    };
    let mut out = Vec::new();
    for s in sections {
        let picker = picker_of(&s.leaf);
        if picker.is_empty() {
            return Err(format!("{}: not a section in the tree", s.leaf));
        }
        let mut pick = json!({ "leaf": s.leaf });
        let screenplay = match &s.screenplay {
            Some(title) => Some(
                tree["screenplays"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|p| text(p, "title").eq_ignore_ascii_case(title))
                    .ok_or_else(|| format!("{}: no screenplay called {title}", s.leaf))?,
            ),
            None => None,
        };
        if picker == "screenplay"
            && let Some(sp) = screenplay
        {
            pick["screenplayId"] = json!(text(sp, "id"));
        }
        if let Some(n) = s.scene {
            let id = match screenplay {
                Some(sp) => sp["scenes"][n.wrapping_sub(1)]["id"]
                    .as_str()
                    .map(String::from),
                None => feature_scenes.get(n.wrapping_sub(1)).cloned(),
            }
            .ok_or_else(|| format!("{}: no scene {n}", s.leaf))?;
            pick["sceneId"] = json!(id);
        }
        if let Some(n) = s.day {
            let label = format!("Shoot Day {n}");
            let day = tree["days"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|d| text(d, "label") == label)
                .ok_or_else(|| format!("{}: no {label}", s.leaf))?;
            let sheet = tree["callSheets"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|c| text(c, "dayId") == text(day, "id"));
            match (picker.as_str(), sheet) {
                ("callSheet", Some(c)) => pick["callSheetId"] = json!(text(c, "id")),
                _ => pick["shootingDayId"] = json!(text(day, "id")),
            }
        }
        out.push(pick);
    }
    Ok(out)
}
