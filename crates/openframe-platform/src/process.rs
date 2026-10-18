//! Managed child processes (the AI engine). The child must never outlive OpenFrame, even when
//! OpenFrame crashes, and must never show a console window.
//!
//! Windows adapter: the child is created suspended, put in a kill-on-close Job Object that also
//! allows only one process (the engine cannot start other programs), and only then resumed, so it
//! never runs a single instruction outside the job. The OS closes the job handle when the parent
//! dies and terminates every process in it. macOS adapter: the child is placed in its own process
//! group so the whole group can be signalled; the parent-death guarantee comes from the sidecar
//! supervisor's stop-on-drop plus the startup sweep of stray engines ([`stamp`] and
//! [`stop_if_same`], used by `openframe_ai::engines`). The macOS part compiles but is verified only
//! on Mac hardware.

use std::path::Path;
use std::process::{Child, Command};

/// A guard that ties a child to this process. Dropping it stops the child on Windows; on macOS
/// it sends SIGTERM to the child's process group.
pub struct ChildGuard {
    _inner: imp::Guard,
}

impl ChildGuard {
    /// Attach the guard to a spawned child. Returns `None` when the operating system refused,
    /// in which case the caller must stop the child and refuse to run (security standard:
    /// "refuse to run when that fails").
    pub fn attach(child: &Child) -> Option<Self> {
        imp::Guard::attach(child).map(|inner| Self { _inner: inner })
    }

    /// Whether this platform gives a kernel-level guarantee that the child dies with the parent.
    pub const fn kernel_enforced() -> bool {
        imp::KERNEL_ENFORCED
    }
}

impl std::fmt::Debug for ChildGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ChildGuard")
            .field("kernel_enforced", &Self::kernel_enforced())
            .finish()
    }
}

/// Why [`spawn_guarded`] did not hand back a running child.
#[derive(Debug)]
pub enum SpawnError {
    /// The program could not be started at all.
    Spawn(std::io::Error),
    /// The operating system refused the guard on a platform where it is kernel enforced. The
    /// child was stopped before it ran; the caller must refuse to run.
    Unguarded,
    /// The child was created and guarded but could not be resumed. It was stopped.
    Resume,
}

/// Start `cmd` as a managed background child: no window, its own process group where the
/// platform has one, and tied to this process. On Windows the child is created suspended, placed
/// in the job, then resumed. Returns the guard as `None` only where the guard is best effort
/// (macOS) and the operating system refused it.
pub fn spawn_guarded(cmd: &mut Command) -> Result<(Child, Option<ChildGuard>), SpawnError> {
    imp::prepare_background(cmd);
    let mut child = cmd.spawn().map_err(SpawnError::Spawn)?;
    let guard = ChildGuard::attach(&child);
    if guard.is_none() && ChildGuard::kernel_enforced() {
        let _ = child.kill();
        let _ = child.wait();
        return Err(SpawnError::Unguarded);
    }
    if !imp::resume(&child) {
        let _ = child.kill();
        let _ = child.wait();
        return Err(SpawnError::Resume);
    }
    Ok((child, guard))
}

/// A process identity that survives pid reuse: the pid plus the process start time as the
/// operating system reports it (opaque units, compared only for equality).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessStamp {
    pub pid: u32,
    pub started: u64,
}

/// The stamp of a running child, or `None` when the operating system does not report it.
pub fn stamp(child: &Child) -> Option<ProcessStamp> {
    let pid = child.id();
    imp::start_time(pid).map(|started| ProcessStamp { pid, started })
}

/// What [`stop_if_same`] found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SweepOutcome {
    /// The recorded process was still running from the recorded program and was stopped.
    Stopped,
    /// No process with that pid is running.
    NotRunning,
    /// The pid now belongs to another process (other start time or program). Left alone.
    Different,
}

/// Stop a process left over from an earlier run, but only when its pid, its start time and its
/// program file all match the record. A reused pid of an unrelated program is never touched.
pub fn stop_if_same(stamp: ProcessStamp, program: &Path) -> SweepOutcome {
    imp::stop_if_same(stamp, program)
}

