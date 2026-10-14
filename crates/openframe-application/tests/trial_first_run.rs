//! Trial reminders, the buy button and the once-only screens after a purchase (owner mocks 182 to
//! 198; `docs/engineering/36-entitlement-and-trial.md` section 10; feature rows PLAT-009,
//! PLAT-012, PLAT-014 and PLAT-015).
//!
//! The stores own the trial (ADR-0023): the Microsoft Store answers through a fake licence source
//! here, the Mac through a fake StoreKit bridge. The cores below share one app data folder, the
//! way launches days apart would. The local calendar day comes from the interface as `today`.

use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use openframe_application::events::RecordingSink;
use openframe_application::modules::ai::toolbox;
use openframe_application::registry::{self, AiExposure};
use openframe_application::{AppConfig, AppCore};
use openframe_domain::Actor;
use openframe_domain::auth::ActorOrigin;
use openframe_entitlement::Channel;
use openframe_entitlement::adapters::{
    StoreAppLicence, StoreLicenceError, StoreLicenceSource, StorePurchaseStatus,
};
use openframe_entitlement::mac::{
    AppTransactionInfo, BridgeError, InstantClock, LIFETIME_PRODUCT_ID, MacInputs, NoNonces,
    ProductInfo, PurchaseOutcome, RevalidateReason, StoreEnvironment, StoreKitBridge,
    UnavailableServer, VerifiedTransaction,
};
use serde_json::{Value, json};

const DAY_SECONDS: u64 = 24 * 60 * 60;
/// 2027-01-15, a fixed purchase date for the Mac fakes.
const T0: i64 = 1_800_000_000_000;

/// A Microsoft Store whose licence answer the test sets (`GetAppLicenseAsync`), and whose
/// purchase dialog (`RequestPurchaseAsync`) turns the licence into a purchase when told to.
#[derive(Clone)]
struct MsStore {
    licence: Arc<Mutex<Result<StoreAppLicence, StoreLicenceError>>>,
    buys: Arc<Mutex<bool>>,
}

impl MsStore {
    fn trial(days: u64) -> Self {
        Self {
            licence: Arc::new(Mutex::new(Ok(trial_licence(days)))),
            buys: Arc::new(Mutex::new(false)),
        }
    }
    fn set_trial(&self, days: u64) {
        *self.licence.lock().unwrap() = Ok(trial_licence(days));
    }
    fn set_bought(&self) {
        *self.licence.lock().unwrap() = Ok(StoreAppLicence {
            is_active: true,
            is_trial: false,
            trial_seconds_left: 0,
        });
    }
    fn set_ended(&self) {
        *self.licence.lock().unwrap() = Ok(StoreAppLicence {
            is_active: false,
            is_trial: true,
            trial_seconds_left: 0,
        });
    }
}

const fn trial_licence(days: u64) -> StoreAppLicence {
    StoreAppLicence {
        is_active: true,
        is_trial: true,
        trial_seconds_left: days * DAY_SECONDS,
    }
}

impl StoreLicenceSource for MsStore {
    fn app_licence(&self) -> Result<StoreAppLicence, StoreLicenceError> {
        *self.licence.lock().unwrap()
    }
    fn request_purchase(&self) -> StorePurchaseStatus {
        if *self.buys.lock().unwrap() {
            self.set_bought();
            StorePurchaseStatus::Succeeded
        } else {
            StorePurchaseStatus::NotPurchased
        }
    }
}

fn ms_config(root: &Path, store: &MsStore) -> AppConfig {
    AppConfig {
        channel: Channel::MicrosoftStore,
        microsoft_store: Some(Box::new(store.clone())),
        ..AppConfig::isolated(root)
    }
}

fn start(config: AppConfig) -> Arc<AppCore> {
    AppCore::new(config, Arc::new(RecordingSink::default())).expect("startup never fails")
}

fn ms_core(root: &Path, store: &MsStore) -> Arc<AppCore> {
    start(ms_config(root, store))
}

