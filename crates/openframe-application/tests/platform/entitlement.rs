//! Licence and free-trial status in the app (build plan area "Payment and free trial in all three
//! channels", items "Shared entitlement interface" and "No data loss on expiry"; ADR-0018,
//! ADR-0022 and ADR-0023).
//!
//! The stores own the 7-day trial: the Microsoft Store through its licence (a fake
//! `StoreLicenceSource` here, the real `GetAppLicenseAsync` needs the Partner Center identity),
//! the Mac through StoreKit and the licensing server (a fake StoreKit bridge here). The Windows
//! installer is an internal build that is never sold. In either store build an ended trial or a
//! refund locks every project operation in Rust until a purchase or a restore; nothing is
//! deleted.

use std::path::Path;
use std::sync::{Arc, Mutex};

use openframe_application::events::RecordingSink;
use openframe_application::modules::ai::toolbox;
use openframe_application::registry::{self, AiExposure, OpClass};
use openframe_application::{AppConfig, AppCore};
use openframe_domain::auth::ActorOrigin;
use openframe_domain::{Actor, Role};
use openframe_entitlement::Channel;
use openframe_entitlement::adapters::{
    StoreAppLicence, StoreLicenceError, StoreLicenceSource, StorePurchaseStatus,
};
use serde_json::{Value, json};

const DAY_SECONDS: u64 = 24 * 60 * 60;

/// A Microsoft Store whose licence answer and purchase result the test sets.
#[derive(Clone)]
struct FakeMsStore {
    licence: Arc<Mutex<Result<StoreAppLicence, StoreLicenceError>>>,
    purchase: Arc<Mutex<StorePurchaseStatus>>,
    after_purchase: Arc<Mutex<Option<StoreAppLicence>>>,
}

impl FakeMsStore {
    fn new(licence: Result<StoreAppLicence, StoreLicenceError>) -> Self {
        Self {
            licence: Arc::new(Mutex::new(licence)),
            purchase: Arc::new(Mutex::new(StorePurchaseStatus::NotPurchased)),
            after_purchase: Arc::new(Mutex::new(None)),
        }
    }
    fn set(&self, licence: Result<StoreAppLicence, StoreLicenceError>) {
        *self.licence.lock().unwrap() = licence;
    }
}

impl StoreLicenceSource for FakeMsStore {
    fn app_licence(&self) -> Result<StoreAppLicence, StoreLicenceError> {
        *self.licence.lock().unwrap()
    }
    fn request_purchase(&self) -> StorePurchaseStatus {
        if let Some(next) = self.after_purchase.lock().unwrap().take() {
            *self.licence.lock().unwrap() = Ok(next);
        }
        *self.purchase.lock().unwrap()
    }
}

const fn trial(days: u64) -> StoreAppLicence {
    StoreAppLicence {
        is_active: true,
        is_trial: true,
        trial_seconds_left: days * DAY_SECONDS,
    }
}
const ENDED: StoreAppLicence = StoreAppLicence {
    is_active: false,
    is_trial: true,
    trial_seconds_left: 0,
};
const BOUGHT: StoreAppLicence = StoreAppLicence {
    is_active: true,
    is_trial: false,
    trial_seconds_left: 0,
};

fn start(config: AppConfig) -> Arc<AppCore> {
    AppCore::new(config, Arc::new(RecordingSink::default())).expect("startup never fails")
}

fn ms_core(root: &Path, store: &FakeMsStore) -> Arc<AppCore> {
    start(AppConfig {
        channel: Channel::MicrosoftStore,
        microsoft_store: Some(Box::new(store.clone())),
        ..AppConfig::isolated(root)
    })
}

fn internal_core(root: &Path) -> Arc<AppCore> {
    start(AppConfig {
        channel: Channel::DirectInstaller,
        ..AppConfig::isolated(root)
    })
}

fn call(core: &AppCore, op: &str, args: Value) -> Value {
    core.dispatch(&core.local_actor(), op, args)
        .unwrap_or_else(|e| panic!("{op} failed: {} ({:?})", e.code_str(), e.detail))
}

