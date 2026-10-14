//! Team password gate tests. Every hash here is made from a test-only password with small costs
//! so the tests stay fast; no real team password appears anywhere.

use super::*;

const T0: i64 = 1_791_244_800_000;
const MINUTE: i64 = 60 * 1000;
/// A test-only password. Never a real one.
const TEST_PASSWORD: &str = "test-only correct horse 7";
const SALT: [u8; 16] = *b"test-only-salt!!";

fn test_hash() -> String {
    hash_with_costs(TEST_PASSWORD, &SALT, 256, 1, 1).unwrap()
}

fn gate(dir: &Path, now: i64) -> TeamGate {
    TeamGate::new(test_hash(), dir, now)
}

fn wrong(g: &TeamGate, now: i64) -> UnlockAnswer {
    g.try_unlock("not the password", now)
}

#[test]
fn the_right_password_opens_and_the_wrong_one_does_not() {
    let dir = tempfile::tempdir().unwrap();
    let g = gate(dir.path(), T0);
    assert!(!g.is_unlocked());
    assert_eq!(
        wrong(&g, T0),
        UnlockAnswer::Wrong {
            tries_before_wait: 4,
            wait_ms: 0
        }
    );
    assert!(!g.is_unlocked());
    // Near misses are wrong too.
    for near in [
        "",
        "test-only correct horse 7 ",
        "Test-only correct horse 7",
        "test-only correct horse",
    ] {
        assert!(!verify_team_password(&test_hash(), near), "{near:?}");
    }
    assert_eq!(g.try_unlock(TEST_PASSWORD, T0), UnlockAnswer::Unlocked);
    assert!(g.is_unlocked());
    assert!(g.view(T0).unlocked);
    // A right password resets the count on disk.
    let again = gate(dir.path(), T0);
    assert!(!again.is_unlocked(), "every launch asks again");
    assert_eq!(again.view(T0).tries_before_wait, 5);
}

#[test]
fn five_wrong_tries_wait_one_minute_then_the_wait_doubles_up_to_30_minutes() {
    let dir = tempfile::tempdir().unwrap();
    let g = gate(dir.path(), T0);
    let mut now = T0;
    let mut waits = Vec::new();
    for _round in 0..8 {
        for i in 0..5 {
            let answer = wrong(&g, now);
            if i < 4 {
                assert!(
                    matches!(answer, UnlockAnswer::Wrong { wait_ms: 0, .. }),
                    "{answer:?}"
                );
            } else {
                let UnlockAnswer::Wrong { wait_ms, .. } = answer else {
                    panic!("{answer:?}");
                };
                waits.push(wait_ms);
                // During the wait a try is refused unchecked, even the right password.
                assert_eq!(
                    g.try_unlock(TEST_PASSWORD, now + wait_ms - 1),
                    UnlockAnswer::Waiting { wait_ms: 1 }
                );
                assert!(!g.is_unlocked());
                now += wait_ms;
            }
        }
    }
    assert_eq!(
        waits,
        vec![1, 2, 4, 8, 16, 30, 30, 30]
            .into_iter()
            .map(|m| m * MINUTE)
            .collect::<Vec<_>>()
    );
    // After the wait the right password still works and resets everything.
    assert_eq!(g.try_unlock(TEST_PASSWORD, now), UnlockAnswer::Unlocked);
    assert_eq!(wait_after(0), 0);
    assert_eq!(wait_after(4), 0);
    assert_eq!(wait_after(6), 0);
    // The largest count (u32::MAX is a multiple of 5) still waits only the longest wait.
    assert_eq!(wait_after(u32::MAX), MAX_WAIT_MS);
}

#[test]
fn a_restart_keeps_the_count_and_the_running_wait() {
    let dir = tempfile::tempdir().unwrap();
    let g = gate(dir.path(), T0);
    for _ in 0..3 {
        wrong(&g, T0);
    }
    drop(g);
    let g = gate(dir.path(), T0 + 1000);
    assert_eq!(g.view(T0 + 1000).tries_before_wait, 2);
    wrong(&g, T0 + 1000);
    assert!(matches!(
        wrong(&g, T0 + 1000),
        UnlockAnswer::Wrong {
            wait_ms: FIRST_WAIT_MS,
            ..
        }
    ));
    drop(g);
    // Restarting during the wait does not end it.
    let g = gate(dir.path(), T0 + 2000);
    assert_eq!(g.view(T0 + 2000).wait_ms, FIRST_WAIT_MS - 1000);
    assert!(matches!(
        g.try_unlock(TEST_PASSWORD, T0 + 2000),
        UnlockAnswer::Waiting { .. }
    ));
}

#[test]
fn a_try_is_counted_before_the_check_so_a_kill_cannot_undo_it() {
    let dir = tempfile::tempdir().unwrap();
    // A gate whose hash can never match: the check fails, and the count was saved first.
    let g = TeamGate::new("not a hash", dir.path(), T0);
    assert!(matches!(wrong(&g, T0), UnlockAnswer::Wrong { .. }));
    assert_eq!(
        g.try_unlock(TEST_PASSWORD, T0),
        UnlockAnswer::Wrong {
            tries_before_wait: 3,
            wait_ms: 0
        }
    );
    let saved =
        std::fs::read_to_string(dir.path().join("entitlement").join("team-gate.json")).unwrap();
    assert!(saved.contains("\"failures\":2"), "{saved}");
    assert!(!saved.contains(TEST_PASSWORD));
}

