//! Make the assistant ready when a project opens, so the first question does not pay for it
//! (spec `ai-conversational-harness`, slice S8). In the background, in this order, each step only
//! if the project is still the open one:
//!
//! 1. the project index the answer-type read and the dossier use (`index::project_index`);
//! 2. the vectors of the labelled example messages (`turn::warm::examples`), kept per search model;
//! 3. the chat engine, only while [`START_ENGINE`] is on: started if memory allows, and the fixed
//!    start of each prompt read once (`turn::warm::prompts`) so the engine keeps its work on it
//!    (`AiManager::prime`).
//!
//! It runs only where the indexer already runs: AI is available and in use this session. It never
//! blocks the screen, stops at once when the project closes or changes or the computer goes to
//! sleep (it runs again after the wake, `intelligence::set_asleep`), does nothing when AI is off,
//! and sends nothing a person wrote. A deterministic run (tests, the end-to-end suite) does
//! not warm up, so call counts and cache state stay as the test set them; the evaluation turns it
//! on with [`set_forced`] to measure what people meet.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use super::{ProjectIndex, service};
use crate::core::AppCore;
use crate::modules::ai::{index, turn};

/// Step 3 is built, tested and **off**. Measured for spec slice S8 on this PC: in every paired run
/// of the first question of the Low Tide conversations (thirteen) it was never shorter with the engine
/// started early, and in five pairs on a quiet machine it took 0.04 to 5.2 s longer (mean +1.6 s; prompt
/// reading and writing were both slower), where the engine start alone costs 3.3 s. The engine alone
/// showed no such effect (primed or not, 1.6 to 3.2 s for 60 tokens), and the cause was not found.
/// Turn it on only together with the priming of the writer's own fixed start (about 5 s on the first
/// question), and measure again with `ai-eval --warmup on|off` on a machine nobody else is using.
const START_ENGINE: bool = false;

static FORCED: AtomicBool = AtomicBool::new(false);
/// Warm-ups running now.
static RUNNING: AtomicUsize = AtomicUsize::new(0);

/// Wait until no warm-up is running, at most `timeout`; true when none is. The evaluation uses it
/// to ask its first question as a person does, a few seconds after the project opened.
pub fn wait_idle(timeout: Duration) -> bool {
    let until = Instant::now() + timeout;
    while RUNNING.load(Ordering::SeqCst) > 0 {
        if Instant::now() >= until {
            return false;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    true
}

/// Counts a running warm-up, also when it ends early or stops on a fault.
struct Running;

impl Running {
    fn begin() -> Self {
        RUNNING.fetch_add(1, Ordering::SeqCst);
        Running
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        RUNNING.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Run the warm-up also in a deterministic run (the evaluation harness).
pub fn set_forced(on: bool) {
    FORCED.store(on, Ordering::SeqCst);
}

/// Start the warm-up for the project `ix` belongs to. Returns at once: true when a warm-up
/// started, false in a deterministic run or while the computer sleeps.
pub fn start(core: &AppCore, ix: &Arc<ProjectIndex>) -> bool {
    if core.config.ai_deterministic && !FORCED.load(Ordering::SeqCst) {
        return false;
    }
    if ix.is_paused() {
        return false;
    }
    let ix = ix.clone();
    let core = Arc::downgrade(&core.arc());
    // Counted before the thread exists, so a caller that waits right after `start` sees it.
    let running = Running::begin();
    let spawned = std::thread::Builder::new()
        .name("of-ai-warmup".into())
        .spawn(move || {
            let _running = running;
            run(&ix, &core);
        });
    match spawned {
        Ok(_) => true,
        Err(e) => {
            tracing::warn!(error = %e, "the AI warm-up could not start");
            false
        }
    }
}

fn run(ix: &ProjectIndex, core: &std::sync::Weak<AppCore>) {
    let stop = || ix.is_stopped() || ix.is_paused();
    let started = Instant::now();
    // Each step takes the core only for its own time, so a warm-up never keeps the app alive.
    let Some(c) = core.upgrade() else { return };
    if stop() {
        return;
    }
    if let Some(s) = c.project_opt() {
        let _ = index::project_index(&s);
    }
    let index_ms = started.elapsed().as_millis();
    if stop() {
        return;
    }
    if let Some(e) = service(&c).embedder(&c) {
        turn::warm::examples(&*e);
    }
    let examples_ms = started.elapsed().as_millis();
    let manager = crate::modules::ai::service::service(&c)
        .manager()
        .ok()
        .cloned();
    drop(c);
    if START_ENGINE
        && !stop()
        && let Some(manager) = manager
        && let Err(e) = manager.prime(&turn::warm::prompts(), &stop)
    {
        tracing::debug!(
            code = e.code_str(),
            "the AI engine could not be prepared early"
        );
    }
    tracing::info!(
        index_ms = index_ms as u64,
        examples_ms = examples_ms as u64,
        total_ms = started.elapsed().as_millis() as u64,
        "the assistant was prepared"
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::AppConfig;
    use crate::events::RecordingSink;
    use serde_json::json;

    /// No engine in this app: the warm-up reads the project index, finds no embedder and no engine,
    /// and ends without starting anything. Once the index is stopped (the project closed) it ends at
    /// the first check.
    #[test]
    fn without_an_engine_it_ends_quietly_and_a_stopped_index_ends_it_at_once() {
        let dir = tempfile::tempdir().unwrap();
        let core = AppCore::new(
            AppConfig::isolated(dir.path()),
            Arc::new(RecordingSink::default()),
        )
        .unwrap();
        core.dispatch(
            &core.local_actor(),
            "project.create",
            json!({ "title": "Warm", "projectType": "Feature Film" }),
        )
        .unwrap();
        let ix = service(&core).current().unwrap();
        let weak = Arc::downgrade(&core);
        run(&ix, &weak);
        let svc = crate::modules::ai::service::service(&core);
        assert!(svc.manager().is_ok_and(|m| !m.available()));
        ix.shutdown();
        let t = Instant::now();
        run(&ix, &weak);
        assert!(t.elapsed().as_millis() < 500);
        core.shutdown();
    }

    /// While the computer sleeps no warm-up starts, and one already running ends at its next
    /// step; after the wake it starts again. Forced on, as the evaluation does, so only the sleep
    /// keeps it from starting.
    #[test]
    fn no_warm_up_starts_while_the_computer_sleeps_and_one_starts_after_the_wake() {
        let dir = tempfile::tempdir().unwrap();
        let core = AppCore::new(
            AppConfig::isolated(dir.path()),
            Arc::new(RecordingSink::default()),
        )
        .unwrap();
        core.dispatch(
            &core.local_actor(),
            "project.create",
            json!({ "title": "Asleep", "projectType": "Feature Film" }),
        )
        .unwrap();
        let ix = service(&core).current().unwrap();
        set_forced(true);
        core.on_power_event(openframe_platform::power::PowerEvent::Suspending);
        let asleep = start(&core, &ix);
        let t = Instant::now();
        run(&ix, &Arc::downgrade(&core));
        let ended_in = t.elapsed();
        core.on_power_event(openframe_platform::power::PowerEvent::Resumed);
        let awake = start(&core, &ix);
        set_forced(false);
        assert!(!asleep, "a warm-up started while asleep");
        assert!(ended_in.as_millis() < 500, "a warm-up kept running asleep");
        assert!(awake, "the warm-up starts again after the wake");
        assert!(wait_idle(Duration::from_secs(30)));
        core.shutdown();
    }
}