fn refused(core: &AppCore, op: &str, args: Value) -> String {
    core.dispatch(&core.local_actor(), op, args)
        .map(|v| panic!("{op} ran while locked: {v}"))
        .unwrap_err()
        .code_str()
        .to_string()
}

fn status(core: &AppCore) -> Value {
    call(core, "entitlement.status", json!({}))
}

fn stop(core: Arc<AppCore>) {
    core.shutdown();
    drop(core);
}

fn trial_file(root: &Path) -> std::path::PathBuf {
    root.join("appdata").join("entitlement").join("trial.json")
}

/// A project with a card in it; returns its folder.
fn make_project(core: &AppCore, title: &str, card: &str) -> String {
    call(
        core,
        "project.create",
        json!({ "title": title, "projectType": "Feature Film" }),
    );
    let act = call(core, "story.create_act", json!({ "title": "Act One" }));
    call(
        core,
        "story.create_card",
        json!({ "parent": { "parentType": "act", "parentId": act["id"] },
                "shortDescription": card }),
    );
    core.project()
        .unwrap()
        .layout
        .root()
        .to_string_lossy()
        .into_owned()
}

/// Open `path`, read the board and export it: the three things a lock must refuse.
fn open_read_export(core: &AppCore, root: &Path, path: &str, card: &str) {
    call(core, "project.open", json!({ "path": path }));
    assert!(
        call(core, "story.board", json!({}))
            .to_string()
            .contains(card)
    );
    let out = root.join("exports");
    std::fs::create_dir_all(&out).unwrap();
    let pdf = out.join(format!("{card}.pdf"));
    let r = call(
        core,
        "story.export_board",
        json!({ "format": "pdf", "path": pdf.to_string_lossy() }),
    );
    assert_eq!(r["format"], "pdf");
    assert!(pdf.is_file(), "export written");
}

/// Every kind of project operation is refused with the licence code, and the lock screen's own
/// operations still answer.
fn assert_locked(core: &AppCore, path: &str) {
    for (op, args) in [
        ("project.open", json!({ "path": path })),
        (
            "project.create",
            json!({ "title": "New", "projectType": "Feature Film" }),
        ),
        ("project.list_recent", json!({})),
        ("project.current", json!({})),
        ("story.board", json!({})),
        (
            "story.export_board",
            json!({ "format": "pdf", "path": "C:/never.pdf" }),
        ),
        // Breakdown, catalog, locations, cast and crew (area 13): reads, edits and the
        // reference check before a delete are all behind the same gate.
        ("breakdown.categories", json!({})),
        (
            "breakdown.set_category_style",
            json!({ "category": "Props", "color": "blue" }),
        ),
        ("breakdown.scenes", json!({ "filter": "missing" })),
        (
            "breakdown.reuse",
            json!({ "sceneIds": ["s1"], "catalogItemIds": ["c1"] }),
        ),
        ("breakdown.source_anchor", json!({ "id": "e1" })),
        (
            "availability.set",
            json!({ "ownerType": "cast_member", "ownerId": "c1", "windows": [] }),
        ),
        (
            "production.references",
            json!({ "kind": "location", "id": "l1" }),
        ),
        // Shooting schedule and Day Out of Days (area 14): the day times, strip start, the
        // derived report, its corrections and its export.
        (
            "schedule.set_day_times",
            json!({ "dayId": "d1", "crewCall": 420 }),
        ),
        (
            "schedule.set_strip_start",
            json!({ "stripId": "s1", "minutes": 480 }),
        ),
        ("schedule.dood", json!({})),
        (
            "schedule.set_dood_override",
            json!({ "scheduleId": "x", "castKey": "char:a", "dayId": "d1", "code": "H" }),
        ),
        (
            "schedule.export_dood",
            json!({ "format": "pdf", "path": "C:/never.pdf" }),
        ),
        (
            "schedule.export_daily_view",
            json!({ "format": "pdf", "path": "C:/never.pdf" }),
        ),
        // Call sheets, budget, templates, samples and exports (area 15).
        (
            "callsheets.add_section",
            json!({ "id": "cs1", "section": "weather" }),
        ),
        ("callsheets.refresh_preview", json!({ "id": "cs1" })),
        (
            "budget.add_line",
            json!({ "budgetId": "b1", "category": "Food", "description": "Catering", "planned": 100, "actual": 90 }),
        ),
        (
            "templates.apply",
            json!({ "scope": "builtin", "id": "builtin:budget" }),
        ),
        ("screenplay.samples", json!({})),
        (
            "screenplay.import_preview",
            json!({ "sample": "example-short" }),
        ),
        (
            "budget.export_summary",
            json!({ "format": "pdf", "path": "C:/never.pdf" }),
        ),
    ] {
        assert_eq!(
            refused(core, op, args),
            "permission.licence_required",
            "{op}"
        );
    }
    for op in [
        "entitlement.status",
        "entitlement.notice",
        "app.info",
        "appearance.get",
    ] {
        call(core, op, json!({}));
    }
    // The assistant and packages are refused too: the gate sits on the one dispatch path.
    let ai = Actor {
        origin: ActorOrigin::Ai {
            request_id: "r1".into(),
        },
        ..core.local_actor()
    };
    let e = core.dispatch(&ai, "story.board", json!({})).unwrap_err();
    assert_eq!(e.code_str(), "permission.licence_required");
}

