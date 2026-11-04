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

use serde_json::{Value, json};
use straft_application::events::{AppEvent, RecordingSink};
use straft_application::modules::ai::toolbox;
use straft_application::registry::{self, AiExposure, OpClass};
use straft_application::{AppConfig, AppCore};
use straft_domain::auth::ActorOrigin;
use straft_domain::{Actor, Role};
use straft_entitlement::Channel;
use straft_entitlement::adapters::{
    StoreAppLicence, StoreLicenceError, StoreLicenceSource, StorePurchaseStatus,
};

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
    let core = start(AppConfig {
        channel: Channel::MicrosoftStore,
        microsoft_store: Some(Box::new(store.clone())),
        ..AppConfig::isolated(root)
    });
    // Startup asks the Microsoft Store nothing: the shell's worker runs the first read.
    core.refresh_entitlement();
    core
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
    // A lock that appears in a running app first leaves a short window for the lock screen to
    // store what the editors hold; the screen ends it with `entitlement.lock_ready`.
    call(core, "entitlement.lock_ready", json!({}));
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
        "the Store owns the trial; Straft keeps no timer"
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

    // The trial ran out while Straft was closed.
    store.set(Ok(ENDED));
    let core = ms_core(dir.path(), &store);
    let s = status(&core);
    assert_eq!(s["state"], json!({ "state": "trialEnded" }));
    assert_eq!(s["canEdit"], false);
    assert_eq!(
        s["summary"],
        "Your free trial has ended. Buy the full version to keep using Straft Studio. Your projects are safe on this computer. Buying unlocks them again exactly as you left them."
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
    // The trial ended between launches: the first read found the lock with the page possibly
    // holding text, so the short window is open until the lock screen says it has saved.
    call(&core, "entitlement.lock_ready", json!({}));
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

    // Offline on a computer that remembers nothing (no memory file) and has no cached licence: a
    // check is needed. Editing locks, nothing else does.
    std::fs::remove_dir_all(dir.path().join("appdata").join("entitlement")).unwrap();
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

    // Editing is locked in Rust: a command that changes the project is refused with plain words,
    // closing the project, the app's settings and the assistant's reads still work.
    let e = core
        .dispatch(
            &core.local_actor(),
            "story.create_act",
            json!({ "title": "Act Two" }),
        )
        .unwrap_err();
    assert_eq!(e.code_str(), "permission.licence_required");
    assert!(
        e.message.contains("could not check your purchase"),
        "{}",
        e.message
    );
    assert!(
        e.message.contains("open your projects and export"),
        "{}",
        e.message
    );
    let ai = Actor {
        origin: ActorOrigin::Ai {
            request_id: "r1".into(),
        },
        ..core.local_actor()
    };
    assert_eq!(
        core.dispatch(&ai, "story.create_act", json!({ "title": "Act Two" }))
            .unwrap_err()
            .code_str(),
        "permission.licence_required"
    );
    call(&core, "appearance.get", json!({}));
    call(&core, "story.board", json!({}));
    call(&core, "project.close", json!({}));

    // A good answer from the Store lifts it.
    store.set(Ok(trial(4)));
    call(&core, "entitlement.restore_purchase", json!({}));
    call(&core, "project.open", json!({ "path": path }));
    call(&core, "story.create_act", json!({ "title": "Act Two" }));
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
    // The project is made on an internal build: editing is locked while a store cannot be checked.
    let core = internal_core(dir.path());
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
    entitlements: Arc<Mutex<Vec<straft_entitlement::mac::VerifiedTransaction>>>,
    on_purchase: Arc<Mutex<Option<straft_entitlement::mac::VerifiedTransaction>>>,
    asked: Arc<std::sync::atomic::AtomicUsize>,
}

fn lifetime_tx(id: &str, revoked: bool) -> straft_entitlement::mac::VerifiedTransaction {
    use straft_entitlement::mac::{LIFETIME_PRODUCT_ID, StoreEnvironment, VerifiedTransaction};
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
    fn new(entitlements: Vec<straft_entitlement::mac::VerifiedTransaction>) -> Self {
        Self {
            entitlements: Arc::new(Mutex::new(entitlements)),
            on_purchase: Arc::new(Mutex::new(None)),
            asked: Arc::new(std::sync::atomic::AtomicUsize::new(0)),
        }
    }
}