/// Program paths compare equal regardless of case on Windows and macOS (default file systems)
/// and of the `\\?\` prefix.
fn same_program(a: &Path, b: &Path) -> bool {
    let norm = |p: &Path| {
        let s = p.to_string_lossy().replace('/', "\\");
        s.strip_prefix(r"\\?\").unwrap_or(&s).to_lowercase()
    };
    norm(a) == norm(b)
}

#[cfg(windows)]
mod imp {
    use std::process::{Child, Command};
    use windows::Win32::Foundation::{CloseHandle, HANDLE};

    pub const KERNEL_ENFORCED: bool = true;

    pub struct Guard {
        handle: HANDLE,
    }

    // SAFETY: a job HANDLE is a kernel object handle; it may be used and closed from any thread.
    unsafe impl Send for Guard {}
    unsafe impl Sync for Guard {}

    impl Guard {
        pub fn attach(child: &Child) -> Option<Self> {
            use std::os::windows::io::AsRawHandle;
            use windows::Win32::System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
                JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
                SetInformationJobObject,
            };
            // SAFETY: plain Win32 calls with owned handles; every failure path closes the job.
            unsafe {
                let job = CreateJobObjectW(None, windows::core::PCWSTR::null()).ok()?;
                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                // One process only: the engine may not start other programs.
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
                    | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION
                    | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
                info.BasicLimitInformation.ActiveProcessLimit = 1;
                if SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const core::ffi::c_void,
                    std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
                .is_err()
                {
                    let _ = CloseHandle(job);
                    return None;
                }
                let proc = HANDLE(child.as_raw_handle());
                if AssignProcessToJobObject(job, proc).is_err() {
                    let _ = CloseHandle(job);
                    return None;
                }
                Some(Self { handle: job })
            }
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            // SAFETY: we own the handle; closing it terminates any process still in the job.
            unsafe {
                let _ = CloseHandle(self.handle);
            }
        }
    }

    pub fn prepare_background(cmd: &mut Command) {
        use std::os::windows::process::CommandExt;
        // No console at all (redirected pipes still work), and suspended until it is in the job.
        // DETACHED_PROCESS rather than CREATE_NO_WINDOW: a console host would be a second
        // process in the one-process job.
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_SUSPENDED: u32 = 0x0000_0004;
        cmd.creation_flags(DETACHED_PROCESS | CREATE_SUSPENDED);
    }

    /// Resume every thread of a child created suspended (a new process has exactly one).
    pub fn resume(child: &Child) -> bool {
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
        };
        use windows::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};
        let pid = child.id();
        let mut resumed = 0;
        // SAFETY: plain Win32 calls; every handle opened here is closed here.
        unsafe {
            let Ok(snap) = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) else {
                return false;
            };
            let mut entry = THREADENTRY32 {
                dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
                ..Default::default()
            };
            let mut more = Thread32First(snap, &mut entry).is_ok();
            while more {
                if entry.th32OwnerProcessID == pid
                    && let Ok(t) = OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID)
                {
                    if ResumeThread(t) != u32::MAX {
                        resumed += 1;
                    }
                    let _ = CloseHandle(t);
                }
                more = Thread32Next(snap, &mut entry).is_ok();
            }
            let _ = CloseHandle(snap);
        }
        resumed > 0
    }

    fn creation_time(process: HANDLE) -> Option<u64> {
        use windows::Win32::Foundation::FILETIME;
        use windows::Win32::System::Threading::GetProcessTimes;
        let (mut c, mut e, mut k, mut u) = (
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
            FILETIME::default(),
        );
        // SAFETY: the handle is open with query rights; the out-pointers are valid.
        unsafe { GetProcessTimes(process, &mut c, &mut e, &mut k, &mut u).ok()? };
        Some((u64::from(c.dwHighDateTime) << 32) | u64::from(c.dwLowDateTime))
    }

    pub fn start_time(pid: u32) -> Option<u64> {
        use windows::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};
        // SAFETY: the handle is closed before returning.
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid).ok()?;
            let t = creation_time(h);
            let _ = CloseHandle(h);
            t
        }
    }

    pub fn stop_if_same(
        stamp: super::ProcessStamp,
        program: &std::path::Path,
    ) -> super::SweepOutcome {
        use super::SweepOutcome;
        use windows::Win32::System::Threading::{
            OpenProcess, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION,
            PROCESS_SYNCHRONIZE, PROCESS_TERMINATE, QueryFullProcessImageNameW, TerminateProcess,
            WaitForSingleObject,
        };
        // SAFETY: the handle is closed on every path.
        unsafe {
            let Ok(h) = OpenProcess(
                PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_TERMINATE | PROCESS_SYNCHRONIZE,
                false,
                stamp.pid,
            ) else {
                return if start_time(stamp.pid).is_some() {
                    SweepOutcome::Different
                } else {
                    SweepOutcome::NotRunning
                };
            };
            let outcome = (|| {
                // A process that has exited can still be opened while another handle to it is
                // held (by its parent, for example). It is not running, whatever its record says.
                if WaitForSingleObject(h, 0) == windows::Win32::Foundation::WAIT_OBJECT_0 {
                    return SweepOutcome::NotRunning;
                }
                if creation_time(h) != Some(stamp.started) {
                    return SweepOutcome::Different;
                }
                let mut buf = vec![0u16; 32_768];
                let mut len = buf.len() as u32;
                if QueryFullProcessImageNameW(
                    h,
                    PROCESS_NAME_WIN32,
                    windows::core::PWSTR(buf.as_mut_ptr()),
                    &mut len,
                )
                .is_err()
                {
                    return SweepOutcome::Different;
                }
                let image = String::from_utf16_lossy(&buf[..len as usize]);
                if !super::same_program(std::path::Path::new(&image), program) {
                    return SweepOutcome::Different;
                }
                if TerminateProcess(h, 1).is_err() {
                    return SweepOutcome::Different;
                }
                let _ = WaitForSingleObject(h, 5_000);
                SweepOutcome::Stopped
            })();
            let _ = CloseHandle(h);
            outcome
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use std::process::{Child, Command};

    pub const KERNEL_ENFORCED: bool = false;

    pub struct Guard {
        /// The child's pid, which is also its process-group id (see `attach`).
        pid: i32,
    }

    impl Guard {
        pub fn attach(child: &Child) -> Option<Self> {
            let pid = i32::try_from(child.id()).ok()?;
            // SAFETY: getpgid only reads kernel state for a pid we own.
            let pgid = unsafe { libc::getpgid(pid) };
            // Only a child that leads its own group may be guarded; otherwise a group signal
            // would reach OpenFrame itself. `prepare_background` makes the child a leader.
            if pgid != pid || pid <= 1 {
                return None;
            }
            watch_parent(std::process::id(), pid);
            Some(Self { pid })
        }
    }

    /// macOS has no kernel object that ends children with their parent (Windows has job
    /// objects). A small watcher, the system shell with a fixed script and numeric arguments
    /// only, ends the child's group within a second of OpenFrame ending, also after a crash or
    /// a forced quit, and ends itself when the child ends first. It runs in a group of its
    /// own, so the signal for the child never reaches it.
    pub(super) fn watch_parent(parent_pid: u32, child_pid: i32) {
        use std::os::unix::process::CommandExt;
        use std::process::Stdio;
        const SCRIPT: &str = "while kill -0 \"$1\" 2>/dev/null && kill -0 \"$2\" 2>/dev/null; do sleep 1; done; \
             kill -0 \"$1\" 2>/dev/null || kill -KILL -\"$2\" 2>/dev/null";
        let parent = parent_pid.to_string();
        let child = child_pid.to_string();
        let spawned = Command::new("/bin/sh")
            .args(["-c", SCRIPT, "openframe-engine-watch", &parent, &child])
            .env_clear()
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0)
            .spawn();
        if let Ok(mut watcher) = spawned {
            // Reap it when it ends so it never lingers as a zombie of OpenFrame.
            std::thread::spawn(move || {
                let _ = watcher.wait();
            });
        }
    }

    impl Drop for Guard {
        fn drop(&mut self) {
            // Signal the group only while the pid still leads it, so a reused pid of an unrelated
            // process is never hit. SIGKILL: the engine holds nothing to save, and it does not
            // always end on SIGTERM while a request runs. SAFETY: plain libc calls on a pid we
            // spawned.
            unsafe {
                if libc::getpgid(self.pid) == self.pid {
                    let _ = libc::killpg(self.pid, libc::SIGKILL);
                }
            }
        }
    }

    pub fn prepare_background(cmd: &mut Command) {
        use std::os::unix::process::CommandExt;
        // A new process group (0 means "use the child's pid") so the group can be signalled.
        cmd.process_group(0);
    }

    /// Children start running at once on macOS; there is nothing to resume.
    pub fn resume(_child: &Child) -> bool {
        true
    }

    pub fn start_time(pid: u32) -> Option<u64> {
        let pid = i32::try_from(pid).ok()?;
        let mut info: libc::proc_bsdinfo = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<libc::proc_bsdinfo>() as i32;
        // SAFETY: the buffer is a zeroed proc_bsdinfo of the size passed.
        let n = unsafe {
            libc::proc_pidinfo(
                pid,
                libc::PROC_PIDTBSDINFO,
                0,
                &mut info as *mut _ as *mut libc::c_void,
                size,
            )
        };
        (n == size).then(|| info.pbi_start_tvsec * 1_000_000 + info.pbi_start_tvusec)
    }

    pub fn stop_if_same(
        stamp: super::ProcessStamp,
        program: &std::path::Path,
    ) -> super::SweepOutcome {
        use super::SweepOutcome;
        let Some(started) = start_time(stamp.pid) else {
            return SweepOutcome::NotRunning;
        };
        if started != stamp.started {
            return SweepOutcome::Different;
        }
        let Ok(pid) = i32::try_from(stamp.pid) else {
            return SweepOutcome::Different;
        };
        let mut buf = vec![0u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
        // SAFETY: the buffer has the size passed.
        let n = unsafe {
            libc::proc_pidpath(pid, buf.as_mut_ptr() as *mut libc::c_void, buf.len() as u32)
        };
        if n <= 0 {
            return SweepOutcome::Different;
        }
        let image = String::from_utf8_lossy(&buf[..n as usize]).into_owned();
        if !super::same_program(std::path::Path::new(&image), program) {
            return SweepOutcome::Different;
        }
        // SAFETY: plain signal to a pid whose identity was just checked.
        unsafe {
            libc::kill(pid, libc::SIGKILL);
        }
        SweepOutcome::Stopped
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod imp {
    use std::process::{Child, Command};

    pub const KERNEL_ENFORCED: bool = false;

    pub struct Guard;

    impl Guard {
        pub fn attach(_child: &Child) -> Option<Self> {
            Some(Self)
        }
    }

    pub fn prepare_background(_cmd: &mut Command) {}

    pub fn resume(_child: &Child) -> bool {
        true
    }

    pub fn start_time(_pid: u32) -> Option<u64> {
        None
    }

    pub fn stop_if_same(
        _stamp: super::ProcessStamp,
        _program: &std::path::Path,
    ) -> super::SweepOutcome {
        super::SweepOutcome::Different
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait_exit(child: &mut Child) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if child.try_wait().unwrap().is_some() {
                return;
            }
            assert!(Instant::now() < deadline, "child outlived its guard");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[cfg(windows)]
    fn comspec() -> std::path::PathBuf {
        std::env::var_os("ComSpec")
            .map(Into::into)
            .unwrap_or_else(|| r"C:\Windows\System32\cmd.exe".into())
    }

    #[cfg(windows)]
    #[test]
    fn a_guarded_child_dies_when_the_guard_is_dropped() {
        let mut cmd = Command::new(comspec());
        cmd.args(["/C", "ping -n 30 127.0.0.1 > NUL"]);
        let (mut child, guard) = spawn_guarded(&mut cmd).unwrap();
        let guard = guard.expect("job object");
        assert!(ChildGuard::kernel_enforced());
        assert!(format!("{guard:?}").contains("kernel_enforced"));
        drop(guard);
        wait_exit(&mut child);
    }

    /// The child was created suspended and resumed after it joined the job: it runs to the end
    /// and reports its own exit code.
    #[cfg(windows)]
    #[test]
    fn a_suspended_child_is_resumed_after_it_joins_the_job() {
        let mut cmd = Command::new(comspec());
        cmd.args(["/C", "exit 3"]);
        let (mut child, guard) = spawn_guarded(&mut cmd).unwrap();
        assert!(guard.is_some());
        let status = child.wait().unwrap();
        assert_eq!(status.code(), Some(3));
    }

    /// The job allows one process: the engine cannot start another program.
    #[cfg(windows)]
    #[test]
    fn a_guarded_child_cannot_start_another_program() {
        let mut cmd = Command::new(comspec());
        cmd.args(["/C", "cmd /C exit 7"]);
        let (mut child, _guard) = spawn_guarded(&mut cmd).unwrap();
        let status = child.wait().unwrap();
        assert_ne!(status.code(), Some(7), "the grandchild must not run");
    }

    #[test]
    fn a_missing_program_is_a_spawn_error() {
        let mut cmd = Command::new("of-no-such-program-7731");
        assert!(matches!(spawn_guarded(&mut cmd), Err(SpawnError::Spawn(_))));
    }

    /// The startup sweep stops a leftover engine only when pid, start time and program match.
    #[cfg(windows)]
    #[test]
    fn stop_if_same_stops_only_the_recorded_process() {
        let mut cmd = Command::new(comspec());
        cmd.args(["/C", "ping -n 30 127.0.0.1 > NUL"]);
        // A plain child (no guard), as an engine left over from a crashed run would be.
        let mut child = cmd.spawn().unwrap();
        let s = stamp(&child).expect("start time");
        assert_eq!(s.pid, child.id());
        let other = std::env::current_exe().unwrap();
        assert_eq!(
            stop_if_same(s, &other),
            SweepOutcome::Different,
            "other program"
        );
        let wrong_time = ProcessStamp {
            started: s.started + 1,
            ..s
        };
        assert_eq!(
            stop_if_same(wrong_time, &comspec()),
            SweepOutcome::Different
        );
        assert!(child.try_wait().unwrap().is_none(), "left alone");
        assert_eq!(stop_if_same(s, &comspec()), SweepOutcome::Stopped);
        wait_exit(&mut child);
        assert_eq!(stop_if_same(s, &comspec()), SweepOutcome::NotRunning);
    }

    #[test]
    fn program_paths_compare_without_case_or_prefix() {
        assert!(same_program(
            Path::new(r"\\?\C:\AI\llama-server.exe"),
            Path::new(r"c:\ai\LLAMA-SERVER.EXE")
        ));
        assert!(!same_program(
            Path::new(r"C:\AI\llama-server.exe"),
            Path::new(r"C:\AI2\llama-server.exe")
        ));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn a_guarded_child_is_killed_when_the_guard_is_dropped() {
        let mut cmd = Command::new("/bin/sleep");
        cmd.arg("30");
        let (mut child, guard) = spawn_guarded(&mut cmd).unwrap();
        drop(guard.expect("process group"));
        wait_exit(&mut child);
    }

    /// The engine must not outlive OpenFrame after a crash or a forced quit: the watcher ends
    /// the child's group once the parent is gone. A stand-in plays the parent here.
    #[cfg(target_os = "macos")]
    #[test]
    fn the_watcher_ends_the_child_when_the_parent_is_gone() {
        use std::os::unix::process::CommandExt;
        let mut parent = Command::new("/bin/sleep").arg("30").spawn().unwrap();
        let mut child = Command::new("/bin/sleep")
            .arg("30")
            .process_group(0)
            .spawn()
            .unwrap();
        imp::watch_parent(parent.id(), i32::try_from(child.id()).unwrap());
        std::thread::sleep(std::time::Duration::from_millis(1500));
        assert!(
            child.try_wait().unwrap().is_none(),
            "the child runs while the parent lives"
        );
        parent.kill().unwrap();
        let _ = parent.wait();
        wait_exit(&mut child);
    }
}