// ------------------------------------------------------------------ the internal installer build

#[test]
fn plat_009_the_internal_installer_build_is_never_sold_and_never_locks() {
    let dir = tempfile::tempdir().unwrap();
    let core = internal_core(dir.path());
    let s = status(&core);
    assert_eq!(s["channel"], "directInstaller");
    assert_eq!(s["state"], json!({ "state": "internalBuild" }));
    assert_eq!(s["summary"], "Internal build, not for sale.");
    assert_eq!(s["canEdit"], true);
    assert_eq!(s["limitation"], Value::Null);
    assert_eq!(
        call(
            &core,
            "entitlement.notice",
            json!({ "today": "2027-01-15" })
        )["notice"],
        json!({ "kind": "none" })
    );
    let r = call(&core, "entitlement.buy", json!({}));
    assert_eq!(r["route"], json!({ "route": "unavailable" }));
    assert_eq!(
        r["message"],
        "This is an internal build and is not for sale."
    );
    assert_eq!(
        call(&core, "entitlement.restore_purchase", json!({}))["state"],
        json!({ "state": "internalBuild" })
    );
    let path = make_project(&core, "Harbour Lights", "Meena waits at the pier");
    open_read_export(&core, dir.path(), &path, "Meena waits at the pier");
    assert!(
        !trial_file(dir.path()).exists(),
        "no trial timer of its own"
    );
}

// ------------------------------------------------------------------ Microsoft Store (ADR-0023)

#[test]
fn plat_009_the_microsoft_store_trial_comes_from_the_store_licence() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(trial(5)));
    let core = ms_core(dir.path(), &store);
    let s = status(&core);
    assert_eq!(s["channel"], "microsoftStore");
    assert_eq!(
        s["state"],
        json!({ "state": "trialActive", "days_left": 5 })
    );
    assert_eq!(s["canEdit"], true);
    assert_eq!(s["limitation"], Value::Null);
    assert!(
        !trial_file(dir.path()).exists(),
        "the Store owns the trial; OpenFrame keeps no timer"
    );
    // TrialTimeRemaining moves on; the shell asks again on OfflineLicensesChanged.
    store.set(Ok(trial(2)));
    core.entitlement.refresh();
    assert_eq!(
        status(&core)["state"],
        json!({ "state": "trialActive", "days_left": 2 })
    );
    assert_eq!(
        core.entitlement.lease(),
        None,
        "no lease: the Store tells us"
    );
}

