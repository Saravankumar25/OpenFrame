//! Spelling dictionaries that come with the operating system (build plan item "Proofing").
//!
//! OpenFrame ships no dictionary files of its own. It asks the operating system which spelling
//! languages are installed and checks text against them, so no licence question arises and
//! nothing is downloaded.
//!
//! - **Windows adapter:** the Windows Spell Checking API (`ISpellCheckerFactory`,
//!   `ISpellChecker`). Every call runs on one worker thread that owns its own COM apartment, so
//!   callers never initialise COM and COM objects never cross threads. Offsets are UTF-16 code
//!   units, which is also what the web view uses.
//! - **macOS adapter:** not built yet. It reports no installed languages, so the editor shows no
//!   underlines and Settings says that spelling dictionaries are not available. The Mac adapter
//!   (`NSSpellChecker`, which must be called on the main thread) needs Mac hardware to build and
//!   verify.
//!
//! Text handed to the operating system stays on this computer. Inputs are bounded by the caller
//! (the application layer); this module also refuses oversized text.

/// One word the dictionary does not know. `start` and `len` are UTF-16 code units in the text
/// that was checked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Misspelling {
    pub start: u32,
    pub len: u32,
}

/// Why a spelling call failed. The message is for logs only; the UI shows its own sentence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpellError {
    /// No dictionary for this language tag on this computer.
    LanguageNotInstalled,
    /// The text is larger than one call accepts.
    TooLarge,
    /// The operating system service failed or did not answer in time.
    Unavailable(String),
}

/// Largest text one `check` call accepts, in UTF-16 code units.
pub const MAX_CHECK_UNITS: usize = 64 * 1024;
/// Largest word `suggest` accepts, in characters.
pub const MAX_WORD_CHARS: usize = 64;

/// A spelling service. The application layer talks to this trait so tests can use a fake.
pub trait SpellEngine: Send + Sync {
    /// Installed language tags such as `en-US`, in the order the system reports them. An error
    /// means the service could not answer (not that nothing is installed).
    fn languages(&self) -> Result<Vec<String>, SpellError>;
    /// Unknown words in `text` for `language`.
    fn check(&self, language: &str, text: &str) -> Result<Vec<Misspelling>, SpellError>;
    /// Replacement suggestions for one word, best first.
    fn suggest(&self, language: &str, word: &str) -> Result<Vec<String>, SpellError>;
}

/// The operating system's spelling service.
pub fn system() -> &'static dyn SpellEngine {
    imp::engine()
}

/// Whether this platform has a spelling adapter at all (false on macOS until the Mac adapter is
/// built). Used by Settings to explain an empty language list.
pub const fn adapter_available() -> bool {
    imp::AVAILABLE
}

#[cfg(windows)]
mod imp {
    use std::collections::HashMap;
    use std::sync::mpsc::{self, Receiver, Sender, SyncSender, TrySendError};
    use std::sync::{Mutex, OnceLock};
    use std::time::{Duration, Instant};

    use windows::Win32::Globalization::{
        CORRECTIVE_ACTION_NONE, ISpellChecker, ISpellCheckerFactory, ISpellingError,
        SpellCheckerFactory,
    };
    use windows::Win32::System::Com::{
        CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx,
        CoTaskMemFree, IEnumString,
    };
    use windows::core::{HSTRING, PWSTR};

    use super::{MAX_CHECK_UNITS, MAX_WORD_CHARS, Misspelling, SpellEngine, SpellError};

    pub const AVAILABLE: bool = true;
    /// The worker answers within this time or the call reports the service as unavailable, so
    /// a stuck system service can never hang the editor.
    const REPLY_TIMEOUT: Duration = Duration::from_secs(5);
    /// Requests waiting for the worker. A full queue fails at once instead of piling up behind
    /// a slow service.
    const QUEUE: usize = 32;
    /// How long the installed language list is reused before asking the system again.
    const LANGUAGES_TTL: Duration = Duration::from_secs(60);