impl straft_entitlement::mac::StoreKitBridge for FakeBridge {
    fn current_entitlements(
        &self,
    ) -> Result<
        Vec<straft_entitlement::mac::VerifiedTransaction>,
        straft_entitlement::mac::BridgeError,
    > {
        self.asked.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(self.entitlements.lock().unwrap().clone())
    }
    fn purchase(
        &self,
        _product_id: &str,
    ) -> Result<straft_entitlement::mac::PurchaseOutcome, straft_entitlement::mac::BridgeError>
    {
        match self.on_purchase.lock().unwrap().take() {
            Some(tx) => {
                self.entitlements.lock().unwrap().push(tx.clone());
                Ok(straft_entitlement::mac::PurchaseOutcome::Success(tx))
            }
            None => Err(straft_entitlement::mac::BridgeError::Other),
        }
    }
    fn app_transaction(
        &self,
    ) -> Result<straft_entitlement::mac::AppTransactionInfo, straft_entitlement::mac::BridgeError>
    {
        Err(straft_entitlement::mac::BridgeError::StoreUnavailable)
    }
    fn environment(&self) -> straft_entitlement::mac::StoreEnvironment {
        straft_entitlement::mac::StoreEnvironment::Production
    }
    fn product_info(
        &self,
        _product_id: &str,
    ) -> Result<straft_entitlement::mac::ProductInfo, straft_entitlement::mac::BridgeError> {
        Err(straft_entitlement::mac::BridgeError::ProductNotFound)
    }
}

fn mac_core_with(root: &Path, bridge: &FakeBridge) -> Arc<AppCore> {
    use straft_entitlement::mac::{InstantClock, MacInputs, NoNonces, UnavailableServer};
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
    use straft_entitlement::mac::RevalidateReason;
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
    use std::sync::atomic::Ordering;
    use straft_entitlement::mac::RevalidateReason;

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
    assert_eq!(state, straft_entitlement::EntitlementState::Purchased);
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
    for op in straft_application::modules::entitlement::ALLOWED_WHILE_LOCKED {
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

// ------------------------------------------------------------------ Store licensing (ADR-0039)

/// A Microsoft Store core whose events the test can read. Runs the first read, as the shell's
/// worker does after startup.
fn ms_core_with_sink(root: &Path, store: &FakeMsStore) -> (Arc<AppCore>, Arc<RecordingSink>) {
    let sink = Arc::new(RecordingSink::default());
    let core = AppCore::new(
        AppConfig {
            channel: Channel::MicrosoftStore,
            microsoft_store: Some(Box::new(store.clone())),
            ..AppConfig::isolated(root)
        },
        sink.clone(),
    )
    .expect("startup never fails");
    core.refresh_entitlement();
    (core, sink)
}

fn licence_changes(sink: &RecordingSink) -> usize {
    sink.events
        .lock()
        .iter()
        .filter(|e| {
            matches!(e, AppEvent::Module { module, name, .. }
                if module == "entitlement" && name == "changed")
        })
        .count()
}

fn notice_kind(core: &AppCore, today: &str) -> Value {
    call(core, "entitlement.notice", json!({ "today": today }))["notice"].clone()
}

fn buy_with(core: &AppCore, store: &FakeMsStore) -> Value {
    *store.purchase.lock().unwrap() = StorePurchaseStatus::Succeeded;
    *store.after_purchase.lock().unwrap() = Some(BOUGHT);
    call(core, "entitlement.buy", json!({}))
}

fn now_ms() -> i64 {
    straft_domain::now_ms()
}

#[test]
fn store_matrix_the_chip_words_the_end_moment_and_the_reminder_follow_the_days_left() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(trial(7)));
    let core = ms_core(dir.path(), &store);
    for days in (1..=7u64).rev() {
        store.set(Ok(trial(days)));
        core.refresh_entitlement();
        let before = now_ms();
        let s = status(&core);
        let after = now_ms();
        assert_eq!(
            s["state"],
            json!({ "state": "trialActive", "days_left": days })
        );
        let words = if days == 1 {
            "Trial ends in 1 day.".to_string()
        } else {
            format!("Trial ends in {days} days.")
        };
        assert_eq!(s["summary"], words);
        let day_ms = i64::try_from(days * DAY_SECONDS * 1000).unwrap();
        let ends = s["trialEndsAtMs"]
            .as_i64()
            .expect("an end moment in a trial");
        assert!(
            (before - 5_000 + day_ms..=after + day_ms).contains(&ends),
            "{days} days: {ends}"
        );
        // Reminders only on 3, 2 and 1 days left.
        let expected = if days <= 3 {
            json!({ "kind": "reminder", "daysLeft": days })
        } else {
            json!({ "kind": "none" })
        };
        assert_eq!(notice_kind(&core, "2027-02-03"), expected, "{days} days");
    }
}

