#!/usr/bin/env bash
# First start of the installed app on a copy of data written under the earlier name. The copy lives in a
# private home folder; the person's own data is only read, to make the copy (clones, so no disk is used).
#
# Usage: apps/macos/E2E/first-launch-old-data.sh <app> <folder with AppSupport-OpenFrame, AppSupport-OpenFrame-Library, Documents-OpenFrame>
# Output: release-out/old-data/<time>/ with the scenario's folder, before.sha and after.sha of the project files.
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$HERE/../../.." && pwd)"
APP="${1:?usage: first-launch-old-data.sh <app> <backup folder>}"
SOURCE="${2:?usage: first-launch-old-data.sh <app> <backup folder>}"
HOME_COPY="$(mktemp -d "${TMPDIR:-/tmp}/straft-old-data.XXXXXX")"
trap 'rm -rf "$HOME_COPY"' EXIT
RUN="$ROOT/release-out/old-data/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$RUN" "$HOME_COPY/Library/Application Support" "$HOME_COPY/Documents"
OLD_NAME=OpenFrame # LEGACY: Open Frame name kept for migration
cp -cR "$SOURCE/AppSupport-$OLD_NAME" "$HOME_COPY/Library/Application Support/$OLD_NAME"
cp -cR "$SOURCE/AppSupport-$OLD_NAME-Library" "$HOME_COPY/Library/Application Support/$OLD_NAME Library"
cp -cR "$SOURCE/Documents-$OLD_NAME" "$HOME_COPY/Documents/$OLD_NAME"
# Stored paths name the person's home; on the copy they must name the private home.
REAL_HOME="$(cd ~ && pwd)"
/usr/bin/python3 - "$HOME_COPY" "$REAL_HOME" "$OLD_NAME" <<'PY'
import json, sqlite3, sys
home, real, old = sys.argv[1:4]
pointer = f"{home}/Library/Application Support/{old} Library/library-pointer.json"
p = json.load(open(pointer))
p["libraryRoot"] = p["libraryRoot"].replace(real, home)
json.dump(p, open(pointer, "w"))
db = sqlite3.connect(f"{home}/Library/Application Support/{old}/app.sqlite")
tables = [r[0] for r in db.execute("select name from sqlite_master where type='table' and name not like 'sqlite_%'")]
for table in tables:
    for column in [r[1] for r in db.execute(f'pragma table_info("{table}")')]:
        db.execute(f'update "{table}" set "{column}" = replace("{column}", ?, ?) where typeof("{column}") = \'text\' and "{column}" like ?', (real, home, f"%{real}%"))
db.commit()
left = sum(db.execute(f'select count(*) from "{t}" where ' + " or ".join(f'"{r[1]}" like ?' for r in db.execute(f'pragma table_info("{t}")').fetchall()), [f"%{real}%"] * len(db.execute(f'pragma table_info("{t}")').fetchall())).fetchone()[0] for t in tables)
if left:
    sys.exit(f"{left} rows still name the real home: refusing to start the app on this copy")
db.close()
PY
( cd "$HOME_COPY/Documents/$OLD_NAME" && find . -type f ! -name '.DS_Store' ! -name '*.sqlite*' -print0 | sort -z | xargs -0 shasum -a 256 ) > "$RUN/before.sha"
export STRAFT_E2E_RUN_DIR="$RUN/run" STRAFT_E2E_OLD_DATA_HOME="$HOME_COPY" STRAFT_E2E_ID_SUFFIX=olddata
# The person's own library must not change: compared before and after.
REAL_LIBRARY="$REAL_HOME/Documents/Straft Studio"
real_state() { ( cd "$REAL_LIBRARY" 2>/dev/null && find . -type f ! -name '.DS_Store' ! -name '*.sqlite-shm' -print0 | sort -z | xargs -0 shasum -a 256 ); }
real_state > "$RUN/real-before.sha"
"$HERE/run.sh" "$APP" "zz-old-data/" | tee "$RUN/run.log"
STATUS=${PIPESTATUS[0]}
real_state > "$RUN/real-after.sha"
if ! cmp -s "$RUN/real-before.sha" "$RUN/real-after.sha"; then echo "the person's own library changed: NO" | tee -a "$RUN/folders.txt"; STATUS=1; fi
NEW_LIBRARY="$HOME_COPY/Documents/Straft Studio"
{
  echo "old app data folder kept: $([[ -f "$HOME_COPY/Library/Application Support/$OLD_NAME/app.sqlite" ]] && echo yes || echo NO)"
  echo "new app data folder: $([[ -f "$HOME_COPY/Library/Application Support/Straft/app.sqlite" ]] && echo yes || echo NO)"
  echo "library renamed: $([[ -d "$NEW_LIBRARY" && ! -e "$HOME_COPY/Documents/$OLD_NAME" ]] && echo yes || echo NO)"
  echo "project folders: $(ls "$NEW_LIBRARY/Projects" 2>/dev/null | tr '\n' ' ')"
} | tee -a "$RUN/folders.txt"
ls -la "$HOME_COPY/Documents" "$HOME_COPY/Library/Application Support" > "$RUN/listing.txt" 2>&1
cat "$HOME_COPY/Library/Application Support/Straft Library/library-pointer.json" >> "$RUN/listing.txt" 2>&1
tail -60 "$HOME_COPY/Library/Application Support/Straft/logs/"* > "$RUN/app-log-tail.txt" 2>&1
if [[ -d "$NEW_LIBRARY" ]]; then
  ( cd "$NEW_LIBRARY" && find . -type f ! -name '.DS_Store' ! -name '*.sqlite*' -print0 | sort -z | xargs -0 shasum -a 256 ) > "$RUN/after.sha"
  sed -E "s#\\.$OLD_NAME/#.straft/#; s#/$OLD_NAME\\.json#/straft.json#" "$RUN/before.sha" | awk '{print $1}' | sort > "$RUN/before.hashes"
  awk '{print $1}' "$RUN/after.sha" | sort > "$RUN/after.hashes"
  echo "project files whose content changed or went missing (a manifest takes the new format name): $(comm -23 "$RUN/before.hashes" "$RUN/after.hashes" | wc -l | tr -d ' ')" | tee -a "$RUN/folders.txt"
fi
grep -q 'NO$' "$RUN/folders.txt" && STATUS=1
exit "$STATUS"