    type Reply<T> = Sender<Result<T, SpellError>>;

    enum Request {
        Languages(Reply<Vec<String>>),
        Check(String, String, Reply<Vec<Misspelling>>),
        Suggest(String, String, Reply<Vec<String>>),
    }

    struct WindowsEngine {
        tx: Mutex<Option<SyncSender<Request>>>,
        languages: Mutex<Option<(Instant, Vec<String>)>>,
    }

    pub fn engine() -> &'static dyn SpellEngine {
        static ENGINE: OnceLock<WindowsEngine> = OnceLock::new();
        ENGINE.get_or_init(|| WindowsEngine {
            tx: Mutex::new(None),
            languages: Mutex::new(None),
        })
    }

    impl WindowsEngine {
        fn send<T>(&self, make: impl FnOnce(Reply<T>) -> Request) -> Result<T, SpellError> {
            let tx = {
                let mut guard = self
                    .tx
                    .lock()
                    .map_err(|_| SpellError::Unavailable("spelling worker lock poisoned".into()))?;
                if guard.is_none() {
                    *guard = Some(start_worker()?);
                }
                guard.as_ref().cloned().expect("worker sender just set")
            };
            let (reply_tx, reply_rx) = mpsc::channel();
            match tx.try_send(make(reply_tx)) {
                Ok(()) => {}
                Err(TrySendError::Full(_)) => {
                    return Err(SpellError::Unavailable("spelling service is busy".into()));
                }
                Err(TrySendError::Disconnected(_)) => {
                    // The worker ended (for example COM could not start); a fresh one is started
                    // on the next call.
                    if let Ok(mut g) = self.tx.lock() {
                        *g = None;
                    }
                    return Err(SpellError::Unavailable("spelling worker stopped".into()));
                }
            }
            reply_rx
                .recv_timeout(REPLY_TIMEOUT)
                .map_err(|_| SpellError::Unavailable("spelling service did not answer".into()))?
        }
    }

    impl SpellEngine for WindowsEngine {
        fn languages(&self) -> Result<Vec<String>, SpellError> {
            if let Ok(g) = self.languages.lock()
                && let Some((at, list)) = g.as_ref()
                && at.elapsed() < LANGUAGES_TTL
            {
                return Ok(list.clone());
            }
            let list = self.send(Request::Languages)?;
            if let Ok(mut g) = self.languages.lock() {
                *g = Some((Instant::now(), list.clone()));
            }
            Ok(list)
        }
        fn check(&self, language: &str, text: &str) -> Result<Vec<Misspelling>, SpellError> {
            if text.encode_utf16().count() > MAX_CHECK_UNITS {
                return Err(SpellError::TooLarge);
            }
            if text.trim().is_empty() {
                return Ok(Vec::new());
            }
            // The system reads a NUL-terminated string; a NUL inside the text would hide the
            // rest. A space keeps every offset where it was.
            let text = text.replace('\0', " ");
            self.send(|r| Request::Check(language.to_string(), text, r))
        }
        fn suggest(&self, language: &str, word: &str) -> Result<Vec<String>, SpellError> {
            if word.chars().count() > MAX_WORD_CHARS
                || word.trim().is_empty()
                || word.contains('\0')
            {
                return Err(SpellError::TooLarge);
            }
            self.send(|r| Request::Suggest(language.to_string(), word.to_string(), r))
        }
    }

    fn start_worker() -> Result<SyncSender<Request>, SpellError> {
        let (tx, rx) = mpsc::sync_channel::<Request>(QUEUE);
        std::thread::Builder::new()
            .name("openframe-spelling".into())
            .spawn(move || worker(rx))
            .map_err(|e| {
                SpellError::Unavailable(format!("could not start spelling worker: {e}"))
            })?;
        Ok(tx)
    }

    fn worker(rx: Receiver<Request>) {
        // SAFETY: called once at the start of this thread, which owns its apartment for life.
        // The thread never uninitialises COM because the COM objects it caches live as long as
        // the thread.
        let init = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        let factory: Option<ISpellCheckerFactory> = if init.is_ok() {
            // SAFETY: the CLSID is the documented spell checker factory; COM is initialised.
            unsafe { CoCreateInstance(&SpellCheckerFactory, None, CLSCTX_INPROC_SERVER).ok() }
        } else {
            None
        };
        let Some(factory) = factory else {
            // Answer the request that started this worker, then end. The next call starts a
            // fresh worker, so a passing failure does not disable spelling until a restart.
            let failed = || SpellError::Unavailable("spell checker service did not start".into());
            if let Ok(req) = rx.recv() {
                match req {
                    Request::Languages(r) => {
                        let _ = r.send(Err(failed()));
                    }
                    Request::Check(_, _, r) => {
                        let _ = r.send(Err(failed()));
                    }
                    Request::Suggest(_, _, r) => {
                        let _ = r.send(Err(failed()));
                    }
                }
            }
            return;
        };
        let mut checkers: HashMap<String, ISpellChecker> = HashMap::new();
        while let Ok(req) = rx.recv() {
            match req {
                Request::Languages(reply) => {
                    let _ = reply.send(languages(&factory));
                }
                Request::Check(lang, text, reply) => {
                    let r = checker(&factory, &mut checkers, &lang).and_then(|c| check(&c, &text));
                    let _ = reply.send(r);
                }
                Request::Suggest(lang, word, reply) => {
                    let r =
                        checker(&factory, &mut checkers, &lang).and_then(|c| suggest(&c, &word));
                    let _ = reply.send(r);
                }
            }
        }
    }

    fn unavailable(e: windows::core::Error) -> SpellError {
        SpellError::Unavailable(format!("{:#x}", e.code().0))
    }

    fn languages(f: &ISpellCheckerFactory) -> Result<Vec<String>, SpellError> {
        // SAFETY: `f` is a live factory owned by this thread.
        let list = unsafe { f.SupportedLanguages() }.map_err(unavailable)?;
        Ok(drain(&list, 64))
    }

    fn checker(
        f: &ISpellCheckerFactory,
        cache: &mut HashMap<String, ISpellChecker>,
        lang: &str,
    ) -> Result<ISpellChecker, SpellError> {
        if let Some(c) = cache.get(lang) {
            return Ok(c.clone());
        }
        let tag = HSTRING::from(lang);
        // SAFETY: `f` is a live factory; `tag` outlives both calls.
        let supported = unsafe { f.IsSupported(&tag) }.map_err(unavailable)?;
        if !supported.as_bool() {
            return Err(SpellError::LanguageNotInstalled);
        }
        // SAFETY: as above.
        let c = unsafe { f.CreateSpellChecker(&tag) }.map_err(unavailable)?;
        cache.insert(lang.to_string(), c.clone());
        Ok(c)
    }

    fn check(c: &ISpellChecker, text: &str) -> Result<Vec<Misspelling>, SpellError> {
        let h = HSTRING::from(text);
        // SAFETY: `c` is a live checker; `h` outlives the call and the enumeration below only
        // returns indexes, never pointers into `h`.
        let errors = unsafe { c.Check(&h) }.map_err(unavailable)?;
        let mut out = Vec::new();
        loop {
            let mut item: Option<ISpellingError> = None;
            // SAFETY: `item` receives an owned interface pointer or stays None at the end.
            let hr = unsafe { errors.Next(&mut item) };
            if hr.is_err() {
                return Err(SpellError::Unavailable(format!("{:#x}", hr.0)));
            }
            let Some(e) = item else { break };
            // SAFETY: `e` is a live spelling error returned above.
            let (start, len, action) = unsafe {
                (
                    e.StartIndex().map_err(unavailable)?,
                    e.Length().map_err(unavailable)?,
                    e.CorrectiveAction().map_err(unavailable)?,
                )
            };
            if action != CORRECTIVE_ACTION_NONE && len > 0 {
                out.push(Misspelling { start, len });
            }
            if out.len() >= 10_000 {
                break;
            }
        }
        Ok(out)
    }

    fn suggest(c: &ISpellChecker, word: &str) -> Result<Vec<String>, SpellError> {
        let h = HSTRING::from(word);
        // SAFETY: `c` is a live checker and `h` outlives the call.
        let list = unsafe { c.Suggest(&h) }.map_err(unavailable)?;
        Ok(drain(&list, 8))
    }

    /// Read up to `max` strings from an `IEnumString`, freeing each one.
    fn drain(list: &IEnumString, max: usize) -> Vec<String> {
        let mut out = Vec::new();
        while out.len() < max {
            let mut slot = [PWSTR::null()];
            let mut fetched = 0u32;
            // SAFETY: one slot, `fetched` tells how many were written.
            let hr = unsafe { list.Next(&mut slot, Some(&mut fetched)) };
            if hr.is_err() || fetched == 0 || slot[0].is_null() {
                break;
            }
            // SAFETY: the enumerator returned a CoTaskMemAlloc'd, NUL-terminated string that
            // we own; it is read once and freed once.
            let s = unsafe { slot[0].to_string() }.unwrap_or_default();
            unsafe { CoTaskMemFree(Some(slot[0].0 as *const _)) };
            if !s.is_empty() {
                out.push(s);
            }
        }
        out
    }
}