#[test]
fn store_matrix_a_purchase_on_any_day_unlocks_at_once_and_the_first_run_screens_follow_in_order() {
    for days in [7, 4, 1] {
        let dir = tempfile::tempdir().unwrap();
        let store = FakeMsStore::new(Ok(trial(days)));
        let (core, sink) = ms_core_with_sink(dir.path(), &store);
        let seen = licence_changes(&sink);
        let r = buy_with(&core, &store);
        assert_eq!(
            r["status"]["state"],
            json!({ "state": "purchased" }),
            "day {days}"
        );
        assert_eq!(r["status"]["trialEndsAtMs"], Value::Null);
        assert_eq!(
            r["status"]["summary"],
            "Straft Studio is unlocked. Thank you."
        );
        assert_eq!(licence_changes(&sink), seen + 1, "the page is told once");
        // No chip, no reminder; the three screens once each, in order.
        assert_eq!(
            notice_kind(&core, "2027-02-03"),
            json!({ "kind": "dataNote" })
        );
        for (kind, next) in [
            ("dataNote", "tutorial"),
            ("tutorial", "welcome"),
            ("welcome", "none"),
        ] {
            let v = call(
                &core,
                "entitlement.record_notice",
                json!({ "notice": kind, "today": "2027-02-03" }),
            );
            assert_eq!(
                v["notice"],
                json!({ "kind": next }),
                "day {days} after {kind}"
            );
        }
        stop(core);
        // A restart never shows them again.
        let core = ms_core(dir.path(), &store);
        assert_eq!(notice_kind(&core, "2027-02-04"), json!({ "kind": "none" }));
    }
}

#[test]
fn store_matrix_a_refund_and_an_ended_trial_lock_with_their_own_words() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(BOUGHT));
    let (core, sink) = ms_core_with_sink(dir.path(), &store);
    let seen = licence_changes(&sink);

    // Refund while open.
    store.set(Ok(ENDED));
    assert_eq!(
        core.refresh_entitlement(),
        straft_entitlement::EntitlementState::Revoked
    );
    let s = status(&core);
    assert_eq!(s["state"], json!({ "state": "revoked" }));
    assert_eq!(
        s["summary"],
        "Your purchase is no longer active. The store no longer shows a purchase for this account, so Straft Studio is locked. Your projects are still on this computer. If this is a mistake, choose Check again or contact us."
    );
    assert_eq!(
        notice_kind(&core, "2027-02-03"),
        json!({ "kind": "locked" })
    );
    assert_eq!(licence_changes(&sink), seen + 1);
    // Bought again from the lock screen: Purchased comes back from the memory.
    let r = buy_with(&core, &store);
    assert_eq!(r["status"]["state"], json!({ "state": "purchased" }));

    // A trial that ends while the app is open, on a computer that never bought.
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(trial(1)));
    let (core, _) = ms_core_with_sink(dir.path(), &store);
    store.set(Ok(ENDED));
    core.refresh_entitlement();
    let s = status(&core);
    assert_eq!(s["state"], json!({ "state": "trialEnded" }));
    assert_eq!(s["trialEndsAtMs"], Value::Null);
    assert_eq!(
        s["summary"],
        "Your free trial has ended. Buy the full version to keep using Straft Studio. Your projects are safe on this computer. Buying unlocks them again exactly as you left them."
    );
    assert_eq!(
        notice_kind(&core, "2027-02-03"),
        json!({ "kind": "locked" })
    );
}

