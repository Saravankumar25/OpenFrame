//! JSON log files for the Mac shell, with the release rules of the Windows adapter (security
//! baseline E7, log redaction test P13): OpenFrame's own targets never log content and may go up
//! to `trace`; every other crate is capped at `info` in a release build, because third-party debug
//! output can quote paths or request bodies. Logs rotate daily and 14 files are kept.

use std::path::Path;

pub fn init(app_data_dir: &Path) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::EnvFilter;
    use tracing_subscriber::util::SubscriberInitExt;
    let dir = app_data_dir.join("logs");
    std::fs::create_dir_all(&dir).ok()?;
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix("openframe")
        .filename_suffix("log")
        .max_log_files(14)
        .build(&dir)
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);
    let requested = std::env::var("OPENFRAME_LOG").ok();
    let directives = filter_directives(requested.as_deref(), !cfg!(debug_assertions));
    let filter = EnvFilter::try_new(&directives).unwrap_or_else(|_| EnvFilter::new("info"));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(writer)
        .with_ansi(false)
        .json()
        .finish()
        .try_init();
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info
            .location()
            .map(|l| format!("{}:{}", l.file(), l.line()))
            .unwrap_or_default();
        let message = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_default();
        tracing::error!("{}", openframe_security::panic_summary(&message, &location));
        default_hook(info);
    }));
    Some(guard)
}

/// Development builds use `OPENFRAME_LOG` as written. Release builds keep only plain `level` and
/// `target=level` directives, cap every target that is not OpenFrame's own at `info` and drop
/// span and field filters.
pub fn filter_directives(requested: Option<&str>, release: bool) -> String {
    const LEVELS: [&str; 6] = ["off", "error", "warn", "info", "debug", "trace"];
    const INFO: usize = 3;
    let requested = requested.map(str::trim).filter(|r| !r.is_empty());
    if !release {
        return requested.unwrap_or("info").to_string();
    }
    let mut out: Vec<String> = Vec::new();
    for directive in requested.unwrap_or("info").split(',').map(str::trim) {
        let (target, level) = match directive.rsplit_once('=') {
            Some((t, l)) => (Some(t.trim()), l.trim()),
            None => (None, directive),
        };
        let Some(rank) = LEVELS.iter().position(|l| l.eq_ignore_ascii_case(level)) else {
            continue;
        };
        let capped = LEVELS[rank.min(INFO)];
        match target {
            None => {
                out.push(capped.to_string());
                if rank > INFO {
                    out.push(format!("openframe={}", LEVELS[rank]));
                }
            }
            Some(t)
                if t.is_empty()
                    || !t
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == ':') =>
            {
                continue;
            }
            Some(t) if t.starts_with("openframe") => out.push(format!("{t}={}", LEVELS[rank])),
            Some(t) => out.push(format!("{t}={capped}")),
        }
    }
    if out.is_empty() {
        out.push("info".to_string());
    }
    out.join(",")
}

#[cfg(test)]
mod tests {
    use super::filter_directives;

    #[test]
    fn release_builds_cap_third_party_targets_at_info() {
        let f = |r: &str| filter_directives(Some(r), true);
        assert_eq!(f("trace"), "info,openframe=trace");
        assert_eq!(
            f("warn,reqwest=trace,openframe_ai=trace"),
            "warn,reqwest=info,openframe_ai=trace"
        );
        assert_eq!(f("openframe[span{x=1}]=trace"), "info");
        assert_eq!(f("nonsense"), "info");
    }

    #[test]
    fn development_builds_take_the_variable_as_written() {
        assert_eq!(filter_directives(Some("reqwest=trace"), false), "reqwest=trace");
        assert_eq!(filter_directives(None, false), "info");
    }
}
