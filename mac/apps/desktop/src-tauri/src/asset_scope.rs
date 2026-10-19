//! The `ofasset` protocol: the only way the webview reads a file from disk.
//!
//! It serves managed media (images, audio and video previews) from the `assets/` folder of the
//! open project and of the Global Idea Vault, and nothing else. The allowed folders are replaced
//! as a whole every time a project opens, closes or switches, so a project that was closed can no
//! longer be read (build plan item "Asset protocol scope"). Tauri's own asset protocol cannot do
//! this: its scope can only grow, and a forbidden folder stays forbidden even after it is opened
//! again, so it is disabled in `tauri.conf.json`.
//!
//! Requests are `http://ofasset.localhost/<percent-encoded absolute path>` (Windows) or
//! `ofasset://localhost/<...>` (macOS), produced by `convertFileSrc(path, "ofasset")` in the
//! web code. A request is answered only when the path, after resolving links, lies inside an
//! allowed folder and is a regular file on a local disk. Refusals log no path.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};

use parking_lot::RwLock;
use tauri::http::{Request, Response, StatusCode, header};

/// Scheme name registered with the webview and used by `convertFileSrc`.
pub const SCHEME: &str = "ofasset";

/// Largest body sent for one request. Larger files are served in ranges (media elements ask for
/// ranges; a request without one gets the first part with status 206).
const MAX_BODY: u64 = 16 * 1024 * 1024;

/// The folders the webview may currently read. Built fresh for every project change.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct AssetRoots(Vec<PathBuf>);

impl AssetRoots {
    /// Allow one managed `assets/` folder and everything inside it. The only form accepted by
    /// architecture lint rule C5 is `allow_directory(<path>.assets(), true)` or
    /// `allow_directory(<path>.join("assets"), true)`.
    pub fn allow_directory(&mut self, dir: PathBuf, recursive: bool) {
        debug_assert!(recursive, "assets folders are always allowed recursively");
        if !self.0.contains(&dir) {
            self.0.push(dir);
        }
    }

    /// The allowed folders. Read by the tests only.
    #[cfg(test)]
    pub fn dirs(&self) -> &[PathBuf] {
        &self.0
    }
}

/// The live scope shared by the protocol handler and the command that changes projects.
#[derive(Debug, Default)]
pub struct AssetScope {
    roots: RwLock<AssetRoots>,
}

impl AssetScope {
    /// Replace every allowed folder. Folders not in `roots` stop being readable at once.
    pub fn replace(&self, roots: AssetRoots) {
        *self.roots.write() = roots;
    }

    /// A copy of the allowed folders. Read by the tests only.
    #[cfg(test)]
    pub fn roots(&self) -> AssetRoots {
        self.roots.read().clone()
    }

    /// The file a request may read, or `None`. `raw` is the percent-encoded request path.
    pub fn resolve(&self, raw: &str) -> Option<PathBuf> {
        let decoded = percent_decode(raw.trim_start_matches('/'))?;
        let requested = PathBuf::from(decoded);
        if !requested.is_absolute()
            || requested
                .components()
                .any(|c| matches!(c, Component::ParentDir | Component::CurDir))
            || !openframe_security::is_local_disk_path(&requested)
        {
            return None;
        }
        // Links and junctions are followed before the check, so a link inside `assets/` that
        // points at the database or `cache/` is refused.
        let real = std::fs::canonicalize(&requested).ok()?;
        if !real.is_file() {
            return None;
        }
        let roots = self.roots.read();
        roots
            .0
            .iter()
            .filter_map(|r| std::fs::canonicalize(r).ok())
            .any(|root| real.starts_with(&root))
            .then_some(real)
    }

