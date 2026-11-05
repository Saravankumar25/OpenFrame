//! The Idea Vault: folders, collections, notes, research, a quote, links, files, a voice memo, a
//! sketch and tags, as listed in `vault.json`; some items are sent to Story.

use std::collections::BTreeMap;

use base64::Engine;
use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn b64(bytes: &[u8]) -> String {
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

pub fn run(g: &Gold, ctx: &mut Ctx) {
    g.phase("Idea Vault");
    let v = ctx.vault.clone();
    let store = "project";
    let mut folders: BTreeMap<String, String> = BTreeMap::new();
    for f in arr(&v["folders"]) {
        let parent = f["parent"].as_str().and_then(|k| folders.get(k)).cloned();
        let id = g.id(&g.ok(
            "vault.create_folder",
            json!({ "store": store, "name": f["name"], "parentId": parent }),
        ));
        folders.insert(s(&f["key"]), id);
    }
    let rn = &v["renameFolder"];
    g.ok(
        "vault.rename_folder",
        json!({ "store": store, "id": folders[&s(&rn["key"])], "name": rn["name"], "expectedRev": null }),
    );
    let mut cols: BTreeMap<String, String> = BTreeMap::new();
    for c in arr(&v["collections"]) {
        let id = g.id(&g.ok(
            "vault.create_collection",
            json!({ "store": store, "name": c["name"], "note": c["note"] }),
        ));
        cols.insert(s(&c["key"]), id);
    }
    let rc = &v["renameCollection"];
    g.ok(
        "vault.rename_collection",
        json!({ "store": store, "id": cols[&s(&rc["key"])], "name": rc["name"], "note": rc["note"], "expectedRev": null }),
    );

    let mut items: BTreeMap<String, String> = BTreeMap::new();
    for it in arr(&v["items"]) {
        let folder = it["folder"].as_str().and_then(|k| folders.get(k));
        let col = it["collection"].as_str().and_then(|k| cols.get(k));
        let id = g.id(&g.ok(
            "vault.create",
            json!({ "store": store, "itemType": it["type"], "title": it["title"], "body": it["body"], "caption": it["caption"],
                    "url": it["url"], "sourceText": it["sourceText"], "folderId": folder, "collectionId": col, "tags": it["tags"] }),
        ));
        items.insert(s(&it["key"]), id);
    }

    let f = &v["files"];
    let paths: Vec<String> = arr(&f["paths"])
        .iter()
        .map(|p| ctx.media_path(&s(p)))
        .collect();
    let added = g.ok(
        "vault.add_files",
        json!({ "store": store, "paths": paths, "mode": "copy", "folderId": folders[&s(&f["folder"])],
                "collectionId": cols[&s(&f["collection"])], "itemType": null }),
    );
    let files: Vec<String> = arr(&added["added"]).iter().map(|x| s(&x["id"])).collect();
    if let Some(img) = files.get(f["opsImageIndex"].as_u64().unwrap_or(0) as usize) {
        ctx.set("vaultImage", img.clone());
    }

    // A palette linked from a shared folder that lives beside the output, relinked after it moved.
    let l = &v["linked"];
    let shared = ctx.out.join("shared-drive");
    std::fs::create_dir_all(&shared).ok();
    let src = shared.join(s(&l["name"]));
    std::fs::copy(ctx.media_path(&s(&l["file"])), &src).ok();
    let linked = g.ok(
        "vault.add_files",
        json!({ "store": store, "paths": [src.to_string_lossy()], "mode": "link", "folderId": folders[&s(&l["folder"])],
                "collectionId": cols[&s(&l["collection"])], "itemType": null }),
    );
    let palette = arr(&linked["added"])
        .first()
        .map(|x| s(&x["id"]))
        .unwrap_or_default();
    let moved = shared.join(s(&l["sub"])).join(s(&l["moved"]));
    std::fs::create_dir_all(moved.parent().unwrap()).ok();
    std::fs::rename(&src, &moved).ok();
    g.ok(
        "vault.relink",
        json!({ "store": store, "id": palette, "path": moved.to_string_lossy() }),
    );

    let vm = &v["voice"];
    let memo = std::fs::read(ctx.media.join(s(&vm["file"]))).unwrap_or_default();
    if memo.is_empty() {
        ctx.facts.insert(
            "voiceMemo".into(),
            json!("skipped: the say tool made no file"),
        );
    } else {
        g.ok(
            "vault.ingest_bytes",
            json!({ "store": store, "itemType": "voice", "fileName": vm["file"], "mediaType": "audio/mp4", "dataBase64": b64(&memo),
                    "durationMs": vm["durationMs"], "title": vm["title"], "caption": vm["caption"],
                    "folderId": folders[&s(&vm["folder"])], "collectionId": null }),
        );
    }
    let sk = &v["sketch"];
    let sketch = std::fs::read(ctx.media.join(s(&sk["file"]))).unwrap_or_default();
    let sketch_id = g.id(&g.ok(
        "vault.ingest_bytes",
        json!({ "store": store, "itemType": "sketch", "fileName": sk["fileName"], "mediaType": "image/png", "dataBase64": b64(&sketch),
                "title": sk["title"], "caption": sk["caption"], "folderId": folders[&s(&sk["folder"])], "collectionId": cols[&s(&sk["collection"])] }),
    ));

    let item = |k: &Value| items.get(&s(k)).cloned().unwrap_or_default();
    let list = |vals: &Value| arr(vals).iter().map(item).collect::<Vec<_>>();
    let tagged = &v["tags"][0];
    g.ok(
        "vault.add_tags",
        json!({ "store": store, "ids": list(&tagged["items"]), "tags": tagged["tags"] }),
    );
    g.ok(
        "vault.add_tags",
        json!({ "store": store, "ids": [sketch_id, palette], "tags": v["tags"][1]["tags"] }),
    );
    g.ok(
        "vault.add_tags",
        json!({ "store": store, "ids": [sketch_id, palette], "tags": ["temp"] }),
    );
    g.ok(
        "vault.remove_tag",
        json!({ "store": store, "ids": [sketch_id, palette], "tag": "temp" }),
    );
    g.ok(
        "vault.set_pinned",
        json!({ "store": store, "ids": list(&v["pinned"]), "pinned": true }),
    );
    g.ok(
        "vault.set_research_flag",
        json!({ "store": store, "ids": [item(&v["flagChecked"])], "flag": "checked" }),
    );
    g.ok(
        "vault.set_research_flag",
        json!({ "store": store, "ids": [item(&v["flagExpert"])], "flag": "needsExpertCheck" }),
    );
    g.ok(
        "vault.set_research_flag",
        json!({ "store": store, "ids": list(&v["flagUnchecked"]), "flag": "unchecked" }),
    );
    g.ok(
        "vault.set_research_flag",
        json!({ "store": store, "ids": [item(&v["flagCleared"])], "flag": null }),
    );
    let unsorted = cols[&s(&l["collection"])].clone();
    let design = cols[&s(&f["collection"])].clone();
    g.ok(
        "vault.add_to_collection",
        json!({ "store": store, "ids": [palette], "collectionId": design }),
    );
    g.ok(
        "vault.remove_from_collection",
        json!({ "store": store, "ids": [palette], "collectionId": unsorted }),
    );
    g.ok(
        "vault.delete_collection",
        json!({ "store": store, "id": unsorted }),
    );
    let mv = &v["moveExperiment"];
    g.ok(
        "vault.move_to_folder",
        json!({ "store": store, "ids": [item(&mv["key"])], "folderId": folders[&s(&mv["to"])] }),
    );
    g.ok(
        "vault.move_to_folder",
        json!({ "store": store, "ids": [item(&mv["key"])], "folderId": folders[&s(&mv["back"])] }),
    );
    let temp = g.id(&g.ok(
        "vault.create_folder",
        json!({ "store": store, "name": "Inbox", "parentId": null }),
    ));
    g.ok("vault.delete_folder", json!({ "store": store, "id": temp }));
    let up = &v["updateTitle"];
    g.ok(
        "vault.update",
        json!({ "store": store, "id": item(&up["key"]), "title": up["title"], "body": null, "caption": null,
                "url": null, "sourceText": null, "expectedRev": null }),
    );

    let gl = &v["global_"];
    let global_note = g.id(&g.ok(
        "vault.create",
        json!({ "store": "global", "itemType": "note", "title": gl["title"], "body": gl["body"], "tags": gl["tags"] }),
    ));
    g.ok("vault.overview", json!({ "store": "global" }));
    g.ok(
        "vault.copy_to_store",
        json!({ "from": "global", "to": "project", "ids": [global_note], "collectionId": cols[&s(&gl["collection"])], "folderId": folders[&s(&gl["folder"])] }),
    );
    g.ok(
        "vault.list",
        json!({ "store": "global", "view": null, "search": null }),
    );
    for view in [
        json!({ "kind": "all" }),
        json!({ "kind": "pinned" }),
        json!({ "kind": "recentlyAdded" }),
        json!({ "kind": "recentlyModified" }),
        json!({ "kind": "folder", "folderId": folders[&s(&v["viewFolder"])] }),
        json!({ "kind": "collection", "collectionId": cols[&s(&v["viewCollection"])] }),
        json!({ "kind": "tag", "tag": v["viewTag"] }),
    ] {
        g.ok(
            "vault.list",
            json!({ "store": store, "view": view, "search": null }),
        );
    }
    g.ok(
        "vault.list",
        json!({ "store": store, "view": null, "search": v["search"] }),
    );
    g.ok(
        "vault.get",
        json!({ "store": store, "id": item(&v["getKey"]) }),
    );
    g.ok("vault.overview", json!({ "store": store }));
    g.ok("visual.vault_images", json!({ "store": store }));

    // Clean-up: one item removed for good, another restored from the vault's Recently Deleted.
    let (purge, restore) = (item(&v["purge"]), item(&v["restore"]));
    g.ok(
        "vault.delete",
        json!({ "store": store, "ids": [purge, restore] }),
    );
    for row in arr(&g.ok("vault.trash_list", json!({ "store": store }))) {
        let id = s(&row["deleted"]["id"]);
        let target = s(&row["deleted"]["objectId"]) + &s(&row["item"]["id"]);
        let title = s(&row["deleted"]["title"]);
        if target == purge || title == title_of(&v, "purge") {
            g.ok("vault.purge", json!({ "store": store, "id": id }));
        } else if target == restore || title == title_of(&v, "restore") {
            g.ok("vault.restore", json!({ "store": store, "id": id }));
        }
    }

    for (key, name) in [
        ("character", "vaultChar"),
        ("card", "vaultCard"),
        ("beat", "vaultBeat"),
        ("sequence", "vaultSeq"),
    ] {
        ctx.set(name, item(&v["toStory"][key]));
    }
    ctx.facts.insert("vaultItems".into(), json!(items.len()));
}

fn title_of(v: &Value, which: &str) -> String {
    let key = s(&v[which]);
    arr(&v["items"])
        .iter()
        .find(|i| s(&i["key"]) == key)
        .map(|i| s(&i["title"]))
        .unwrap_or_default()
}

/// Ideas sent from the vault to Story (copies; the vault keeps its own items).
pub fn send_to_story(g: &Gold, ctx: &mut Ctx) {
    g.phase("Idea Vault to Story");
    g.ok("vault.story_targets", json!({}));
    let store = "project";
    let c = g.ok(
        "vault.send_to_story",
        json!({ "store": store, "id": ctx.id("vaultChar"), "target": "character", "parentType": null, "parentId": null }),
    );
    let name = arr(&ctx.vault["items"])
        .iter()
        .find(|i| s(&i["key"]) == s(&ctx.vault["toStory"]["character"]))
        .map(|i| s(&i["title"]))
        .unwrap_or_default();
    ctx.chars.insert(name, s(&c["storyId"]));
    let card = g.ok(
        "vault.send_to_story",
        json!({ "store": store, "id": ctx.id("vaultCard"), "target": "sceneCard", "parentType": "parking", "parentId": null }),
    );
    ctx.set("parkedCard", s(&card["storyId"]));
    g.ok(
        "vault.send_to_story",
        json!({ "store": store, "id": ctx.id("vaultBeat"), "target": "beat", "parentType": "parking", "parentId": null }),
    );
    let act = ctx.id(&format!(
        "act{}",
        ctx.vault["toStory"]["sequenceAct"].as_u64().unwrap_or(1)
    ));
    let seq = g.ok(
        "vault.send_to_story",
        json!({ "store": store, "id": ctx.id("vaultSeq"), "target": "sequence", "parentType": "act", "parentId": act }),
    );
    ctx.set("cutSequence", s(&seq["storyId"]));
}