#[test]
fn plat_009_the_microsoft_store_trial_over_locks_every_project_operation_until_bought() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(trial(1)));
    let core = ms_core(dir.path(), &store);
    let card = "Ravi waits for the last ferry";
    let path = make_project(&core, "Monsoon Ferry", card);
    stop(core);

    // The trial ran out while OpenFrame was closed.
    store.set(Ok(ENDED));
    let core = ms_core(dir.path(), &store);
    let s = status(&core);
    assert_eq!(s["state"], json!({ "state": "trialEnded" }));
    assert_eq!(s["canEdit"], false);
    assert_eq!(
        s["summary"],
        "Your free trial has ended. Buy OpenFrame Studio or restore your purchase to open your projects again. Nothing is deleted."
    );
    assert_eq!(
        call(
            &core,
            "entitlement.notice",
            json!({ "today": "2027-01-23" })
        )["notice"],
        json!({ "kind": "locked" })
    );
    assert_locked(&core, &path);
    // The window asks about the team password before anything else on every launch: a store
    // package answers "not needed" under the lock too, so the lock screen is what shows.
    let gate = call(&core, "team_gate.status", json!({}));
    assert_eq!(gate["required"], false, "{gate}");
    assert_eq!(gate["unlocked"], true, "{gate}");

    // The Store dialog is closed without buying: still locked, no failure shown.
    let r = call(&core, "entitlement.buy", json!({}));
    assert_eq!(r["route"], json!({ "route": "store" }));
    assert_eq!(r["message"], Value::Null);
    assert_locked(&core, &path);
    // Offline in the Store dialog: plain words, still locked.
    *store.purchase.lock().unwrap() = StorePurchaseStatus::NetworkError;
    let r = call(&core, "entitlement.buy", json!({}));
    assert!(
        r["message"]
            .as_str()
            .unwrap()
            .contains("could not reach the Microsoft Store"),
        "{r}"
    );
    // Bought: everything works again, exactly as it was.
    *store.purchase.lock().unwrap() = StorePurchaseStatus::Succeeded;
    *store.after_purchase.lock().unwrap() = Some(BOUGHT);
    let r = call(&core, "entitlement.buy", json!({}));
    assert_eq!(r["status"]["state"], json!({ "state": "purchased" }));
    open_read_export(&core, dir.path(), &path, card);
    assert_eq!(
        call(
            &core,
            "entitlement.notice",
            json!({ "today": "2027-01-23" })
        )["notice"],
        json!({ "kind": "dataNote" })
    );
    stop(core);

    // Bought on another computer: "Restore purchase" reads the Store licence again.
    store.set(Ok(ENDED));
    let core = ms_core(dir.path(), &store);
    assert_locked(&core, &path);
    store.set(Ok(BOUGHT));
    let s = call(&core, "entitlement.restore_purchase", json!({}));
    assert_eq!(s["state"], json!({ "state": "purchased" }));
    open_read_export(&core, dir.path(), &path, card);
}

#[test]
fn plat_009_the_store_lock_refuses_deleting_or_restoring_a_screenplay() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(trial(1)));
    let core = ms_core(dir.path(), &store);
    let path = make_project(&core, "Monsoon Ferry", "Ravi waits for the last ferry");
    call(&core, "screenplay.create", json!({}));
    let short = call(&core, "screenplay.create", json!({ "title": "The Short" }));
    let sid = short["screenplay"]["id"].as_str().unwrap().to_string();
    stop(core);

    store.set(Ok(ENDED));
    let core = ms_core(dir.path(), &store);
    for (op, args) in [
        ("screenplay.delete", json!({ "screenplayId": sid })),
        ("trash.restore", json!({ "id": "any" })),
        ("project.deleted_items", json!({})),
    ] {
        assert_eq!(
            refused(&core, op, args),
            "permission.licence_required",
            "{op}"
        );
    }
    // Bought: both screenplays are still there.
    store.set(Ok(BOUGHT));
    call(&core, "entitlement.restore_purchase", json!({}));
    call(&core, "project.open", json!({ "path": path }));
    let ov = call(&core, "screenplay.overview", json!({}));
    assert_eq!(ov["screenplays"].as_array().unwrap().len(), 2);
    call(&core, "screenplay.delete", json!({ "screenplayId": sid }));
}

