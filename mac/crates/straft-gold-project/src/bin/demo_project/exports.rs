//! Every export the app offers, written into `exports/` under the output folder.

use serde_json::{Value, json};

use crate::Ctx;
use crate::gold::{Gold, arr, s};

fn record(ctx: &mut Ctx, path: &str) {
    let mut v = ctx.facts.get("exports").cloned().unwrap_or(json!([]));
    if let Some(list) = v.as_array_mut() {
        list.push(json!(path));
    }
    ctx.facts.insert("exports".into(), v);
}

fn export(g: &Gold, ctx: &mut Ctx, op: &str, rel: &str, mut args: Value) {
    let path = ctx.outp(&format!("exports/{rel}"));
    args["path"] = json!(path);
    let r = g.ok(op, args);
    if !r.is_null() {
        let written = s(&r["path"]);
        record(ctx, if written.is_empty() { &path } else { &written });
    }
}

fn opts(title: bool, numbers: bool, notes: bool, marks: bool, info: bool) -> Value {
    json!({ "titlePage": title, "sceneNumbers": numbers, "includeNotes": notes, "revisionMarks": marks, "revisionInfo": info, "sceneIds": null })
}

pub fn run(g: &Gold, ctx: &mut Ctx) {
    g.phase("Exports: screenplay");
    let t = ctx.title.clone();
    let blue = ctx.id("draftBlue");
    let shooting = ctx.id("draftShooting");
    let blue_name = s(&ctx.script["blueDraft"]);
    let shoot_name = s(&ctx.script["shootingDraft"]);
    let sp = "screenplay.export";
    g.ok("interchange.export_preview", json!({ "draftId": blue, "options": opts(true, true, false, true, true), "pageNumbers": true }));
    export(
        g,
        ctx,
        sp,
        &format!("screenplay/{t} - {shoot_name}.pdf"),
        json!({ "draftId": shooting, "format": "pdf", "options": opts(true, true, false, false, false) }),
    );
    export(
        g,
        ctx,
        sp,
        &format!("screenplay/{t} - {blue_name} (marked).pdf"),
        json!({ "draftId": blue, "format": "pdf", "options": opts(true, true, false, true, true) }),
    );
    export(
        g,
        ctx,
        sp,
        &format!("screenplay/{t} - {blue_name}.fdx"),
        json!({ "draftId": blue, "format": "fdx", "options": opts(true, true, true, true, true) }),
    );
    export(
        g,
        ctx,
        sp,
        &format!("screenplay/{t} - {blue_name}.fountain"),
        json!({ "draftId": blue, "format": "fountain", "options": opts(true, true, true, false, true) }),
    );
    export(
        g,
        ctx,
        sp,
        &format!("screenplay/{t} - {blue_name}.docx"),
        json!({ "draftId": blue, "format": "docx", "options": opts(true, true, false, true, true) }),
    );
    export(
        g,
        ctx,
        sp,
        &format!("screenplay/{t} - {blue_name}.txt"),
        json!({ "draftId": blue, "format": "txt", "options": opts(true, true, false, false, false) }),
    );
    let doc = crate::script::document(g, &blue);
    let scenes = arr(&doc["scenes"]);
    let tail: Vec<Value> = scenes
        .iter()
        .skip(scenes.len().saturating_sub(scenes.len() / 4))
        .map(|x| x["id"].clone())
        .collect();
    let mut o = opts(false, true, false, false, false);
    o["sceneIds"] = json!(tail);
    export(
        g,
        ctx,
        sp,
        &format!("screenplay/{t} - last quarter only.pdf"),
        json!({ "draftId": blue, "format": "pdf", "options": o }),
    );
    export(
        g,
        ctx,
        sp,
        &format!("screenplay/{t} - First Draft.pdf"),
        json!({ "draftId": ctx.id("draftFirst"), "format": "pdf", "options": opts(true, false, false, false, false) }),
    );

    // Every exported screenplay format is read back by the importer (preview only).
    let mut back = serde_json::Map::new();
    for (fmt, ext, name) in [
        ("fdx", "fdx", &blue_name),
        ("docx", "docx", &blue_name),
        ("pdf", "pdf", &shoot_name),
        ("txt", "txt", &blue_name),
        ("fountain", "fountain", &blue_name),
    ] {
        let p = g.ok(
            "screenplay.import_preview",
            json!({ "path": ctx.outp(&format!("exports/screenplay/{t} - {name}.{ext}")), "pastedText": null, "format": null }),
        );
        back.insert(fmt.into(), json!({ "scenes": p["sceneCount"], "elements": p["elementCount"], "characters": p["characterCount"], "pages": p["approxPages"], "confidence": p["confidence"], "warnings": arr(&p["warnings"]).len() }));
    }
    ctx.facts
        .insert("exportReadBack".into(), Value::Object(back));

    g.phase("Exports: story");
    let sb = "story.export_board";
    export(
        g,
        ctx,
        sb,
        "story/Story outline.pdf",
        json!({ "format": "pdf", "layout": "outline", "scope": "board", "includeParking": true, "includeNotes": true, "includeComments": true }),
    );
    export(
        g,
        ctx,
        sb,
        "story/Beat board cards.pdf",
        json!({ "format": "pdf", "layout": "board", "scope": "board" }),
    );
    export(
        g,
        ctx,
        sb,
        "story/Story outline.txt",
        json!({ "format": "txt", "scope": "project", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        sb,
        "story/Story outline.csv",
        json!({ "format": "csv", "scope": "board" }),
    );
    let last_act = format!("act{}", arr(&ctx.story["acts"]).len());
    export(
        g,
        ctx,
        sb,
        "story/Last act outline.pdf",
        json!({ "format": "pdf", "scope": "selected", "itemIds": [ctx.id(&last_act)], "includeNotes": true }),
    );
    export(
        g,
        ctx,
        "story.page.export",
        "story/Story page.pdf",
        json!({ "format": "pdf" }),
    );
    export(
        g,
        ctx,
        "story.page.export",
        "story/Story page.docx",
        json!({ "format": "docx" }),
    );

    g.phase("Exports: production");
    let br = "breakdown.export_report";
    export(
        g,
        ctx,
        br,
        "breakdown/Breakdown sheets and catalog.pdf",
        json!({ "format": "pdf", "content": "both", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        br,
        "breakdown/Breakdown summary.pdf",
        json!({ "format": "pdf", "content": "summary" }),
    );
    export(
        g,
        ctx,
        br,
        "breakdown/Breakdown.xlsx",
        json!({ "format": "xlsx", "content": "both", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        br,
        "breakdown/Breakdown scenes.csv",
        json!({ "format": "csv", "content": "scenes" }),
    );
    export(
        g,
        ctx,
        br,
        "breakdown/Catalog.csv",
        json!({ "format": "csv", "content": "catalog" }),
    );
    let bu = "budget.export_summary";
    export(
        g,
        ctx,
        bu,
        "budget/Budget.pdf",
        json!({ "format": "pdf", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        bu,
        "budget/Budget.xlsx",
        json!({ "format": "xlsx", "includeNotes": true }),
    );
    export(g, ctx, bu, "budget/Budget.csv", json!({ "format": "csv" }));
    export(
        g,
        ctx,
        bu,
        "budget/Budget v1 snapshot.pdf",
        json!({ "format": "pdf", "budgetId": ctx.id("budgetSnapshotV1") }),
    );
    let se = "schedule.export_schedule";
    export(
        g,
        ctx,
        se,
        "schedule/Shooting schedule.pdf",
        json!({ "format": "pdf", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        se,
        "schedule/Shooting schedule.xlsx",
        json!({ "format": "xlsx", "includeNotes": true }),
    );
    export(
        g,
        ctx,
        se,
        "schedule/Shooting schedule.csv",
        json!({ "format": "csv" }),
    );
    let fo = ctx.production["schedule"]["focus"].clone();
    let week: Vec<String> = arr(&fo["firstWeekDays"])
        .iter()
        .filter_map(|n| ctx.days.get(&(n.as_u64().unwrap_or(0) as u32)).cloned())
        .collect();
    export(
        g,
        ctx,
        se,
        "schedule/Week one.pdf",
        json!({ "format": "pdf", "dayIds": week, "includeUnscheduled": false }),
    );
    let dd = "schedule.export_dood";
    export(
        g,
        ctx,
        dd,
        "schedule/Day Out of Days.pdf",
        json!({ "format": "pdf" }),
    );
    export(
        g,
        ctx,
        dd,
        "schedule/Day Out of Days.xlsx",
        json!({ "format": "xlsx" }),
    );
    export(
        g,
        ctx,
        dd,
        "schedule/Day Out of Days.csv",
        json!({ "format": "csv" }),
    );
    let daily_day = ctx
        .days
        .get(&(fo["dailyDay"].as_u64().unwrap_or(0) as u32))
        .cloned();
    export(
        g,
        ctx,
        "schedule.export_daily_view",
        "schedule/Daily View.pdf",
        json!({ "format": "pdf", "dayId": daily_day }),
    );
    export(
        g,
        ctx,
        "schedule.export_daily_view",
        "schedule/Daily View.txt",
        json!({ "format": "txt", "dayId": daily_day }),
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
    let si = "sides.export_pdf";
    export(
        g,
        ctx,
        si,
        "sides/Sides first.pdf",
        json!({ "format": "pdf", "sideId": ctx.id("sidesFirst") }),
    );
    export(
        g,
        ctx,
        si,
        "sides/Sides revision pages.pdf",
        json!({ "format": "pdf", "sideId": ctx.id("sidesRevision") }),
    );
    export(
        g,
        ctx,
        si,
        "sides/Sides for one actor.pdf",
        json!({ "format": "pdf", "sideId": ctx.id("sidesCast") }),
    );
    let last_day = ctx
        .days
        .get(&(fo["templateDay"].as_u64().unwrap_or(0) as u32))
        .cloned();
    export(
        g,
        ctx,
        si,
        "sides/Sides last day (live).pdf",
        json!({ "format": "pdf", "dayId": last_day, "stripIds": [], "includeCover": true, "showDraftName": true }),
    );
    let rp = "reports.export_report";
    for r in arr(&ctx.production["reports"]) {
        let ty = s(&r["type"]);
        export(
            g,
            ctx,
            rp,
            &format!("reports/{ty}.pdf"),
            json!({ "format": "pdf", "reportType": ty }),
        );
    }
    export(
        g,
        ctx,
        rp,
        "reports/cast_scene.xlsx",
        json!({ "format": "xlsx", "reportType": "cast_scene" }),
    );
    let filter = ctx.breakdown["reportFilterProp"].clone();
    export(
        g,
        ctx,
        rp,
        "reports/prop (filtered).csv",
        json!({ "format": "csv", "reportType": "prop", "filter": filter }),
    );
    export(
        g,
        ctx,
        rp,
        "reports/scene (saved white pages).pdf",
        json!({ "format": "pdf", "reportId": ctx.id("report:scene") }),
    );

    g.phase("Exports: visual");
    let mb = "moodboard.export_pdf";
    export(
        g,
        ctx,
        mb,
        "visual/Moodboard - look of the film.pdf",
        json!({ "moodboardId": ctx.id("mood:0"), "format": "pdf", "includeTitle": true, "includeImages": true, "includeCaptions": true }),
    );
    export(
        g,
        ctx,
        mb,
        "visual/Moodboard - look of the film (internal).pdf",
        json!({ "moodboardId": ctx.id("mood:0"), "format": "pdf", "includeInternalNotes": true }),
    );
    export(
        g,
        ctx,
        mb,
        "visual/Moodboard - scene board.pdf",
        json!({ "moodboardId": ctx.id("mood:1"), "format": "pdf" }),
    );
    let sy = "storyboard.export_sheet";
    export(
        g,
        ctx,
        sy,
        "visual/Storyboards.pdf",
        json!({ "format": "pdf", "includeNotes": true, "columns": 3 }),
    );
    export(
        g,
        ctx,
        sy,
        "visual/Storyboard key scene.pdf",
        json!({ "format": "pdf", "storyboardIds": [ctx.id("boardFocus")], "includeNotes": true, "columns": 2 }),
    );
    let sh = "shot.export_list";
    export(
        g,
        ctx,
        sh,
        "visual/Shot list.pdf",
        json!({ "format": "pdf" }),
    );
    export(
        g,
        ctx,
        sh,
        "visual/Shot list.xlsx",
        json!({ "format": "xlsx" }),
    );
    export(
        g,
        ctx,
        sh,
        "visual/Shot list.csv",
        json!({ "format": "csv", "includeRemoved": true }),
    );

    g.phase("Exports: support");
    let path = ctx.outp("exports/support/diagnostics.zip");
    let r = g.ok("diagnostics.export_bundle", json!({ "path": path }));
    if !r.is_null() {
        record(ctx, &path);
    }
}
