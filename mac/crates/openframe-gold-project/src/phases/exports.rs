//! Every export the app offers, written into `tests/gold-project/outputs/`.

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn record(ctx: &mut Ctx, path: &str) {
    let mut v = ctx.facts.get("exports").cloned().unwrap_or(json!([]));
    v.as_array_mut().unwrap().push(json!(path));
    ctx.facts.insert("exports".into(), v);
}

fn export(g: &Gold, ctx: &mut Ctx, op: &str, rel: &str, mut args: Value) {
    let path = ctx.outp(rel);
    args["path"] = json!(path);
    let r = g.ok(op, args);
    if !r.is_null() {
        let written = s(&r["path"]);
        record(ctx, if written.is_empty() { &path } else { &written });
    }
}

pub fn run(g: &Gold, ctx: &mut Ctx) {
    g.phase("Exports: screenplay");
    let blue = ctx.id("draftBlue");
    let shooting = ctx.id("draftShooting");
    let opts = |title: bool, numbers: bool, notes: bool, marks: bool, info: bool| json!({ "titlePage": title, "sceneNumbers": numbers, "includeNotes": notes, "revisionMarks": marks, "revisionInfo": info, "sceneIds": null });
    g.ok("interchange.export_preview", json!({ "draftId": blue, "options": opts(true, true, false, true, true), "pageNumbers": true }));
    export(
        g,
        ctx,
        "screenplay.export",
        "screenplay/Low Tide Radio - Shooting Draft.pdf",
        json!({ "draftId": shooting, "format": "pdf", "options": opts(true, true, false, false, false) }),
    );
    export(
        g,
        ctx,
        "screenplay.export",
        "screenplay/Low Tide Radio - Blue Revision (marked).pdf",
        json!({ "draftId": blue, "format": "pdf", "options": opts(true, true, false, true, true) }),
    );
    export(
        g,
        ctx,
        "screenplay.export",
        "screenplay/Low Tide Radio - Blue Revision.fdx",
        json!({ "draftId": blue, "format": "fdx", "options": opts(true, true, true, true, true) }),
    );
    export(
        g,
        ctx,
        "screenplay.export",
        "screenplay/Low Tide Radio - Blue Revision.fountain",
        json!({ "draftId": blue, "format": "fountain", "options": opts(true, true, true, false, true) }),
    );
    export(
        g,
        ctx,
        "screenplay.export",
        "screenplay/Low Tide Radio - Blue Revision.docx",
        json!({ "draftId": blue, "format": "docx", "options": opts(true, true, false, true, true) }),
    );
    export(
        g,
        ctx,
        "screenplay.export",
        "screenplay/Low Tide Radio - Blue Revision.txt",
        json!({ "draftId": blue, "format": "txt", "options": opts(true, true, false, false, false) }),
    );
    let doc = crate::phases::screenplay::document(g, &blue);
    let act_three: Vec<Value> = arr(&doc["scenes"])
        .iter()
        .filter(|sc| sc["number"].as_u64().unwrap_or(0) >= 36)
        .map(|sc| sc["id"].clone())
        .collect();
    let mut o = opts(false, true, false, false, false);
    o["sceneIds"] = json!(act_three);
    export(
        g,
        ctx,
        "screenplay.export",
        "screenplay/Low Tide Radio - Act Three only.pdf",
        json!({ "draftId": blue, "format": "pdf", "options": o }),
    );
    export(
        g,
        ctx,
        "screenplay.export",
        "screenplay/Low Tide Radio - First Draft.pdf",
        json!({ "draftId": ctx.id("draftFirst"), "format": "pdf", "options": opts(true, false, false, false, false) }),
    );

    // Round trip: every exported screenplay format is read back by the importer (preview only).
    let mut back = serde_json::Map::new();
    for (fmt, file) in [
        ("fdx", "screenplay/Low Tide Radio - Blue Revision.fdx"),
        ("docx", "screenplay/Low Tide Radio - Blue Revision.docx"),
        ("pdf", "screenplay/Low Tide Radio - Shooting Draft.pdf"),
        ("txt", "screenplay/Low Tide Radio - Blue Revision.txt"),
        (
            "fountain",
            "screenplay/Low Tide Radio - Blue Revision.fountain",
        ),
    ] {
        let p = g.ok(
            "screenplay.import_preview",
            json!({ "path": ctx.outp(file), "pastedText": null, "format": null }),
        );
        back.insert(fmt.into(), json!({ "scenes": p["sceneCount"], "elements": p["elementCount"], "characters": p["characterCount"], "pages": p["approxPages"], "confidence": p["confidence"], "warnings": arr(&p["warnings"]).len() }));
    }
    ctx.facts
        .insert("exportReadBack".into(), Value::Object(back));

    g.phase("Exports: story");
    export(
        g,
        ctx,
        "story.export_board",
        "story/Story outline.pdf",
        json!({ "format": "pdf", "layout": "outline", "scope": "board", "includeParking": true, "includeNotes": true, "includeComments": true }),
    );
    export(
        g,
        ctx,
        "story.export_board",
        "story/Beat board cards.pdf",
        json!({ "format": "pdf", "layout": "board", "scope": "board" }),
    );
    export(
        g,
        ctx,
        "story.export_board",
        "story/Story outline.txt",
        json!({ "format": "txt", "scope": "project", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        "story.export_board",
        "story/Story outline.csv",
        json!({ "format": "csv", "scope": "board" }),
    );
    export(
        g,
        ctx,
        "story.export_board",
        "story/Act Three outline.pdf",
        json!({ "format": "pdf", "scope": "selected", "itemIds": [ctx.id("act3")], "includeNotes": true }),
    );

    g.phase("Exports: production");
    export(
        g,
        ctx,
        "breakdown.export_report",
        "breakdown/Breakdown sheets and catalog.pdf",
        json!({ "format": "pdf", "content": "both", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        "breakdown.export_report",
        "breakdown/Breakdown summary.pdf",
        json!({ "format": "pdf", "content": "summary" }),
    );
    export(
        g,
        ctx,
        "breakdown.export_report",
        "breakdown/Breakdown.xlsx",
        json!({ "format": "xlsx", "content": "both", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        "breakdown.export_report",
        "breakdown/Breakdown scenes.csv",
        json!({ "format": "csv", "content": "scenes" }),
    );
    export(
        g,
        ctx,
        "breakdown.export_report",
        "breakdown/Catalog.csv",
        json!({ "format": "csv", "content": "catalog" }),
    );
    export(
        g,
        ctx,
        "budget.export_summary",
        "budget/Budget.pdf",
        json!({ "format": "pdf", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        "budget.export_summary",
        "budget/Budget.xlsx",
        json!({ "format": "xlsx", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        "budget.export_summary",
        "budget/Budget.csv",
        json!({ "format": "csv" }),
    );
    export(
        g,
        ctx,
        "budget.export_summary",
        "budget/Budget v1 snapshot.pdf",
        json!({ "format": "pdf", "budgetId": ctx.id("budgetSnapshotV1") }),
    );
    export(
        g,
        ctx,
        "schedule.export_schedule",
        "schedule/Shooting schedule.pdf",
        json!({ "format": "pdf", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        "schedule.export_schedule",
        "schedule/Shooting schedule.xlsx",
        json!({ "format": "xlsx", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        "schedule.export_schedule",
        "schedule/Shooting schedule.csv",
        json!({ "format": "csv" }),
    );
    let week1: Vec<String> = (1..=6).filter_map(|n| ctx.days.get(&n).cloned()).collect();
    export(
        g,
        ctx,
        "schedule.export_schedule",
        "schedule/Week one.pdf",
        json!({ "format": "pdf", "dayIds": week1, "includeUnscheduled": false }),
    );
    export(
        g,
        ctx,
        "schedule.export_dood",
        "schedule/Day Out of Days.pdf",
        json!({ "format": "pdf" }),
    );
    export(
        g,
        ctx,
        "schedule.export_dood",
        "schedule/Day Out of Days.xlsx",
        json!({ "format": "xlsx" }),
    );
    export(
        g,
        ctx,
        "schedule.export_dood",
        "schedule/Day Out of Days.csv",
        json!({ "format": "csv" }),
    );
    for (n, cs) in ctx.call_sheets.clone() {
        export(
            g,
            ctx,
            "callsheets.export_pdf",
            &format!("callsheets/Call sheet day {n:02}.pdf"),
            json!({ "id": cs, "format": "pdf", "includeOptional": true }),
        );
    }
    export(
        g,
        ctx,
        "sides.export_pdf",
        "sides/Sides day 1.pdf",
        json!({ "format": "pdf", "sideId": ctx.id("sidesDay1") }),
    );
    export(
        g,
        ctx,
        "sides.export_pdf",
        "sides/Sides day 9 blue pages.pdf",
        json!({ "format": "pdf", "sideId": ctx.id("sidesDay9Blue") }),
    );
    export(
        g,
        ctx,
        "sides.export_pdf",
        "sides/Sides day 9 SURESH.pdf",
        json!({ "format": "pdf", "sideId": ctx.id("sidesDay9") }),
    );
    export(
        g,
        ctx,
        "sides.export_pdf",
        "sides/Sides day 16 (live).pdf",
        json!({ "format": "pdf", "dayId": ctx.days[&16], "stripIds": [], "includeCover": true, "showDraftName": true }),
    );
    for ty in [
        "scene",
        "location",
        "cast_scene",
        "prop",
        "schedule",
        "breakdown_completeness",
    ] {
        export(
            g,
            ctx,
            "reports.export_report",
            &format!("reports/{ty}.pdf"),
            json!({ "format": "pdf", "reportType": ty }),
        );
    }
    export(
        g,
        ctx,
        "reports.export_report",
        "reports/cast_scene.xlsx",
        json!({ "format": "xlsx", "reportType": "cast_scene" }),
    );
    export(
        g,
        ctx,
        "reports.export_report",
        "reports/prop (reel).csv",
        json!({ "format": "csv", "reportType": "prop", "filter": "reel" }),
    );
    export(
        g,
        ctx,
        "reports.export_report",
        "reports/scene (saved white pages).pdf",
        json!({ "format": "pdf", "reportId": ctx.id("report:scene") }),
    );

    g.phase("Exports: visual");
    export(
        g,
        ctx,
        "moodboard.export_pdf",
        "visual/Moodboard - look of the film.pdf",
        json!({ "moodboardId": ctx.id("moodLook"), "format": "pdf", "includeTitle": true, "includeImages": true, "includeCaptions": true }),
    );
    export(
        g,
        ctx,
        "moodboard.export_pdf",
        "visual/Moodboard - look of the film (internal).pdf",
        json!({ "moodboardId": ctx.id("moodLook"), "format": "pdf", "includeInternalNotes": true }),
    );
    export(
        g,
        ctx,
        "moodboard.export_pdf",
        "visual/Moodboard - listening night.pdf",
        json!({ "moodboardId": ctx.id("moodBeach"), "format": "pdf" }),
    );
    export(
        g,
        ctx,
        "storyboard.export_sheet",
        "visual/Storyboards.pdf",
        json!({ "format": "pdf", "includeNotes": true, "columns": 3 }),
    );
    export(
        g,
        ctx,
        "storyboard.export_sheet",
        "visual/Storyboard scene 24.pdf",
        json!({ "format": "pdf", "storyboardIds": [ctx.id("board24")], "includeNotes": true, "columns": 2 }),
    );
    export(
        g,
        ctx,
        "shot.export_list",
        "visual/Shot list.pdf",
        json!({ "format": "pdf" }),
    );
    export(
        g,
        ctx,
        "shot.export_list",
        "visual/Shot list.xlsx",
        json!({ "format": "xlsx" }),
    );
    export(
        g,
        ctx,
        "shot.export_list",
        "visual/Shot list.csv",
        json!({ "format": "csv", "includeRemoved": true }),
    );

    g.phase("Exports: support");
    let path = ctx.outp("support/diagnostics.zip");
    let r = g.ok("diagnostics.export_bundle", json!({ "path": path }));
    if !r.is_null() {
        record(ctx, &path);
    }
}