fn call(core: &AppCore, op: &str, args: Value) -> Value {
    core.dispatch(&core.local_actor(), op, args)
        .unwrap_or_else(|e| panic!("{op} failed: {} ({:?})", e.code_str(), e.detail))
}

fn fail(core: &AppCore, op: &str, args: Value) -> String {
    core.dispatch(&core.local_actor(), op, args)
        .expect_err(op)
        .code
        .0
}

fn notice(core: &AppCore, today: &str) -> Value {
    call(core, "entitlement.notice", json!({ "today": today }))["notice"].clone()
}

fn record(core: &AppCore, kind: &str, today: Option<&str>) -> Value {
    call(
        core,
        "entitlement.record_notice",
        json!({ "notice": kind, "today": today }),
    )
}

fn stop(core: Arc<AppCore>) {
    core.shutdown();
    drop(core);
}

fn notices_file(root: &Path) -> std::path::PathBuf {
    root.join("appdata")
        .join("entitlement")
        .join("notices.json")
}

#[test]
fn plat_009_reminders_on_trial_days_five_six_and_seven_once_per_calendar_day() {
    let dir = tempfile::tempdir().unwrap();
    let store = MsStore::trial(7);

    // Days 1 to 4 (7 to 4 days left on the Store's clock): no reminder.
    for days_left in [7, 6, 5, 4] {
        store.set_trial(days_left);
        let core = ms_core(dir.path(), &store);
        assert_eq!(
            notice(&core, "2027-01-15"),
            json!({ "kind": "none" }),
            "{days_left} days left"
        );
        stop(core);
    }
    // Day 5: "ends in 3 days", shown once that calendar day, also after a restart.
    store.set_trial(3);
    let core = ms_core(dir.path(), &store);
    assert_eq!(
        notice(&core, "2027-01-19"),
        json!({ "kind": "reminder", "daysLeft": 3 })
    );
    let after = record(&core, "reminder", Some("2027-01-19"));
    assert_eq!(after["notice"], json!({ "kind": "none" }));
    stop(core);
    let core = ms_core(dir.path(), &store);
    assert_eq!(
        notice(&core, "2027-01-19"),
        json!({ "kind": "none" }),
        "same day after a restart"
    );
    stop(core);
    // Day 6 and day 7.
    store.set_trial(2);
    let core = ms_core(dir.path(), &store);
    assert_eq!(
        notice(&core, "2027-01-20"),
        json!({ "kind": "reminder", "daysLeft": 2 })
    );
    record(&core, "reminder", Some("2027-01-20"));
    stop(core);
    store.set_trial(1);
    let core = ms_core(dir.path(), &store);
    assert_eq!(
        notice(&core, "2027-01-21"),
        json!({ "kind": "reminder", "daysLeft": 1 })
    );
    record(&core, "reminder", Some("2027-01-21"));
    // The computer's clock moved back a day: the reminder is not shown twice.
    assert_eq!(notice(&core, "2027-01-20"), json!({ "kind": "none" }));
    stop(core);
    // After the trial: the lock, never a reminder.
    store.set_ended();
    let core = ms_core(dir.path(), &store);
    assert_eq!(
        call(&core, "entitlement.status", json!({}))["state"],
        json!({ "state": "trialEnded" })
    );
    assert_eq!(notice(&core, "2027-01-23"), json!({ "kind": "locked" }));
    assert_eq!(
        fail(
            &core,
            "entitlement.record_notice",
            json!({ "notice": "reminder", "today": "2027-01-23" })
        ),
        "conflict.state"
    );
}

