#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if [ "$#" -lt 1 ] || [ "$1" = "-h" ] || [ "$1" = "--help" ]; then
  echo "Usage: ./update-version.sh <version> [version_code]"
  echo ""
  echo "Examples:"
  echo "  ./update-version.sh 0.2.0        # sets versionName/Cargo version to 0.2.0 and increments versionCode"
  echo "  ./update-version.sh v1.0.0 10    # sets version to 1.0.0 and versionCode to 10"
  exit 0
fi

RAW_VERSION="$1"
VERSION="${RAW_VERSION#v}"
EXPLICIT_CODE="${2:-}"

if ! [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+([+-][0-9A-Za-z.-]+)?$ ]]; then
  echo "ERROR: Invalid semantic version '$RAW_VERSION' (expected e.g. 0.2.0 or v1.0.0)" >&2
  exit 1
fi

if [ -n "$EXPLICIT_CODE" ] && ! [[ "$EXPLICIT_CODE" =~ ^[0-9]+$ ]]; then
  echo "ERROR: version_code must be a positive integer (got '$EXPLICIT_CODE')" >&2
  exit 1
fi

python3 - "$ROOT_DIR" "$VERSION" "$EXPLICIT_CODE" <<'PY'
import pathlib
import re
import sys

root = pathlib.Path(sys.argv[1])
version = sys.argv[2]
explicit_code = sys.argv[3]

# 1. Update app/build.gradle.kts
gradle_path = root / "app" / "build.gradle.kts"
gradle_txt = gradle_path.read_text(encoding="utf-8")

code_match = re.search(r"(versionCode\s*=\s*)(\d+)", gradle_txt)
if not code_match:
    sys.exit("ERROR: Could not find versionCode in app/build.gradle.kts")

old_code = int(code_match.group(2))
new_code = int(explicit_code) if explicit_code else old_code + 1

name_match = re.search(r'(versionName\s*=\s*")([^"]+)(")', gradle_txt)
old_version = name_match.group(2) if name_match else "unknown"

gradle_txt = re.sub(r"(versionCode\s*=\s*)\d+", rf"\g<1>{new_code}", gradle_txt, count=1)
gradle_txt = re.sub(r'(versionName\s*=\s*")[^"]+(")', rf'\g<1>{version}\2', gradle_txt, count=1)
gradle_path.write_text(gradle_txt, encoding="utf-8")

# 2. Update Cargo.toml files ([package] section version only)
def update_cargo_toml(path: pathlib.Path):
    txt = path.read_text(encoding="utf-8")
    updated, count = re.subn(
        r'(\[package\][\s\S]*?\nversion\s*=\s*")([^"]+)(")',
        rf'\g<1>{version}\3',
        txt,
        count=1,
    )
    if count == 0:
        sys.exit(f"ERROR: Could not find [package] version in {path}")
    path.write_text(updated, encoding="utf-8")

for rel in ["rust-core/Cargo.toml", "cli/Cargo.toml"]:
    p = root / rel
    if p.exists():
        update_cargo_toml(p)

print(f"Updated version: {old_version} (code {old_code}) -> {version} (code {new_code})")
print("  - app/build.gradle.kts")
print("  - rust-core/Cargo.toml")
print("  - cli/Cargo.toml")
print("")
print(f"Next step for a release:")
print(f"  git commit -am \"Bump version to v{version}\" && git tag v{version} && git push && git push origin v{version}")
PY
