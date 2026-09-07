#!/usr/bin/env bash

set -uo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
CALL_DIR="$(pwd)"

display_path() {
  python3 - <<'PY' "$CALL_DIR" "$1"
from pathlib import Path
import sys

call_dir = Path(sys.argv[1]).resolve()
target = Path(sys.argv[2]).resolve()

try:
    relative = target.relative_to(call_dir)
    text = "./" if str(relative) == "." else f"./{relative}"
except ValueError:
    text = str(target)

print(text)
PY
}

SHELL_VERSION="${1:-50}"
OUTPUT_DIR="${2:-./layout-extensions-report-shell-${SHELL_VERSION}}"
UUIDS_FILE="$OUTPUT_DIR/layout-extension-uuids.txt"
REPORT_JSON="$OUTPUT_DIR/report.json"
SUMMARY_TXT="$OUTPUT_DIR/summary.txt"
WORK_DIR="$OUTPUT_DIR/work"

mkdir -p "$OUTPUT_DIR" "$WORK_DIR"

echo "[1/4] Zbieram unikalne UUID rozszerzen z data/layouts/*.de"
python3 - <<'PY' "$ROOT_DIR" "$UUIDS_FILE"
from pathlib import Path
import re
import sys

root_dir = Path(sys.argv[1])
uuids_file = Path(sys.argv[2])
layouts_dir = root_dir / "data" / "layouts"
uuids = set()

for path in sorted(layouts_dir.glob("*.de")):
    text = path.read_text(encoding="utf-8")
    for match in re.finditer(r'^\s*id:\s*([^\s]+@[^\s]+)\s*$', text, re.MULTILINE):
        uuids.add(match.group(1))

uuids_file.write_text("".join(f"{uuid}\n" for uuid in sorted(uuids)), encoding="utf-8")
print(f"Found {len(uuids)} unique extension UUIDs")
PY

echo "[2/4] Uruchamiam sprawdzenie dla GNOME Shell $SHELL_VERSION"
set +e
"$ROOT_DIR/dev/check_gnome_extensions.py" \
  --shell-version "$SHELL_VERSION" \
  --uuids-file "$UUIDS_FILE" \
  --work-dir "$WORK_DIR" \
  --json > "$REPORT_JSON"
CHECK_EXIT_CODE=$?
set -e

if [ ! -f "$REPORT_JSON" ]; then
  echo "Brak report.json, sprawdzenie nie zapisalo wynikow." >&2
  exit 1
fi

echo "[3/4] Tworze tekstowe podsumowanie"
python3 - <<'PY' "$REPORT_JSON" "$SUMMARY_TXT"
from pathlib import Path
import json
import sys

report_path = Path(sys.argv[1])
summary_path = Path(sys.argv[2])
reports = json.loads(report_path.read_text(encoding="utf-8"))

status_counts = {}
for item in reports:
    status = item.get("status", "unknown")
    status_counts[status] = status_counts.get(status, 0) + 1

lines = []
lines.append(f"Total extensions: {len(reports)}")
lines.append("Status counts:")
for status in sorted(status_counts):
    lines.append(f"  - {status}: {status_counts[status]}")
lines.append("")

for item in reports:
    lines.append(f"UUID: {item['uuid']}")
    lines.append(f"Name: {item.get('name') or '(unknown)'}")
    lines.append(f"Status: {item.get('status')}")
    lines.append(f"Online compatible: {item.get('online_compatible')}")
    lines.append(f"Metadata compatible: {item.get('metadata_compatible')}")
    lines.append(
        f"Target version: {item.get('target_version')} (tag={item.get('target_version_tag')})"
    )
    lines.append(
        f"Newest known version: {item.get('newest_known_version')} (tag={item.get('newest_known_version_tag')})"
    )
    lines.append(f"Download URL: {item.get('download_url') or '(none)'}")
    lines.append(
        "Metadata shell versions: "
        + (
            ", ".join(item.get("metadata_shell_versions") or [])
            if item.get("metadata_shell_versions")
            else "(none)"
        )
    )
    if item.get("error"):
        lines.append(f"Error: {item['error']}")
    lines.append("-" * 72)

summary_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
print(f"Saved summary to {summary_path}")
PY

echo "[4/4] Gotowe"
echo "UUID list:    $(display_path "$UUIDS_FILE")"
echo "JSON report:  $(display_path "$REPORT_JSON")"
echo "Text summary: $(display_path "$SUMMARY_TXT")"
echo "Work dir:     $(display_path "$WORK_DIR")"
echo "Exit code from checker: $CHECK_EXIT_CODE"

if [ "$CHECK_EXIT_CODE" -ne 0 ]; then
  echo ""
  echo "Uwaga: checker zwrocil kod $CHECK_EXIT_CODE."
  echo "To zwykle znaczy, ze czesc rozszerzen byla niezgodna albo nie dalo sie pobrac metadanych."
fi