    /// Answer one protocol request.
    pub fn respond(&self, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
        let Some(path) = self.resolve(request.uri().path()) else {
            tracing::debug!("asset request refused");
            return status(StatusCode::NOT_FOUND);
        };
        let range = request
            .headers()
            .get(header::RANGE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        match serve(&path, range.as_deref()) {
            Ok(r) => r,
            Err(code) => status(code),
        }
    }
}

fn status(code: StatusCode) -> Response<Vec<u8>> {
    Response::builder()
        .status(code)
        .header(header::CACHE_CONTROL, "no-store")
        .body(Vec::new())
        .unwrap_or_default()
}

fn serve(path: &Path, range: Option<&str>) -> Result<Response<Vec<u8>>, StatusCode> {
    let mut file = std::fs::File::open(path).map_err(|_| StatusCode::NOT_FOUND)?;
    let len = file
        .metadata()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .len();
    let (start, end, partial) = match range {
        Some(r) => {
            let (s, e) = parse_range(r, len).ok_or(StatusCode::RANGE_NOT_SATISFIABLE)?;
            (s, e, true)
        }
        None if len > MAX_BODY => (0, MAX_BODY - 1, true),
        None => (0, len.saturating_sub(1), false),
    };
    let count = if len == 0 { 0 } else { end - start + 1 };
    let mut body = vec![0u8; count as usize];
    file.seek(SeekFrom::Start(start))
        .and_then(|_| file.read_exact(&mut body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mime = mime_for(path);
    let mut b = Response::builder()
        .status(if partial {
            StatusCode::PARTIAL_CONTENT
        } else {
            StatusCode::OK
        })
        .header(header::CONTENT_TYPE, mime)
        .header(header::CONTENT_LENGTH, count.to_string())
        .header(header::ACCEPT_RANGES, "bytes")
        .header(header::CACHE_CONTROL, "no-store")
        .header("X-Content-Type-Options", "nosniff")
        // An SVG opened on its own must not run scripts or load anything.
        .header(
            header::CONTENT_SECURITY_POLICY,
            "default-src 'none'; img-src 'self'; style-src 'unsafe-inline'; sandbox",
        );
    if partial {
        b = b.header(header::CONTENT_RANGE, format!("bytes {start}-{end}/{len}"));
    }
    b.body(body).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// One `bytes=` range, clamped to the file and to [`MAX_BODY`]. Several ranges are refused.
pub fn parse_range(header: &str, len: u64) -> Option<(u64, u64)> {
    let spec = header.trim().strip_prefix("bytes=")?.trim();
    if spec.contains(',') || len == 0 {
        return None;
    }
    let (a, b) = spec.split_once('-')?;
    let (start, end) = match (a.trim(), b.trim()) {
        ("", "") => return None,
        ("", suffix) => {
            let n: u64 = suffix.parse().ok()?;
            if n == 0 {
                return None;
            }
            (len.saturating_sub(n), len - 1)
        }
        (s, "") => (s.parse().ok()?, len - 1),
        (s, e) => (s.parse().ok()?, e.parse::<u64>().ok()?.min(len - 1)),
    };
    if start > end || start >= len {
        return None;
    }
    Some((start, end.min(start + MAX_BODY - 1)))
}

/// `%XX` decoding of a request path. Invalid escapes and text that is not UTF-8 are refused.
pub fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hex = s.get(i + 1..i + 3)?;
                out.push(u8::from_str_radix(hex, 16).ok()?);
                i += 3;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    let text = String::from_utf8(out).ok()?;
    (!text.contains('\0')).then_some(text)
}

fn mime_for(path: &Path) -> &'static str {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match ext.as_str() {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "bmp" => "image/bmp",
        "avif" => "image/avif",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" | "oga" => "audio/ogg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encode(p: &Path) -> String {
        let mut s = String::from("/");
        for b in p.to_string_lossy().as_bytes() {
            if b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b'_') {
                s.push(*b as char);
            } else {
                s.push_str(&format!("%{b:02X}"));
            }
        }
        s
    }

    fn roots(dirs: &[&Path]) -> AssetRoots {
        let mut r = AssetRoots::default();
        for d in dirs {
            r.allow_directory(d.to_path_buf(), true);
        }
        r
    }

    #[test]
    fn replacing_the_roots_revokes_the_previous_project_at_once() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("A.openframe").join("assets");
        let b = tmp.path().join("B.openframe").join("assets");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        std::fs::write(a.join("still.png"), b"a").unwrap();
        std::fs::write(b.join("still.png"), b"b").unwrap();
        let scope = AssetScope::default();
        scope.replace(roots(&[&a]));
        assert!(scope.resolve(&encode(&a.join("still.png"))).is_some());
        assert!(scope.resolve(&encode(&b.join("still.png"))).is_none());
        // Switch to B: A is refused straight away.
        scope.replace(roots(&[&b]));
        assert!(scope.resolve(&encode(&a.join("still.png"))).is_none());
        assert!(scope.resolve(&encode(&b.join("still.png"))).is_some());
        // Close: nothing from either project.
        scope.replace(AssetRoots::default());
        assert!(scope.resolve(&encode(&b.join("still.png"))).is_none());
        // Reopening A works again (a forbid list could not do this).
        scope.replace(roots(&[&a]));
        assert!(scope.resolve(&encode(&a.join("still.png"))).is_some());
    }