#[test]
fn plat_009_an_invalid_or_missing_day_shows_no_reminder_and_cannot_be_recorded() {
    let dir = tempfile::tempdir().unwrap();
    let core = ms_core(dir.path(), &MsStore::trial(1));
    for bad in [
        "",
        "21-01-2027",
        "2027-02-30",
        "../../etc",
        "2027-01-21\u{0}",
    ] {
        assert_eq!(notice(&core, bad), json!({ "kind": "none" }), "{bad:?}");
        assert_eq!(
            fail(
                &core,
                "entitlement.record_notice",
                json!({ "notice": "reminder", "today": bad })
            ),
            "validation.invalid_input"
        );
    }
    let none = call(&core, "entitlement.notice", json!({}));
    assert_eq!(none["notice"], json!({ "kind": "none" }));
    assert_eq!(
        fail(
            &core,
            "entitlement.notice",
            json!({ "today": "2027-01-21", "x": 1 })
        ),
        "validation.invalid_input",
        "unknown fields are refused"
    );
    assert_eq!(
        fail(
            &core,
            "entitlement.record_notice",
            json!({ "notice": "everything" })
        ),
        "validation.invalid_input"
    );
}

#[test]
fn plat_009_a_purchase_mid_trial_shows_the_note_tutorial_and_welcome_once_in_order() {
    let dir = tempfile::tempdir().unwrap();
    let store = MsStore::trial(2);
    let core = ms_core(dir.path(), &store);
    assert_eq!(notice(&core, "2027-01-20")["kind"], "reminder");

    // "Buy full version": the Store's purchase dialog is closed first, then used to buy.
    let r = call(&core, "entitlement.buy", json!({}));
    assert_eq!(r["route"], json!({ "route": "store" }));
    assert_eq!(r["message"], Value::Null);
    assert_eq!(notice(&core, "2027-01-20")["kind"], "reminder");
    *store.buys.lock().unwrap() = true;
    let r = call(&core, "entitlement.buy", json!({}));
    assert_eq!(r["status"]["state"], json!({ "state": "purchased" }));
    assert_eq!(
        notice(&core, "2027-01-20"),
        json!({ "kind": "dataNote" }),
        "no reminder after purchase"
    );

    // Out of order is refused: the screens cannot be skipped.
    assert_eq!(
        fail(
            &core,
            "entitlement.record_notice",
            json!({ "notice": "tutorial" })
        ),
        "conflict.state"
    );
    assert_eq!(
        fail(
            &core,
            "entitlement.record_notice",
            json!({ "notice": "welcome" })
        ),
        "conflict.state"
    );

    assert_eq!(
        record(&core, "dataNote", None)["notice"],
        json!({ "kind": "tutorial" })
    );
    stop(core);
    // A restart in the middle resumes at the tutorial.
    let core = ms_core(dir.path(), &store);
    assert_eq!(notice(&core, "2027-01-20"), json!({ "kind": "tutorial" }));
    assert_eq!(
        fail(
            &core,
            "entitlement.record_notice",
            json!({ "notice": "welcome" })
        ),
        "conflict.state"
    );
    assert_eq!(
        record(&core, "tutorial", None)["notice"],
        json!({ "kind": "welcome" })
    );
    assert_eq!(
        record(&core, "welcome", None)["notice"],
        json!({ "kind": "none" })
    );
    stop(core);
    let core = ms_core(dir.path(), &store);
    assert_eq!(
        notice(&core, "2027-02-14"),
        json!({ "kind": "none" }),
        "shown once only"
    );
    let text = std::fs::read_to_string(notices_file(dir.path())).unwrap();
    for flag in ["dataNoteAcknowledged", "tutorialCompleted", "welcomeShown"] {
        assert!(text.contains(&format!("\"{flag}\": true")), "{text}");
    }
}

#[test]
fn plat_009_the_notice_names_the_one_folder_to_back_up() {
    let dir = tempfile::tempdir().unwrap();
    let store = MsStore::trial(7);
    let core = ms_core(dir.path(), &store);
    // Projects and the Global Idea Vault share a parent: that parent is the folder to back up.
    let v = call(
        &core,
        "entitlement.notice",
        json!({ "today": "2027-01-15" }),
    );
    assert_eq!(v["dataRoot"], dir.path().to_string_lossy().as_ref());
    stop(core);
    // Kept apart: the projects folder itself.
    let apart = dir.path().join("elsewhere").join("Vault");
    let core = start(AppConfig {
        global_vault_dir: apart,
        ..ms_config(dir.path(), &store)
    });
    let v = call(&core, "entitlement.notice", json!({}));
    assert_eq!(
        v["dataRoot"],
        dir.path().join("projects").to_string_lossy().as_ref()
    );
}

