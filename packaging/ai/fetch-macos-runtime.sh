#!/bin/bash
# Fetch the pinned llama.cpp Apple silicon (Metal) engine and lay it out the way the core expects
# under an app's Contents/Resources/ai folder (ADR-0033, ADR-0026).
#
#   packaging/ai/fetch-macos-runtime.sh [output-folder]
#
# Output (default .build-cache/ai-runtime/macos-arm64/), which is the contents of the "ai" folder:
#   runtimes/<runtime-id>/<files of the lock>      real files, never links (the core refuses links)
#   notices/                                       licence notices
#
# The archive and every installed file are checked against packaging/ai/ai-assets.lock.json (bytes
# and SHA-256). The archive is untrusted data: it is downloaded into its own empty folder and
# unpacked into another, and nothing in it is run. The archive is deleted after the install.
# The lock names each file by the name the engine loads (libggml.0.dylib); the archive keeps those
# as links to the versioned file, so the install copies what the link points at.
# The binary and every library find each other through @loader_path, so no rpath change is needed.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo="$(cd "$here/../.." && pwd)"
lock="$repo/packaging/ai/ai-assets.lock.json"
out="${1:-$repo/.build-cache/ai-runtime/macos-arm64}"

if [ "$(uname -s)" != "Darwin" ] || [ "$(uname -m)" != "arm64" ]; then
  echo "This script installs the Apple silicon engine and runs on an Apple silicon Mac only." >&2
  exit 1
fi

# One line per fact, read from the lock: id, archive url, archive bytes, archive sha256, then files.
plan="$(python3 -I - "$lock" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
r = [x for x in d["runtimes"] if x["os"] == "macos" and x["arch"] == "aarch64" and x["backend"] == "metal"]
if len(r) != 1:
    sys.exit("the lock has no single macOS aarch64 Metal runtime")
r = r[0]
print("id", r["runtimeId"])
print("url", r["source"]["url"])
print("abytes", r["source"]["bytes"])
print("asha", r["source"]["sha256"])
print("exe", r["executable"])
for f in r["files"]:
    print("file", f["path"], f["bytes"], f["sha256"])
PY
)"
field() { printf '%s\n' "$plan" | awk -v k="$1" '$1 == k { print $2 }'; }
runtime_id="$(field id)"
url="$(field url)"
abytes="$(field abytes)"
asha="$(field asha)"
exe="$(field exe)"

sha() { shasum -a 256 "$1" | awk '{ print $1 }'; }
size() { stat -f %z "$1"; }

work="$repo/.build-cache/ai-download/$runtime_id"
mkdir -p "$repo/.build-cache/ai-download"
rm -rf "$work"
mkdir "$work"
mkdir "$work/archive" "$work/unpacked"
trap 'rm -rf "$work"' EXIT

echo "Downloading $url"
curl --fail --silent --show-error --location --proto '=https' --tlsv1.2 \
  --output "$work/archive/engine.tar.gz" "$url"
if [ "$(size "$work/archive/engine.tar.gz")" != "$abytes" ] || [ "$(sha "$work/archive/engine.tar.gz")" != "$asha" ]; then
  echo "The downloaded archive does not match the lock. Nothing was installed." >&2
  exit 1
fi

# Refuse names that could leave the folder before unpacking anything.
if tar -tzf "$work/archive/engine.tar.gz" | grep -E '(^/|(^|/)\.\.(/|$))' >/dev/null; then
  echo "The archive holds an unsafe path. Nothing was installed." >&2
  exit 1
fi
tar -xzf "$work/archive/engine.tar.gz" -C "$work/unpacked"
src="$(find "$work/unpacked" -mindepth 1 -maxdepth 1 -type d | head -n 1)"

dest="$out/runtimes/$runtime_id"
rm -rf "$out"
mkdir -p "$dest" "$out/notices"

printf '%s\n' "$plan" | awk '$1 == "file"' | while read -r _ name bytes want; do
  # -L follows the archive's link to the versioned file; the copy is a regular file.
  cp -L "$src/$name" "$dest/$name"
  if [ "$(size "$dest/$name")" != "$bytes" ] || [ "$(sha "$dest/$name")" != "$want" ]; then
    echo "$name does not match the lock. Removing the install." >&2
    rm -rf "$out"
    exit 1
  fi
done

chmod 755 "$dest/$exe"
# Files fetched from the internet carry com.apple.quarantine when a browser saved them; clear any
# extended attribute so the engine starts from this folder.
xattr -cr "$out" 2>/dev/null || true
cp "$repo"/packaging/ai/notices/*.txt "$out/notices/"

# Prove the program starts and loads every library (it only prints its version).
"$dest/$exe" --version >/dev/null 2>&1 || {
  echo "The installed engine did not start." >&2
  exit 1
}
echo "Installed $runtime_id into $out"