#[cfg(not(windows))]
mod imp {
    use super::{Misspelling, SpellEngine, SpellError};

    /// The macOS adapter is not built yet (see the module documentation).
    pub const AVAILABLE: bool = false;

    struct NoEngine;

    pub fn engine() -> &'static dyn SpellEngine {
        static ENGINE: NoEngine = NoEngine;
        &ENGINE
    }

    impl SpellEngine for NoEngine {
        fn languages(&self) -> Result<Vec<String>, SpellError> {
            Ok(Vec::new())
        }
        fn check(&self, _language: &str, _text: &str) -> Result<Vec<Misspelling>, SpellError> {
            Err(SpellError::LanguageNotInstalled)
        }
        fn suggest(&self, _language: &str, _word: &str) -> Result<Vec<String>, SpellError> {
            Err(SpellError::LanguageNotInstalled)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_input_is_refused_before_reaching_the_system() {
        let huge = "a ".repeat(MAX_CHECK_UNITS);
        let r = system().check("en-US", &huge);
        if adapter_available() {
            assert_eq!(r, Err(SpellError::TooLarge));
        } else {
            assert!(r.is_err());
        }
        let long_word = "x".repeat(MAX_WORD_CHARS + 1);
        assert!(system().suggest("en-US", &long_word).is_err());
    }

    #[test]
    fn unknown_language_tag_is_reported_not_crashed() {
        let r = system().check("zz-ZZ", "hello wrold");
        assert!(r.is_err(), "{r:?}");
    }

    /// Runs against the real Windows dictionary when English (United States) is installed,
    /// which is the default on Windows 11. Skips silently elsewhere.
    #[test]
    fn system_dictionary_flags_a_misspelling_with_utf16_offsets() {
        if !system()
            .languages()
            .unwrap_or_default()
            .iter()
            .any(|l| l == "en-US")
        {
            return;
        }
        let text = "Meera walks into teh room.";
        let misses = system().check("en-US", text).unwrap();
        let start = text.encode_utf16().count() - "teh room.".encode_utf16().count();
        assert!(
            misses.contains(&Misspelling {
                start: start as u32,
                len: 3
            }),
            "{misses:?}"
        );
        let sug = system().suggest("en-US", "teh").unwrap();
        assert!(sug.iter().any(|s| s == "the"), "{sug:?}");
        // Empty text never reaches the system; a NUL does not hide the words after it.
        assert!(system().check("en-US", "   ").unwrap().is_empty());
        let with_nul = "ok\0 teh";
        assert!(
            system()
                .check("en-US", with_nul)
                .unwrap()
                .contains(&Misspelling { start: 4, len: 3 })
        );
    }
}