#[test]
fn store_matrix_a_store_that_cannot_be_reached_keeps_what_the_app_last_knew() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(BOUGHT));
    let core = ms_core(dir.path(), &store);
    stop(core);
    // A buyer is never locked by a network error, also after a restart.
    store.set(Err(StoreLicenceError::Unreachable));
    let core = ms_core(dir.path(), &store);
    assert_eq!(status(&core)["state"], json!({ "state": "purchased" }));
    assert_eq!(
        notice_kind(&core, "2027-02-03"),
        json!({ "kind": "dataNote" })
    );
    stop(core);

    // A trial keeps counting from the last good answer, never more, also after a restart.
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(trial(3)));
    let core = ms_core(dir.path(), &store);
    let ends = status(&core)["trialEndsAtMs"].as_i64().unwrap();
    store.set(Err(StoreLicenceError::Unreachable));
    core.refresh_entitlement();
    let s = status(&core);
    assert_eq!(
        s["state"],
        json!({ "state": "trialActive", "days_left": 3 })
    );
    assert!(
        s["trialEndsAtMs"].as_i64().unwrap() <= ends + 1_000,
        "never later than before"
    );
    stop(core);
    let core = ms_core(dir.path(), &store);
    assert_eq!(
        status(&core)["state"],
        json!({ "state": "trialActive", "days_left": 3 })
    );
    stop(core);

    // The memory file says the trial ran out three days ago: nothing to count down.
    std::fs::write(
        dir.path()
            .join("appdata")
            .join("entitlement")
            .join("notices.json"),
        format!(
            r#"{{"lastState":"trial","lastTrialSeconds":{},"lastTrialReadAtMs":{}}}"#,
            3 * DAY_SECONDS,
            now_ms() - 4 * 86_400_000
        ),
    )
    .unwrap();
    let core = ms_core(dir.path(), &store);
    assert_eq!(
        status(&core)["state"],
        json!({ "state": "needsOnlineCheck" })
    );
    assert_eq!(
        notice_kind(&core, "2027-02-03"),
        json!({ "kind": "checkNeeded" })
    );

    // Nothing remembered and no answer: a check is needed, and opening still works.
    let dir = tempfile::tempdir().unwrap();
    let core = ms_core(
        dir.path(),
        &FakeMsStore::new(Err(StoreLicenceError::Unreachable)),
    );
    assert_eq!(
        status(&core)["state"],
        json!({ "state": "needsOnlineCheck" })
    );
    call(
        &core,
        "project.create",
        json!({ "title": "Still Opens", "projectType": "Short Film" }),
    );
}

#[test]
fn store_matrix_the_internal_build_shows_no_chip_no_end_moment_and_no_screen() {
    let dir = tempfile::tempdir().unwrap();
    let core = internal_core(dir.path());
    let s = status(&core);
    assert_eq!(s["trialEndsAtMs"], Value::Null);
    assert_eq!(notice_kind(&core, "2027-02-03"), json!({ "kind": "none" }));
    assert_eq!(
        call(&core, "entitlement.notice", json!({}))["debugBuild"],
        false
    );
}

#[test]
fn a_lock_that_appears_in_a_running_app_keeps_the_typed_text() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(BOUGHT));
    let (core, _sink) = ms_core_with_sink(dir.path(), &store);
    let path = make_project(&core, "Monsoon Ferry", "Ravi waits for the last ferry");
    let typed = "typed while the lock was opening";
    const LATE: &str = "zq-edit-after-the-lock-was-ready";

    // The Store takes the purchase back while a writer is typing.
    store.set(Ok(ENDED));
    core.refresh_entitlement();
    assert!(core.entitlement.locks_the_app());
    assert!(core.entitlement.flush_window_open());
    assert_eq!(
        notice_kind(&core, "2027-02-03"),
        json!({ "kind": "locked" })
    );

    // The lock screen stores what the editors hold: the save still goes through.
    let act = call(&core, "story.create_act", json!({ "title": "Act Two" }));
    call(
        &core,
        "story.create_card",
        json!({ "parent": { "parentType": "act", "parentId": act["id"] }, "shortDescription": typed }),
    );
    // Then the screen says it is ready: the window ends and the lock holds.
    call(&core, "entitlement.lock_ready", json!({}));
    assert!(!core.entitlement.flush_window_open());
    assert_eq!(
        refused(&core, "story.board", json!({})),
        "permission.licence_required"
    );
    assert_eq!(
        refused(
            &core,
            "story.create_card",
            json!({ "parent": { "parentType": "act", "parentId": act["id"] }, "shortDescription": LATE })
        ),
        "permission.licence_required"
    );
    // Said twice, it changes nothing.
    call(&core, "entitlement.lock_ready", json!({}));

    // The text is in the project's main database file, not only in the write-ahead log: a copy
    // of that one file, without its side files, has it.
    let main = core.project().unwrap().layout.database();
    let alone = dir.path().join("main-file-only.sqlite");
    std::fs::copy(&main, &alone).unwrap();
    let conn =
        rusqlite::Connection::open_with_flags(&alone, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    assert!(
        database_holds(&conn, typed),
        "the typed text is in the main file after the lock"
    );
    assert!(
        !database_holds(&conn, LATE),
        "nothing was added after the lock"
    );
    drop(conn);

    // Bought again: everything is as it was left, the text included.
    *store.purchase.lock().unwrap() = StorePurchaseStatus::Succeeded;
    *store.after_purchase.lock().unwrap() = Some(BOUGHT);
    call(&core, "entitlement.buy", json!({}));
    call(&core, "project.open", json!({ "path": path }));
    assert!(
        call(&core, "story.board", json!({}))
            .to_string()
            .contains(typed)
    );
}