#[test]
fn plat_009_trial_users_and_the_internal_build_never_see_the_first_run_screens() {
    let dir = tempfile::tempdir().unwrap();
    let core = ms_core(dir.path(), &MsStore::trial(6));
    assert_eq!(notice(&core, "2027-01-15"), json!({ "kind": "none" }));
    assert_eq!(
        fail(
            &core,
            "entitlement.record_notice",
            json!({ "notice": "dataNote" })
        ),
        "conflict.state"
    );
    assert!(
        !notices_file(dir.path()).exists(),
        "nothing written for a refused screen"
    );
    stop(core);
    // The internal installer build: no trial, no reminder, no purchase screens (ADR-0023).
    let core = start(AppConfig::isolated(dir.path()));
    assert_eq!(notice(&core, "2027-01-15"), json!({ "kind": "none" }));
    for kind in ["reminder", "dataNote"] {
        assert_eq!(
            fail(
                &core,
                "entitlement.record_notice",
                json!({ "notice": kind, "today": "2027-01-15" })
            ),
            "conflict.state",
            "{kind}"
        );
    }
}

#[test]
fn plat_009_a_broken_notice_record_never_fails_startup_and_only_shows_screens_again() {
    let dir = tempfile::tempdir().unwrap();
    let store = MsStore::trial(7);
    store.set_bought();
    let core = ms_core(dir.path(), &store);
    record(&core, "dataNote", None);
    stop(core);
    for broken in [&b"{ not json"[..], b"\x00\xff", &vec![b'x'; 10_000][..]] {
        std::fs::write(notices_file(dir.path()), broken).unwrap();
        let core = ms_core(dir.path(), &store);
        assert_eq!(
            call(&core, "entitlement.status", json!({}))["state"],
            json!({ "state": "purchased" }),
            "the licence answer does not depend on this file"
        );
        assert_eq!(notice(&core, "2027-01-16"), json!({ "kind": "dataNote" }));
        stop(core);
    }
}

#[test]
fn plat_009_a_folder_that_cannot_be_written_never_traps_the_user_on_a_mandatory_screen() {
    let dir = tempfile::tempdir().unwrap();
    let store = MsStore::trial(7);
    store.set_bought();
    let core = ms_core(dir.path(), &store);
    // The notice record path now goes through a folder that cannot hold the file.
    std::fs::create_dir_all(notices_file(dir.path())).unwrap();
    assert_eq!(notice(&core, "2027-01-15"), json!({ "kind": "dataNote" }));
    assert_eq!(
        record(&core, "dataNote", None)["notice"],
        json!({ "kind": "tutorial" })
    );
    assert_eq!(
        record(&core, "tutorial", None)["notice"],
        json!({ "kind": "welcome" })
    );
    assert_eq!(
        record(&core, "welcome", None)["notice"],
        json!({ "kind": "none" })
    );
}

#[test]
fn plat_009_the_microsoft_store_starts_the_trial_itself() {
    let dir = tempfile::tempdir().unwrap();
    let core = ms_core(dir.path(), &MsStore::trial(7));
    let r = call(&core, "entitlement.start_trial", json!({}));
    assert_eq!(r["route"], json!({ "route": "unavailable" }));
    assert_eq!(
        r["status"]["state"],
        json!({ "state": "trialActive", "days_left": 7 })
    );
}

#[test]
fn plat_009_only_the_person_at_this_computer_buys_or_confirms_a_screen() {
    let dir = tempfile::tempdir().unwrap();
    let store = MsStore::trial(7);
    store.set_bought();
    let core = ms_core(dir.path(), &store);
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
        for (op, args) in [
            ("entitlement.buy", json!({})),
            ("entitlement.start_trial", json!({})),
            ("entitlement.record_notice", json!({ "notice": "dataNote" })),
        ] {
            let e = core.dispatch(&who, op, args).unwrap_err();
            assert_eq!(e.code.0, "permission.denied", "{op}");
        }
    }
    assert_eq!(
        notice(&core, "2027-01-15"),
        json!({ "kind": "dataNote" }),
        "nothing was recorded"
    );
}

