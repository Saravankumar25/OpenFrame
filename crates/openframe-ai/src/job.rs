//! Kill-on-close Job Object: the sidecar can never outlive OpenFrame, even if
//! OpenFrame itself crashes (the OS closes the job handle and terminates the
//! child). On other platforms this is a no-op; the supervisor still kills the
//! child on stop/drop.

use std::process::Child;

pub struct KillOnCloseJob {
    #[cfg(windows)]
    handle: windows::Win32::Foundation::HANDLE,
}

// SAFETY: a job HANDLE is a kernel object handle; it may be used and closed from any thread.
unsafe impl Send for KillOnCloseJob {}
unsafe impl Sync for KillOnCloseJob {}

impl KillOnCloseJob {
    #[cfg(windows)]
    pub fn assign(child: &Child) -> Option<Self> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW,
            JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };
        // SAFETY: straightforward Win32 calls with owned handles; every failure path closes the job.
        unsafe {
            let job = CreateJobObjectW(None, windows::core::PCWSTR::null()).ok()?;
            let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            info.BasicLimitInformation.LimitFlags =
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE | JOB_OBJECT_LIMIT_DIE_ON_UNHANDLED_EXCEPTION;
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

    #[cfg(not(windows))]
    pub fn assign(_child: &Child) -> Option<Self> {
        None
    }
}

impl Drop for KillOnCloseJob {
    fn drop(&mut self) {
        #[cfg(windows)]
        // SAFETY: we own the handle; closing it terminates any process still in the job.
        unsafe {
            let _ = windows::Win32::Foundation::CloseHandle(self.handle);
        }
    }
}

/// Process creation flags: no console window pops up for the sidecar.
#[cfg(windows)]
pub fn hide_window(cmd: &mut std::process::Command) {
    use std::os::windows::process::CommandExt;
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    cmd.creation_flags(CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
pub fn hide_window(_cmd: &mut std::process::Command) {}
