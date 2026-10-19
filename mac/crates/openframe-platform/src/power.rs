//! Sleep and wake notifications, so the app can make its data safe before the computer sleeps
//! (a laptop can run out of battery while asleep) and recover after it wakes.
//!
//! Windows adapter: `PowerRegisterSuspendResumeNotification` with a callback (no window needed).
//! `PBT_APMSUSPEND` becomes [`PowerEvent::Suspending`] and `PBT_APMRESUMEAUTOMATIC` (sent on
//! every wake, with or without a user) becomes [`PowerEvent::Resumed`]. macOS adapter: not written
//! yet (IOKit `IORegisterForSystemPower` needs a run loop thread and Mac hardware to verify);
//! [`subscribe`] returns `None` there and callers carry on without notifications.

use std::sync::Arc;

/// A change in the computer's power state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerEvent {
    /// The computer is about to sleep. Work done in the handler should be short (Windows allows
    /// about two seconds).
    Suspending,
    /// The computer woke from sleep.
    Resumed,
}

/// The handler called on each event, on an operating-system thread.
pub type PowerHandler = Arc<dyn Fn(PowerEvent) + Send + Sync>;

/// Keeps the notification registered; dropping it unregisters.
pub struct PowerSubscription {
    _inner: imp::Registration,
}

impl std::fmt::Debug for PowerSubscription {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("PowerSubscription")
    }
}

/// Call `handler` on every sleep and wake until the returned value is dropped. `None` when the
/// system refused or this platform has no adapter yet.
pub fn subscribe(handler: PowerHandler) -> Option<PowerSubscription> {
    imp::Registration::new(handler).map(|r| PowerSubscription { _inner: r })
}

/// The event for a Windows power broadcast code (`PBT_*`), shared so it can be tested anywhere.
pub fn from_broadcast(code: u32) -> Option<PowerEvent> {
    const PBT_APMSUSPEND: u32 = 4;
    const PBT_APMRESUMEAUTOMATIC: u32 = 18;
    match code {
        PBT_APMSUSPEND => Some(PowerEvent::Suspending),
        PBT_APMRESUMEAUTOMATIC => Some(PowerEvent::Resumed),
        // PBT_APMRESUMESUSPEND (7) follows 18 when a user is present: not reported twice.
        _ => None,
    }
}

#[cfg(windows)]
mod imp {
    use super::{PowerHandler, from_broadcast};
    use std::ffi::c_void;

    type Callback = unsafe extern "system" fn(*const c_void, u32, *const c_void) -> u32;

    /// `DEVICE_NOTIFY_SUBSCRIBE_PARAMETERS`.
    #[repr(C)]
    struct SubscribeParameters {
        callback: Callback,
        context: *mut c_void,
    }

    #[link(name = "powrprof")]
    unsafe extern "system" {
        fn PowerRegisterSuspendResumeNotification(
            flags: u32,
            recipient: *const c_void,
            registration: *mut *mut c_void,
        ) -> u32;
        fn PowerUnregisterSuspendResumeNotification(registration: *mut c_void) -> u32;
    }

    /// `DEVICE_NOTIFY_CALLBACK`.
    const DEVICE_NOTIFY_CALLBACK: u32 = 2;

    unsafe extern "system" fn on_power(
        context: *const c_void,
        kind: u32,
        _s: *const c_void,
    ) -> u32 {
        if let Some(event) = from_broadcast(kind) {
            // SAFETY: `context` is the `PowerHandler` box owned by the live `Registration`; the
            // registration is removed before that box is freed.
            let handler = unsafe { &*(context as *const PowerHandler) };
            // A panic must not cross the system callback.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(event)));
        }
        0
    }

    pub struct Registration {
        handle: *mut c_void,
        // Both must live as long as the registration.
        _params: Box<SubscribeParameters>,
        handler: *mut PowerHandler,
    }

    // SAFETY: the handle is a system registration handle usable from any thread, and the handler
    // is `Send + Sync`.
    unsafe impl Send for Registration {}
    unsafe impl Sync for Registration {}

    impl Registration {
        pub fn new(handler: PowerHandler) -> Option<Self> {
            let handler = Box::into_raw(Box::new(handler));
            let params = Box::new(SubscribeParameters {
                callback: on_power,
                context: handler as *mut c_void,
            });
            let mut handle: *mut c_void = std::ptr::null_mut();
            // SAFETY: `params` and `handler` stay alive until the registration is removed.
            let rc = unsafe {
                PowerRegisterSuspendResumeNotification(
                    DEVICE_NOTIFY_CALLBACK,
                    &*params as *const SubscribeParameters as *const c_void,
                    &mut handle,
                )
            };
            if rc != 0 || handle.is_null() {
                // SAFETY: not registered, so nothing else holds the pointer.
                drop(unsafe { Box::from_raw(handler) });
                return None;
            }
            Some(Self {
                handle,
                _params: params,
                handler,
            })
        }
    }

    impl Drop for Registration {
        fn drop(&mut self) {
            // SAFETY: unregister first (no callback runs after it returns), then free the handler.
            unsafe {
                let _ = PowerUnregisterSuspendResumeNotification(self.handle);
                drop(Box::from_raw(self.handler));
            }
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::PowerHandler;

    pub struct Registration;

    impl Registration {
        pub fn new(_handler: PowerHandler) -> Option<Self> {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broadcast_codes_map_to_one_event_each() {
        assert_eq!(from_broadcast(4), Some(PowerEvent::Suspending));
        assert_eq!(from_broadcast(18), Some(PowerEvent::Resumed));
        for other in [7u32, 10, 0x8013, 0] {
            assert_eq!(from_broadcast(other), None, "{other}");
        }
    }

    #[cfg(windows)]
    #[test]
    fn a_subscription_registers_and_unregisters_cleanly() {
        let sub = subscribe(Arc::new(|_| {})).expect("registered");
        assert!(format!("{sub:?}").contains("PowerSubscription"));
        drop(sub);
        // Registering again after the drop works (nothing was left behind).
        drop(subscribe(Arc::new(|_| {})).expect("registered again"));
    }
}