#[test]
fn plat_009_the_assistant_cannot_see_the_new_licence_operations() {
    let reg = registry::catalog();
    for op in [
        "entitlement.notice",
        "entitlement.record_notice",
        "entitlement.buy",
        "entitlement.start_trial",
    ] {
        let m = reg
            .metadata(op)
            .unwrap_or_else(|| panic!("{op} registered"));
        assert!(
            matches!(m.ai_exposure, AiExposure::Hidden(_)),
            "{op} hidden"
        );
        assert_eq!(m.required_capability, None, "{op} is application level");
        for (tool, _, ops) in toolbox::coverage() {
            assert!(!ops.contains(&op), "{tool} covers {op}");
        }
    }
    assert!(
        reg.metadata("entitlement.debug_set").is_none(),
        "the hand-set state is not part of the product catalog"
    );
}

#[test]
fn plat_009_the_hand_set_state_exists_only_when_the_shell_turns_it_on() {
    let dir = tempfile::tempdir().unwrap();
    let core = start(AppConfig::isolated(dir.path()));
    let args = json!({ "channel": "macAppStore", "state": { "state": "trialEnded" } });
    assert_eq!(
        fail(&core, "entitlement.debug_set", args.clone()),
        "security.unknown_operation"
    );
    stop(core);

    let core = start(AppConfig {
        entitlement_debug_controls: true,
        ..AppConfig::isolated(dir.path())
    });
    let s = call(&core, "entitlement.debug_set", args);
    assert_eq!(s["channel"], "macAppStore");
    assert_eq!(notice(&core, "2027-01-15"), json!({ "kind": "locked" }));
    // Restore cannot undo a hand-set state, and Buy says the store is not connected.
    assert_eq!(
        call(&core, "entitlement.restore_purchase", json!({}))["state"],
        json!({ "state": "trialEnded" })
    );
    let r = call(&core, "entitlement.buy", json!({}));
    assert_eq!(r["route"], json!({ "route": "unavailable" }));
    assert_eq!(
        r["message"],
        "The App Store connection is not available in this build."
    );
    call(
        &core,
        "entitlement.debug_set",
        json!({ "channel": "microsoftStore", "state": { "state": "purchased" }, "resetNotices": true }),
    );
    assert_eq!(notice(&core, "2027-01-15"), json!({ "kind": "dataNote" }));
    assert_eq!(
        call(&core, "entitlement.buy", json!({}))["message"],
        "The Microsoft Store connection is not available in this build."
    );
}

// ------------------------------------------------------------------ Mac App Store with a fake bridge

/// A fake StoreKit bridge: no purchase at first, then whatever the test queues. A real Mac and the
/// App Store are not used; this proves the core and the screens' rules against the adapter.
#[derive(Clone, Default)]
struct FakeStore {
    owned: Arc<Mutex<Vec<VerifiedTransaction>>>,
    next: Arc<Mutex<Option<Result<PurchaseOutcome, BridgeError>>>>,
    purchases: Arc<AtomicUsize>,
}

fn lifetime_tx() -> VerifiedTransaction {
    VerifiedTransaction {
        product_id: LIFETIME_PRODUCT_ID.to_string(),
        transaction_id: "2000000077".to_string(),
        original_transaction_id: "2000000077".to_string(),
        app_transaction_id: None,
        purchase_date_ms: T0,
        revocation_date_ms: None,
        environment: StoreEnvironment::Production,
        signed_jws: "aGVhZA.Ym9keQ.c2ln".to_string(),
    }
}