/// True when any text column of any table in `conn` contains `needle`.
fn database_holds(conn: &rusqlite::Connection, needle: &str) -> bool {
    let tables: Vec<String> = conn
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let like = format!("%{needle}%");
    tables.iter().any(|t| {
        let columns: Vec<String> = conn
            .prepare(&format!("PRAGMA table_info(\"{t}\")"))
            .unwrap()
            .query_map([], |r| r.get(1))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        columns.iter().any(|c| {
            conn.query_row(
                &format!("SELECT 1 FROM \"{t}\" WHERE \"{c}\" LIKE ?1 LIMIT 1"),
                [&like],
                |_| Ok(()),
            )
            .is_ok()
        })
    })
}

#[test]
fn the_flush_window_ends_by_itself_and_a_lock_found_at_launch_has_none() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(trial(1)));
    let (core, _) = ms_core_with_sink(dir.path(), &store);
    core.entitlement
        .set_flush_window_for_tests(std::time::Duration::from_millis(150));
    store.set(Ok(ENDED));
    core.refresh_entitlement();
    // Inside the window an operation passes, although the page never answered.
    call(&core, "project.list_recent", json!({}));
    std::thread::sleep(std::time::Duration::from_millis(300));
    assert!(!core.entitlement.flush_window_open());
    assert_eq!(
        refused(&core, "project.list_recent", json!({})),
        "permission.licence_required"
    );
    stop(core);

    // The trial was over before the app started: nothing is unsaved, so no window.
    let core = ms_core(dir.path(), &store);
    assert!(core.entitlement.locks_the_app());
    assert!(!core.entitlement.flush_window_open());
    assert_eq!(
        refused(&core, "project.list_recent", json!({})),
        "permission.licence_required"
    );
    // A purchase closes the lock; a second lock opens a fresh window.
    buy_with(&core, &store);
    assert!(!core.entitlement.locks_the_app());
    store.set(Ok(ENDED));
    core.refresh_entitlement();
    assert!(core.entitlement.flush_window_open());
}