    #[test]
    fn only_files_inside_an_assets_folder_are_served() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("Railway.openframe");
        let assets = project.join("assets");
        std::fs::create_dir_all(assets.join("ab")).unwrap();
        std::fs::create_dir_all(project.join("cache")).unwrap();
        std::fs::create_dir_all(project.join("assets2")).unwrap();
        std::fs::write(project.join("project.sqlite"), b"db").unwrap();
        std::fs::write(project.join("cache").join("index.bin"), b"x").unwrap();
        std::fs::write(project.join("assets2").join("x.png"), b"x").unwrap();
        std::fs::write(assets.join("ab").join("x.png"), b"x").unwrap();
        let scope = AssetScope::default();
        scope.replace(roots(&[&assets]));
        assert!(
            scope
                .resolve(&encode(&assets.join("ab").join("x.png")))
                .is_some()
        );
        for refused in [
            project.join("project.sqlite"),
            project.join("cache").join("index.bin"),
            project.join("assets2").join("x.png"),
            assets.join("..").join("project.sqlite"),
            assets.join("ab"),
            assets.join("missing.png"),
        ] {
            assert!(scope.resolve(&encode(&refused)).is_none(), "{refused:?}");
        }
        for raw in ["/relative%2Fx.png", "/%ZZ", "/%00", "", "/"] {
            assert!(scope.resolve(raw).is_none(), "{raw}");
        }
    }

    #[cfg(unix)]
    #[test]
    fn a_link_inside_assets_that_points_outside_is_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let assets = tmp.path().join("P.openframe").join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        let db = tmp.path().join("P.openframe").join("project.sqlite");
        std::fs::write(&db, b"db").unwrap();
        std::os::unix::fs::symlink(&db, assets.join("db.png")).unwrap();
        let scope = AssetScope::default();
        scope.replace(roots(&[&assets]));
        assert!(scope.resolve(&encode(&assets.join("db.png"))).is_none());
    }

    #[test]
    fn unicode_paths_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let assets = tmp.path().join("రైలు फ़िल्म.openframe").join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(assets.join("దృశ్యం.png"), b"x").unwrap();
        let scope = AssetScope::default();
        scope.replace(roots(&[&assets]));
        assert!(scope.resolve(&encode(&assets.join("దృశ్యం.png"))).is_some());
    }

    #[test]
    fn responses_carry_type_range_and_no_script_headers() {
        let tmp = tempfile::tempdir().unwrap();
        let assets = tmp.path().join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(assets.join("clip.mp4"), b"0123456789").unwrap();
        std::fs::write(assets.join("art.svg"), b"<svg/>").unwrap();
        let scope = AssetScope::default();
        scope.replace(roots(&[&assets]));
        let get = |p: &Path, range: Option<&str>| {
            let mut r = Request::builder().uri(format!("http://ofasset.localhost{}", encode(p)));
            if let Some(v) = range {
                r = r.header(header::RANGE, v);
            }
            scope.respond(&r.body(Vec::new()).unwrap())
        };
        let full = get(&assets.join("clip.mp4"), None);
        assert_eq!(full.status(), StatusCode::OK);
        assert_eq!(full.body(), b"0123456789");
        assert_eq!(full.headers()[header::CONTENT_TYPE], "video/mp4");
        let part = get(&assets.join("clip.mp4"), Some("bytes=2-4"));
        assert_eq!(part.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(part.body(), b"234");
        assert_eq!(part.headers()[header::CONTENT_RANGE], "bytes 2-4/10");
        let bad = get(&assets.join("clip.mp4"), Some("bytes=20-"));
        assert_eq!(bad.status(), StatusCode::RANGE_NOT_SATISFIABLE);
        let svg = get(&assets.join("art.svg"), None);
        assert!(
            svg.headers()[header::CONTENT_SECURITY_POLICY]
                .to_str()
                .unwrap()
                .contains("sandbox")
        );
        let refused = get(&tmp.path().join("assets").join("..").join("x"), None);
        assert_eq!(refused.status(), StatusCode::NOT_FOUND);
    }

    #[test]
    fn ranges_are_parsed_and_bounded() {
        assert_eq!(parse_range("bytes=0-", 10), Some((0, 9)));
        assert_eq!(parse_range("bytes=-3", 10), Some((7, 9)));
        assert_eq!(parse_range("bytes=5-100", 10), Some((5, 9)));
        assert_eq!(
            parse_range("bytes=0-", 3 * MAX_BODY),
            Some((0, MAX_BODY - 1))
        );
        for bad in [
            "bytes=9-2",
            "bytes=10-",
            "bytes=0-1,3-4",
            "items=0-1",
            "bytes=-0",
            "bytes=-",
            "bytes=a-b",
        ] {
            assert_eq!(parse_range(bad, 10), None, "{bad}");
        }
        assert_eq!(parse_range("bytes=0-", 0), None);
    }
}