impl StoreKitBridge for FakeStore {
    fn current_entitlements(&self) -> Result<Vec<VerifiedTransaction>, BridgeError> {
        Ok(self.owned.lock().unwrap().clone())
    }
    fn purchase(&self, _product_id: &str) -> Result<PurchaseOutcome, BridgeError> {
        self.purchases.fetch_add(1, Ordering::SeqCst);
        let outcome = self
            .next
            .lock()
            .unwrap()
            .take()
            .unwrap_or(Ok(PurchaseOutcome::Cancelled));
        if let Ok(PurchaseOutcome::Success(tx)) = &outcome {
            self.owned.lock().unwrap().push(tx.clone());
        }
        outcome
    }
    fn app_transaction(&self) -> Result<AppTransactionInfo, BridgeError> {
        Err(BridgeError::StoreUnavailable)
    }
    fn environment(&self) -> StoreEnvironment {
        StoreEnvironment::Production
    }
    fn product_info(&self, _product_id: &str) -> Result<ProductInfo, BridgeError> {
        Err(BridgeError::ProductNotFound)
    }
}

fn mac_core(root: &Path, store: &FakeStore) -> Arc<AppCore> {
    start(AppConfig {
        channel: Channel::MacAppStore,
        mac_inputs: Some(MacInputs {
            bridge: Box::new(store.clone()),
            server: Box::new(UnavailableServer),
            nonces: Box::new(NoNonces),
            clock: Box::new(InstantClock::new()),
            response_key: None,
        }),
        ..AppConfig::isolated(root)
    })
}

#[test]
fn plat_014_mac_first_launch_then_apple_sheet_then_the_first_run_screens() {
    let dir = tempfile::tempdir().unwrap();
    let store = FakeStore::default();
    let core = mac_core(dir.path(), &store);
    // Before the background check answers: the purchase check screen.
    assert_eq!(
        notice(&core, "2027-01-15"),
        json!({ "kind": "checkNeeded" })
    );
    core.entitlement.revalidate(RevalidateReason::Launch);
    assert_eq!(
        notice(&core, "2027-01-15"),
        json!({ "kind": "macFirstLaunch" })
    );

    // "Buy full version": Apple's sheet is cancelled, fails offline, then succeeds.
    let r = call(&core, "entitlement.buy", json!({}));
    assert_eq!(r["route"], json!({ "route": "store" }));
    assert_eq!(r["message"], Value::Null);
    assert_eq!(
        notice(&core, "2027-01-15"),
        json!({ "kind": "macFirstLaunch" })
    );
    *store.next.lock().unwrap() = Some(Err(BridgeError::NoConnection));
    let r = call(&core, "entitlement.buy", json!({}));
    assert!(
        r["message"]
            .as_str()
            .unwrap()
            .contains("could not reach the App Store"),
        "{r}"
    );
    *store.next.lock().unwrap() = Some(Ok(PurchaseOutcome::Success(lifetime_tx())));
    let r = call(&core, "entitlement.buy", json!({}));
    assert_eq!(r["status"]["state"], json!({ "state": "purchased" }));
    assert_eq!(r["message"], Value::Null);
    assert_eq!(store.purchases.load(Ordering::SeqCst), 3);
    assert_eq!(notice(&core, "2027-01-15"), json!({ "kind": "dataNote" }));
    assert_eq!(
        record(&core, "dataNote", None)["notice"],
        json!({ "kind": "tutorial" })
    );
}

#[test]
fn plat_014_mac_without_the_store_bridge_shows_the_check_screen_and_buy_explains() {
    let dir = tempfile::tempdir().unwrap();
    let core = start(AppConfig {
        channel: Channel::MacAppStore,
        ..AppConfig::isolated(dir.path())
    });
    assert_eq!(
        notice(&core, "2027-01-15"),
        json!({ "kind": "checkNeeded" })
    );
    let r = call(&core, "entitlement.buy", json!({}));
    assert_eq!(r["status"]["state"], json!({ "state": "needsOnlineCheck" }));
    assert!(r["message"].as_str().is_some_and(|m| !m.is_empty()), "{r}");
    let r = call(&core, "entitlement.start_trial", json!({}));
    assert!(r["message"].as_str().is_some_and(|m| !m.is_empty()), "{r}");
}