#[test]
fn plat_009_a_microsoft_store_check_that_cannot_answer_keeps_open_and_export() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(trial(4)));
    let core = ms_core(dir.path(), &store);
    let path = make_project(&core, "Night Bus", "Asha misses the night bus");
    stop(core);

    // Offline and no cached licence: a check is needed. Editing locks, nothing else does.
    store.set(Err(StoreLicenceError::Unreachable));
    let core = ms_core(dir.path(), &store);
    let s = status(&core);
    assert_eq!(s["state"], json!({ "state": "needsOnlineCheck" }));
    assert_eq!(s["canEdit"], false);
    assert!(s["summary"].as_str().unwrap().contains("Microsoft Store"));
    assert_eq!(
        call(
            &core,
            "entitlement.notice",
            json!({ "today": "2027-01-17" })
        )["notice"],
        json!({ "kind": "checkNeeded" })
    );
    open_read_export(&core, dir.path(), &path, "Asha misses the night bus");
}

#[test]
fn plat_009_a_microsoft_store_build_without_the_store_calls_needs_a_check_and_cannot_sell() {
    let dir = tempfile::tempdir().unwrap();
    let core = start(AppConfig {
        channel: Channel::MicrosoftStore,
        ..AppConfig::isolated(dir.path())
    });
    let s = status(&core);
    assert_eq!(s["state"], json!({ "state": "needsOnlineCheck" }));
    assert_eq!(
        s["limitation"],
        "The Microsoft Store connection is not available in this build."
    );
    let r = call(&core, "entitlement.buy", json!({}));
    assert_eq!(r["route"], json!({ "route": "unavailable" }));
    assert_eq!(
        r["message"],
        "The Microsoft Store connection is not available in this build."
    );
    // A build gap, not a lock: projects still open.
    call(
        &core,
        "project.create",
        json!({ "title": "Still Works", "projectType": "Short Film" }),
    );
}

// ------------------------------------------------------------------ Mac App Store (ADR-0022)

#[test]
fn plat_014_mac_channel_without_the_store_bridge_needs_an_online_check_and_keeps_open_and_export() {
    let dir = tempfile::tempdir().unwrap();
    let mac_core = || {
        start(AppConfig {
            channel: Channel::MacAppStore,
            ..AppConfig::isolated(dir.path())
        })
    };
    let core = mac_core();
    let path = make_project(&core, "Harbour Lights", "Meena waits at the pier");
    stop(core);

    // Next launch: no StoreKit bridge, so nothing is granted and the trial is not guessed. This
    // is a gap in today's build, not a state a user of a finished Mac build sees.
    let core = mac_core();
    let s = status(&core);
    assert_eq!(s["channel"], "macAppStore");
    assert_eq!(s["state"], json!({ "state": "needsOnlineCheck" }));
    assert_eq!(s["canEdit"], false);
    assert_eq!(
        s["limitation"],
        "The App Store connection is not available in this build."
    );
    assert!(
        s["summary"].as_str().unwrap().contains("can be exported"),
        "{s}"
    );
    let restored = call(&core, "entitlement.restore_purchase", json!({}));
    assert_eq!(restored["state"], json!({ "state": "needsOnlineCheck" }));
    assert!(!trial_file(dir.path()).exists());
    open_read_export(&core, dir.path(), &path, "Meena waits at the pier");
    stop(core);
}

/// A StoreKit bridge whose transactions and purchase answer the test sets, counting the asks.
#[derive(Clone)]
struct FakeBridge {
    entitlements: Arc<Mutex<Vec<openframe_entitlement::mac::VerifiedTransaction>>>,
    on_purchase: Arc<Mutex<Option<openframe_entitlement::mac::VerifiedTransaction>>>,
    asked: Arc<std::sync::atomic::AtomicUsize>,
}

fn lifetime_tx(id: &str, revoked: bool) -> openframe_entitlement::mac::VerifiedTransaction {
    use openframe_entitlement::mac::{LIFETIME_PRODUCT_ID, StoreEnvironment, VerifiedTransaction};
    VerifiedTransaction {
        product_id: LIFETIME_PRODUCT_ID.to_string(),
        transaction_id: id.to_string(),
        original_transaction_id: id.to_string(),
        app_transaction_id: None,
        purchase_date_ms: 1_800_000_000_000,
        revocation_date_ms: revoked.then_some(1_800_100_000_000),
        environment: StoreEnvironment::Production,
        signed_jws: "aGVhZA.Ym9keQ.c2ln".to_string(),
    }
}