#[test]
fn a_damaged_count_file_starts_at_the_first_wait_not_at_zero() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("entitlement").join("team-gate.json");
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    for bytes in [
        b"{broken".to_vec(),
        br#"{"version":1,"failures":0,"waitStartedMs":null,"waitMs":-5}"#.to_vec(),
        vec![b' '; 10_000],
    ] {
        std::fs::write(&file, &bytes).unwrap();
        let g = gate(dir.path(), T0);
        assert_eq!(g.view(T0).wait_ms, FIRST_WAIT_MS);
        assert!(matches!(
            g.try_unlock(TEST_PASSWORD, T0),
            UnlockAnswer::Waiting { .. }
        ));
    }
}

#[test]
fn a_clock_turned_back_never_makes_a_wait_longer_than_its_length() {
    let dir = tempfile::tempdir().unwrap();
    let g = gate(dir.path(), T0);
    for _ in 0..5 {
        wrong(&g, T0);
    }
    assert_eq!(g.view(T0 + 30 * 1000).wait_ms, 30 * 1000);
    // The clock goes back a year in the middle of the wait. The wait starts again from then and
    // still ends after one minute of real time, not when the clock catches up a year later.
    let back = T0 - 365 * 24 * 60 * MINUTE;
    assert_eq!(g.view(back).wait_ms, FIRST_WAIT_MS);
    assert_eq!(g.view(back + 30 * 1000).wait_ms, 30 * 1000);
    assert_eq!(g.view(back + FIRST_WAIT_MS).wait_ms, 0);
    assert!(matches!(
        wrong(&g, back + FIRST_WAIT_MS),
        UnlockAnswer::Wrong { wait_ms: 0, .. }
    ));
    // The new start is saved, so a restart keeps it.
    drop(g);
    let g = gate(dir.path(), back + FIRST_WAIT_MS);
    assert_eq!(g.view(back + FIRST_WAIT_MS).wait_ms, 0);
    assert_eq!(g.view(back + FIRST_WAIT_MS).tries_before_wait, 4);
}

#[test]
fn a_clock_turned_back_during_a_wait_seen_first_by_a_try_restarts_the_wait() {
    let dir = tempfile::tempdir().unwrap();
    let g = gate(dir.path(), T0);
    for _ in 0..5 {
        wrong(&g, T0);
    }
    let back = T0 - 10 * MINUTE;
    // The try is refused unchecked and the wait counts from the earlier time.
    assert_eq!(
        g.try_unlock(TEST_PASSWORD, back),
        UnlockAnswer::Waiting {
            wait_ms: FIRST_WAIT_MS
        }
    );
    assert_eq!(
        g.try_unlock(TEST_PASSWORD, back + FIRST_WAIT_MS - 1),
        UnlockAnswer::Waiting { wait_ms: 1 }
    );
    assert_eq!(
        g.try_unlock(TEST_PASSWORD, back + FIRST_WAIT_MS),
        UnlockAnswer::Unlocked
    );
}

#[test]
fn only_an_argon2id_hash_with_the_build_costs_is_embedded() {
    let hash = hash_for_build(TEST_PASSWORD, &SALT).unwrap();
    assert!(
        hash.starts_with("$argon2id$v=19$m=65536,t=3,p=1$"),
        "{hash}"
    );
    assert!(!hash.contains(TEST_PASSWORD));
    assert!(verify_team_password(&hash, TEST_PASSWORD));
    assert!(!verify_team_password(&hash, "wrong password!"));
    // Another salt gives another hash for the same password.
    let other = hash_with_costs(TEST_PASSWORD, b"other-test-salt!", 256, 1, 1).unwrap();
    assert_ne!(other, test_hash());
    // Other Argon2 variants, garbage and oversized input never match.
    let argon2i = test_hash().replacen("argon2id", "argon2i", 1);
    assert!(!verify_team_password(&argon2i, TEST_PASSWORD));
    assert!(!verify_team_password("", TEST_PASSWORD));
    assert!(!verify_team_password(
        &test_hash(),
        &"x".repeat(MAX_PASSWORD_BYTES + 1)
    ));
}

#[test]
fn store_builds_carry_no_gate_and_internal_release_builds_need_the_variable() {
    // Store packages: never a gate or a hash, even when the variable is set on the machine.
    assert_eq!(
        build_plan("store", "release", Some(TEST_PASSWORD)),
        Ok(GatePlan::NoGate)
    );
    assert_eq!(build_plan("store", "release", None), Ok(GatePlan::NoGate));
    // An internal release build without the variable fails, and says which variable.
    for missing in [None, Some("")] {
        let e = build_plan("direct", "release", missing).unwrap_err();
        assert!(e.contains(TEAM_PASSWORD_ENV), "{e}");
    }
    // A short password is refused; the message never repeats it.
    let e = build_plan("direct", "release", Some("short-pw")).unwrap_err();
    assert!(!e.contains("short-pw"), "{e}");
    assert_eq!(
        build_plan("direct", "release", Some(TEST_PASSWORD)),
        Ok(GatePlan::Gate(TEST_PASSWORD.to_string()))
    );
    // Development builds run without it.
    assert_eq!(build_plan("direct", "debug", None), Ok(GatePlan::NoGate));
    assert_eq!(
        build_plan("direct", "debug", Some(TEST_PASSWORD)),
        Ok(GatePlan::Gate(TEST_PASSWORD.to_string()))
    );
}