#[test]
fn a_hand_set_lock_goes_through_the_same_window_and_only_the_person_can_end_it() {
    let dir = tempfile::tempdir().unwrap();
    let core = start(AppConfig {
        entitlement_debug_controls: true,
        debug_build: true,
        ..AppConfig::isolated(dir.path())
    });
    let s = call(
        &core,
        "entitlement.debug_set",
        json!({ "channel": "microsoftStore", "state": { "state": "trialEnded" }, "trialEndsAtMs": null }),
    );
    assert_eq!(s["state"], json!({ "state": "trialEnded" }));
    assert!(
        core.entitlement.flush_window_open(),
        "also for the first call"
    );
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
            .dispatch(&who, "entitlement.lock_ready", json!({}))
            .unwrap_err();
        assert_eq!(e.code.0, "permission.denied");
    }
    assert!(
        core.entitlement.flush_window_open(),
        "refused callers end nothing"
    );
    call(&core, "entitlement.lock_ready", json!({}));
    assert!(!core.entitlement.flush_window_open());
    // Nothing is open and no store is behind a hand-set state: ending the window is harmless.
    call(&core, "entitlement.lock_ready", json!({}));

    // The forced end moment and the debug flag reach the page.
    let end = now_ms() + 2 * 86_400_000;
    let s = call(
        &core,
        "entitlement.debug_set",
        json!({ "channel": "microsoftStore", "state": { "state": "trialActive", "days_left": 2 }, "trialEndsAtMs": end }),
    );
    assert_eq!(s["trialEndsAtMs"], end);
    assert_eq!(
        call(&core, "entitlement.notice", json!({}))["debugBuild"],
        true
    );
    // The assistant never sees the new operation.
    let m = registry::catalog()
        .metadata("entitlement.lock_ready")
        .unwrap();
    assert!(matches!(m.ai_exposure, AiExposure::Hidden(r) if r.contains("assistant")));
    assert_eq!(
        call(
            &core,
            "entitlement.debug_set",
            json!({ "channel": "microsoftStore", "state": { "state": "clockChanged" } })
        )["state"],
        json!({ "state": "clockChanged" })
    );
    assert_eq!(
        notice_kind(&core, "2027-02-03"),
        json!({ "kind": "clockChanged" })
    );
}

#[test]
fn the_page_is_told_once_when_the_licence_changes_and_not_when_it_does_not() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(trial(5)));
    let (core, sink) = ms_core_with_sink(dir.path(), &store);
    assert_eq!(licence_changes(&sink), 1, "Unknown to a running trial");

    core.refresh_entitlement();
    core.refresh_entitlement();
    assert_eq!(licence_changes(&sink), 1, "the same answer says nothing");

    store.set(Ok(trial(4)));
    core.refresh_entitlement();
    assert_eq!(licence_changes(&sink), 2, "a day passed");
    core.refresh_entitlement();
    assert_eq!(licence_changes(&sink), 2);

    // Restore with no change says nothing; a buy that changes the state says so once.
    call(&core, "entitlement.restore_purchase", json!({}));
    assert_eq!(licence_changes(&sink), 2);
    buy_with(&core, &store);
    assert_eq!(licence_changes(&sink), 3);
    call(&core, "entitlement.restore_purchase", json!({}));
    assert_eq!(licence_changes(&sink), 3);

    // The Store goes quiet and a refund shows up: one more.
    store.set(Ok(ENDED));
    core.refresh_entitlement();
    assert_eq!(licence_changes(&sink), 4);
}

#[test]
fn an_update_keeps_the_flags_and_the_reminder_day_of_a_file_from_the_earlier_version() {
    let dir = tempfile::tempdir().unwrap();
    let folder = dir.path().join("appdata").join("entitlement");
    std::fs::create_dir_all(&folder).unwrap();
    let file = folder.join("notices.json");
    // Written by the version before the Store memory existed.
    std::fs::write(
        &file,
        r#"{
  "reminderShownOn": "2027-02-02",
  "dataNoteAcknowledged": true,
  "tutorialCompleted": true,
  "welcomeShown": true
}"#,
    )
    .unwrap();
    let store = FakeMsStore::new(Ok(trial(2)));
    let core = ms_core(dir.path(), &store);
    assert_eq!(
        notice_kind(&core, "2027-02-02"),
        json!({ "kind": "none" }),
        "shown today already"
    );
    assert_eq!(
        notice_kind(&core, "2027-02-03"),
        json!({ "kind": "reminder", "daysLeft": 2 })
    );
    // The first read saved the memory next to the flags and changed none of them.
    let text = std::fs::read_to_string(&file).unwrap();
    for part in [
        "\"reminderShownOn\": \"2027-02-02\"",
        "\"dataNoteAcknowledged\": true",
        "\"tutorialCompleted\": true",
        "\"welcomeShown\": true",
        "\"version\": 1",
        "\"lastState\": \"trial\"",
    ] {
        assert!(text.contains(part), "{part} in {text}");
    }
    // Bought after the update: the flags hold, so no screen repeats.
    buy_with(&core, &store);
    assert_eq!(notice_kind(&core, "2027-02-03"), json!({ "kind": "none" }));
    assert!(
        std::fs::read_to_string(&file)
            .unwrap()
            .contains("\"lastState\": \"purchased\"")
    );
    stop(core);

    // A damaged file starts empty, is kept aside, and never stops the app.
    std::fs::write(&file, b"{\"welcomeShown\": tr").unwrap();
    let core = ms_core(dir.path(), &store);
    assert_eq!(status(&core)["state"], json!({ "state": "purchased" }));
    assert_eq!(
        std::fs::read(folder.join("notices.json.bad")).unwrap(),
        b"{\"welcomeShown\": tr"
    );
}