impl FakeBridge {
    fn new(entitlements: Vec<openframe_entitlement::mac::VerifiedTransaction>) -> Self {
        Self {
            entitlements: Arc::new(Mutex::new(entitlements)),
            on_purchase: Arc::new(Mutex::new(None)),
            asked: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
}

impl openframe_entitlement::mac::StoreKitBridge for FakeBridge {
    fn current_entitlements(
        &self,
    ) -> Result<
        Vec<openframe_entitlement::mac::VerifiedTransaction>,
        openframe_entitlement::mac::BridgeError,
    > {
        self.asked.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(self.entitlements.lock().unwrap().clone())
    }
    fn purchase(
        &self,
        _product_id: &str,
    ) -> Result<openframe_entitlement::mac::PurchaseOutcome, openframe_entitlement::mac::BridgeError>
    {
        match self.on_purchase.lock().unwrap().take() {
            Some(tx) => {
                self.entitlements.lock().unwrap().push(tx.clone());
                Ok(openframe_entitlement::mac::PurchaseOutcome::Success(tx))
            }
            None => Err(openframe_entitlement::mac::BridgeError::Other),
        }
    }
    fn app_transaction(
        &self,
    ) -> Result<
        openframe_entitlement::mac::AppTransactionInfo,
        openframe_entitlement::mac::BridgeError,
    > {
        Err(openframe_entitlement::mac::BridgeError::StoreUnavailable)
    }
    fn environment(&self) -> openframe_entitlement::mac::StoreEnvironment {
        openframe_entitlement::mac::StoreEnvironment::Production
    }
    fn product_info(
        &self,
        _product_id: &str,
    ) -> Result<openframe_entitlement::mac::ProductInfo, openframe_entitlement::mac::BridgeError>
    {
        Err(openframe_entitlement::mac::BridgeError::ProductNotFound)
    }
}

fn mac_core_with(root: &Path, bridge: &FakeBridge) -> Arc<AppCore> {
    use openframe_entitlement::mac::{InstantClock, MacInputs, NoNonces, UnavailableServer};
    start(AppConfig {
        channel: Channel::MacAppStore,
        mac_inputs: Some(MacInputs {
            bridge: Box::new(bridge.clone()),
            server: Box::new(UnavailableServer),
            nonces: Box::new(NoNonces),
            clock: Box::new(InstantClock::new()),
            response_key: None,
        }),
        ..AppConfig::isolated(root)
    })
}

#[test]
fn plat_014_a_mac_refund_locks_open_edit_and_export_in_rust_and_a_purchase_unlocks() {
    use openframe_entitlement::mac::RevalidateReason;
    let dir = tempfile::tempdir().unwrap();
    let bridge = FakeBridge::new(vec![lifetime_tx("2000000031", false)]);
    let core = mac_core_with(dir.path(), &bridge);
    core.entitlement.revalidate(RevalidateReason::Launch);
    assert_eq!(status(&core)["state"], json!({ "state": "purchased" }));
    let card = "Meena waits at the pier";
    let path = make_project(&core, "Harbour Lights", card);

    // Apple refunds the purchase; StoreKit reports it and the lock holds at once.
    *bridge.entitlements.lock().unwrap() = vec![lifetime_tx("2000000031", true)];
    core.entitlement
        .revalidate(RevalidateReason::StoreKitChanged);
    let s = status(&core);
    assert_eq!(s["state"], json!({ "state": "revoked" }));
    assert_eq!(s["canEdit"], false);
    assert_eq!(
        call(
            &core,
            "entitlement.notice",
            json!({ "today": "2027-01-15" })
        )["notice"],
        json!({ "kind": "locked" })
    );
    assert_locked(&core, &path);
    // "Restore purchase" finds nothing new: still locked.
    call(&core, "entitlement.restore_purchase", json!({}));
    assert_locked(&core, &path);

    // Buying again through Apple's sheet unlocks everything, exactly as it was.
    *bridge.on_purchase.lock().unwrap() = Some(lifetime_tx("2000000032", false));
    let r = call(&core, "entitlement.buy", json!({}));
    assert_eq!(r["route"], json!({ "route": "store" }));
    assert_eq!(r["status"]["state"], json!({ "state": "purchased" }));
    open_read_export(&core, dir.path(), &path, card);
}

#[test]
fn plat_014_a_mac_trial_that_ended_locks_open_edit_and_export_and_a_restore_unlocks() {
    // The licensing server that dates the trial is not part of this test (the Mac module's own
    // tests cover its verdict); the state is set by hand, which the lock treats the same way.
    let dir = tempfile::tempdir().unwrap();
    let core = start(AppConfig {
        entitlement_debug_controls: true,
        ..AppConfig::isolated(dir.path())
    });
    let card = "Ira counts the lanterns";
    let path = make_project(&core, "Lantern Night", card);
    for state in ["trialEnded", "revoked"] {
        call(
            &core,
            "entitlement.debug_set",
            json!({ "channel": "macAppStore", "state": { "state": state } }),
        );
        assert_locked(&core, &path);
        // The first question of every launch still has an answer, so the lock screen shows.
        assert_eq!(
            call(&core, "team_gate.status", json!({}))["required"],
            false
        );
        // The Microsoft Store locks the same way (ADR-0023); the internal build never does.
        call(
            &core,
            "entitlement.debug_set",
            json!({ "channel": "microsoftStore", "state": { "state": state } }),
        );
        assert_locked(&core, &path);
        call(
            &core,
            "entitlement.debug_set",
            json!({ "channel": "directInstaller", "state": { "state": state } }),
        );
        open_read_export(&core, dir.path(), &path, card);
    }
    // A purchase restored: everything answers again.
    call(
        &core,
        "entitlement.debug_set",
        json!({ "channel": "macAppStore", "state": { "state": "purchased" } }),
    );
    open_read_export(&core, dir.path(), &path, card);
    // A failed online check never locks opening or exporting (ADR-0018 decision 6).
    call(
        &core,
        "entitlement.debug_set",
        json!({ "channel": "macAppStore", "state": { "state": "needsOnlineCheck" } }),
    );
    open_read_export(&core, dir.path(), &path, card);
}

#[test]
fn plat_014_with_the_store_bridge_startup_asks_nothing_and_the_background_check_decides() {
    use openframe_entitlement::mac::RevalidateReason;
    use std::sync::atomic::Ordering;

    let dir = tempfile::tempdir().unwrap();
    let bridge = FakeBridge::new(vec![lifetime_tx("2000000009", false)]);
    let config = AppConfig {
        channel: Channel::MacAppStore,
        ..AppConfig::isolated(dir.path())
    };
    assert!(
        !format!("{config:?}").contains("2000000009"),
        "the config's debug output prints no store data"
    );
    let core = mac_core_with(dir.path(), &bridge);

    // Startup runs on the main thread: StoreKit and the server are not asked there.
    assert_eq!(bridge.asked.load(Ordering::SeqCst), 0);
    let s = status(&core);
    assert_eq!(s["state"], json!({ "state": "needsOnlineCheck" }));
    assert_eq!(s["canEdit"], false);
    assert_eq!(s["limitation"], Value::Null, "the adapter is not built yet");
    assert_eq!(
        core.entitlement.lease(),
        Some(std::time::Duration::from_secs(15 * 60)),
        "a check that has not answered is retried soon"
    );

    // The shell's background check answers.
    let state = core.entitlement.revalidate(RevalidateReason::Launch);
    assert_eq!(state, openframe_entitlement::EntitlementState::Purchased);
    assert_eq!(bridge.asked.load(Ordering::SeqCst), 1);
    let s = status(&core);
    assert_eq!(s["state"], json!({ "state": "purchased" }));
    assert_eq!(s["canEdit"], true);
    assert_eq!(
        core.entitlement.lease(),
        Some(std::time::Duration::from_secs(24 * 60 * 60))
    );

    // A StoreKit change and "Restore purchase" ask StoreKit again; nothing is cached as a grant.
    core.entitlement
        .revalidate(RevalidateReason::StoreKitChanged);
    call(&core, "entitlement.restore_purchase", json!({}));
    assert_eq!(bridge.asked.load(Ordering::SeqCst), 3);
    assert!(!trial_file(dir.path()).exists());
    stop(core);

    // The Windows channels have no lease.
    let windows = internal_core(dir.path());
    assert_eq!(windows.entitlement.lease(), None);
    stop(windows);
}

// ------------------------------------------------------------------ every channel

#[test]
fn plat_009_every_project_role_can_read_the_status() {
    let dir = tempfile::tempdir().unwrap();
    let core = ms_core(dir.path(), &FakeMsStore::new(Ok(trial(3))));
    for role in [Role::Viewer, Role::Commenter, Role::ExportOnly, Role::Owner] {
        let mut who = core.local_actor();
        who.role = role;
        let s = core
            .dispatch(&who, "entitlement.status", json!({}))
            .unwrap_or_else(|e| panic!("{role:?}: {}", e.message));
        assert_eq!(s["trialDays"], 7, "{role:?}");
    }
}

#[test]
fn plat_009_only_the_person_at_this_computer_can_restore_a_purchase() {
    let dir = tempfile::tempdir().unwrap();
    let core = ms_core(dir.path(), &FakeMsStore::new(Ok(trial(3))));
    for origin in [
        ActorOrigin::Ai {
            request_id: "r1".into(),
        },
        ActorOrigin::Exchange {
            package_id: "p1".into(),
        },
    ] {
        let who = Actor {
            origin,
            ..core.local_actor()
        };
        let e = core
            .dispatch(&who, "entitlement.restore_purchase", json!({}))
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied");
    }
}

#[test]
fn plat_009_the_assistant_cannot_see_the_licence_operations() {
    let reg = registry::catalog();
    for op in ["entitlement.status", "entitlement.restore_purchase"] {
        let m = reg
            .metadata(op)
            .unwrap_or_else(|| panic!("{op} registered"));
        assert!(
            matches!(m.ai_exposure, AiExposure::Hidden(r) if r.contains("assistant")),
            "{op} hidden"
        );
        assert_eq!(m.required_capability, None, "{op} is application level");
        for (tool, _, ops) in toolbox::coverage() {
            assert!(!ops.contains(&op), "{tool} covers {op}");
        }
    }
    assert_eq!(
        reg.metadata("entitlement.status").unwrap().operation_class,
        OpClass::Read
    );
    for name in toolbox::all_tool_names() {
        assert!(
            !name.contains("entitlement") && !name.contains("licen"),
            "{name}"
        );
    }
}

#[test]
fn plat_009_the_operations_the_lock_screen_needs_are_registered_and_touch_no_project() {
    let reg = registry::catalog();
    for op in openframe_application::modules::entitlement::ALLOWED_WHILE_LOCKED {
        let m = reg
            .metadata(op)
            .unwrap_or_else(|| panic!("{op} registered"));
        assert_eq!(m.required_capability, None, "{op} is application level");
        assert_eq!(m.operation_class, OpClass::Read, "{op} only reads");
    }
}

#[test]
fn plat_009_an_unwritable_entitlement_folder_never_fails_startup_or_blocks() {
    let dir = tempfile::tempdir().unwrap();
    // A file where the entitlement folder should be, so nothing can be saved there.
    let app_data = dir.path().join("appdata");
    std::fs::create_dir_all(&app_data).unwrap();
    std::fs::write(app_data.join("entitlement"), b"not a folder").unwrap();
    for core in [
        internal_core(dir.path()),
        ms_core(dir.path(), &FakeMsStore::new(Ok(trial(6)))),
    ] {
        assert_eq!(status(&core)["canEdit"], true);
        call(
            &core,
            "project.create",
            json!({ "title": "Still Works", "projectType": "Short Film" }),
        );
        stop(core);
    }
    assert!(
        app_data.join("entitlement").is_file(),
        "nothing overwritten"
    );
}

#[test]
fn plat_009_status_takes_no_arguments() {
    let dir = tempfile::tempdir().unwrap();
    let core = internal_core(dir.path());
    let e = core
        .dispatch(&core.local_actor(), "entitlement.status", json!({ "x": 1 }))
        .unwrap_err();
    assert_eq!(e.code.0, "validation.invalid_input");
}
