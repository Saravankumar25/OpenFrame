//! Test harness shared by all module test suites.
//!
//! ```ignore
//! let env = TestEnv::with_project("Railway", "Feature Film");
//! let act = env.ok("story.create_act", json!({ "title": "Act One" }));
//! ```

use std::sync::Arc;

use openframe_application::events::RecordingSink;
use openframe_application::{AppConfig, AppCore};
use openframe_domain::auth::ActorOrigin;
use openframe_domain::{Actor, AppError, Role};
use serde_json::{Value, json};

pub struct TestEnv {
    pub dir: tempfile::TempDir,
    pub core: Arc<AppCore>,
    pub sink: Arc<RecordingSink>,
}

impl TestEnv {
    /// An isolated OpenFrame environment (own app data, projects dir, global vault).
    pub fn new() -> TestEnv {
        let dir = tempfile::tempdir().expect("tempdir");
        let sink = Arc::new(RecordingSink::default());
        let core = AppCore::new(AppConfig::isolated(dir.path()), sink.clone()).expect("core");
        TestEnv { dir, core, sink }
    }

    /// Environment with a freshly created, open project.
    pub fn with_project(title: &str, project_type: &str) -> TestEnv {
        let env = TestEnv::new();
        env.ok(
            "project.create",
            json!({ "title": title, "projectType": project_type }),
        );
        env
    }

    pub fn actor(&self) -> Actor {
        self.core.local_actor()
    }

    /// An actor with a different role (e.g. to test permission denials).
    pub fn actor_with_role(&self, role: Role) -> Actor {
        let mut a = self.core.local_actor();
        a.role = role;
        a
    }

    /// A different user (collaborator whose reviewed exchange-package changes are applied).
    pub fn other_user(&self, role: Role) -> Actor {
        Actor {
            user_id: openframe_domain::new_id(),
            display_name: "Collaborator".into(),
            role,
            origin: ActorOrigin::Exchange {
                package_id: "test".into(),
            },
        }
    }

    pub fn call(&self, op: &str, args: Value) -> Result<Value, AppError> {
        self.core.dispatch(&self.actor(), op, args)
    }

    pub fn call_as(&self, actor: &Actor, op: &str, args: Value) -> Result<Value, AppError> {
        self.core.dispatch(actor, op, args)
    }

    /// Call and panic with the error code on failure.
    #[track_caller]
    pub fn ok(&self, op: &str, args: Value) -> Value {
        match self.call(op, args) {
            Ok(v) => v,
            Err(e) => panic!("{op} failed: {} — {} ({:?})", e.code, e.message, e.detail),
        }
    }

    /// Call expecting failure; returns the error code.
    #[track_caller]
    pub fn err(&self, op: &str, args: Value) -> String {
        match self.call(op, args) {
            Ok(v) => panic!("{op} unexpectedly succeeded: {v}"),
            Err(e) => e.code.0,
        }
    }

    pub fn undo(&self) -> Value {
        self.ok("history.undo", json!({}))
    }

    pub fn redo(&self) -> Value {
        self.ok("history.redo", json!({}))
    }

    /// Simulate quitting the app cleanly and starting it again on the same data.
    pub fn restart(self) -> TestEnv {
        self.core.shutdown();
        let TestEnv { dir, core, .. } = self;
        drop(core);
        let sink = Arc::new(RecordingSink::default());
        let core = AppCore::new(AppConfig::isolated(dir.path()), sink.clone()).expect("core");
        TestEnv { dir, core, sink }
    }

    /// Simulate a crash: the process disappears without closing the project.
    pub fn crash_and_restart(self) -> TestEnv {
        let TestEnv { dir, core, .. } = self;
        // Drop the session without the clean-close path (no checkpoint, marker stays).
        let session = core.set_project(None);
        drop(session);
        drop(core);
        let sink = Arc::new(RecordingSink::default());
        let core = AppCore::new(AppConfig::isolated(dir.path()), sink.clone()).expect("core");
        TestEnv { dir, core, sink }
    }

    /// Path of the currently open project folder.
    pub fn project_path(&self) -> String {
        self.core
            .project()
            .expect("project open")
            .layout
            .root()
            .to_string_lossy()
            .into_owned()
    }

    pub fn reopen_project(&self, path: &str) -> Value {
        self.ok("project.open", json!({ "path": path }))
    }

    /// Write a scratch file inside the environment and return its path.
    pub fn write_file(&self, name: &str, bytes: &[u8]) -> String {
        let p = self.dir.path().join("inputs").join(name);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, bytes).unwrap();
        p.to_string_lossy().into_owned()
    }
}

impl Default for TestEnv {
    fn default() -> Self {
        Self::new()
    }
}