/// ADR-0039 item 5: a lock saved by an earlier run holds before the Store has answered, and a
/// lock the first read finds with nothing saved leaves the page the short window to store text.
#[test]
fn a_saved_lock_holds_before_the_first_read_and_a_first_read_lock_opens_the_window() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeMsStore::new(Ok(trial(1)));
    let core = ms_core(dir.path(), &store);
    store.set(Ok(ENDED));
    core.refresh_entitlement();
    assert!(core.entitlement.locks_the_app());
    stop(core);

    // The next launch, with the first read not run yet (the Store may hang for 40 seconds).
    let core = start(AppConfig {
        channel: Channel::MicrosoftStore,
        microsoft_store: Some(Box::new(store.clone())),
        ..AppConfig::isolated(dir.path())
    });
    assert!(
        core.entitlement.locks_the_app(),
        "locked from the first moment"
    );
    assert_eq!(
        refused(&core, "project.list_recent", json!({})),
        "permission.licence_required"
    );
    core.refresh_entitlement();
    assert!(
        !core.entitlement.flush_window_open(),
        "known lock, nothing unsaved"
    );
    stop(core);

    // The trial ended between launches and nothing was saved: the first read finds the lock
    // while the page may already hold typing.
    let fresh = tempfile::tempdir().unwrap();
    let core = ms_core(fresh.path(), &store);
    assert!(core.entitlement.locks_the_app());
    assert!(core.entitlement.flush_window_open());
}

/// A Store whose purchase window stays open until the test lets it go.
#[derive(Clone)]
struct OpenWindowStore {
    inner: FakeMsStore,
    entered: Arc<Mutex<std::sync::mpsc::Sender<()>>>,
    release: Arc<Mutex<std::sync::mpsc::Receiver<()>>>,
}

impl StoreLicenceSource for OpenWindowStore {
    fn app_licence(&self) -> Result<StoreAppLicence, StoreLicenceError> {
        self.inner.app_licence()
    }
    fn request_purchase(&self) -> StorePurchaseStatus {
        self.entered.lock().unwrap().send(()).unwrap();
        let _ = self
            .release
            .lock()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(30));
        self.inner.request_purchase()
    }
}

/// ADR-0039 item 6: reads keep running while the Store's window is open, and a second "Buy"
/// never opens a second window.
#[test]
fn reads_run_while_the_purchase_window_is_open_and_a_second_buy_waits_for_it() {
    let dir = tempfile::tempdir().unwrap();
    let inner = FakeMsStore::new(Ok(trial(1)));
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let store = OpenWindowStore {
        inner: inner.clone(),
        entered: Arc::new(Mutex::new(entered_tx)),
        release: Arc::new(Mutex::new(release_rx)),
    };
    let core = start(AppConfig {
        channel: Channel::MicrosoftStore,
        microsoft_store: Some(Box::new(store)),
        ..AppConfig::isolated(dir.path())
    });
    core.refresh_entitlement();
    let buyer = {
        let core = core.clone();
        std::thread::spawn(move || core.dispatch(&core.local_actor(), "entitlement.buy", json!({})))
    };
    entered_rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("the Store window opened");

    // A second Buy answers plainly and opens no second window.
    let second = call(&core, "entitlement.buy", json!({}));
    assert!(
        second["message"]
            .as_str()
            .is_some_and(|m| m.contains("already open")),
        "{second}"
    );
    assert!(entered_rx.try_recv().is_err(), "no second window");

    // The trial ends while the dialog is open: the read runs and the lock shows.
    inner.set(Ok(ENDED));
    core.refresh_entitlement();
    assert!(core.entitlement.locks_the_app());

    release_tx.send(()).unwrap();
    buyer.join().unwrap().unwrap();
    assert!(
        core.entitlement.locks_the_app(),
        "closing the window bought nothing"
    );
}
