#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/package.sh --target <triple> [--out-dir <dir>]

Options:
  --target   Target triple (ex: x86_64-unknown-linux-gnu)
  --out-dir  Output directory (default: dist)
  -h, --help Show this help

Environment:
  SIGN=1     Enable GPG signing if gpg is available
USAGE
}

TARGET=""
OUT_DIR="dist"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --target)
      TARGET="$2"
      shift 2
      ;;
    --out-dir)
      OUT_DIR="$2"
      shift 2
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown argument: $1" >&2
      usage
      exit 1
      ;;
  esac
done

if [[ -z "$TARGET" ]]; then
  echo "--target is required." >&2
  usage
  exit 1
fi

VERSION=$(python3 - <<'PY'
import pathlib
import tomllib

cargo_toml = pathlib.Path("Cargo.toml")
with cargo_toml.open("rb") as f:
    data = tomllib.load(f)
print(data["package"]["version"])
PY
)

BIN_NAME="roxanne"
BIN_EXT=""
if [[ "$TARGET" == *"windows"* ]]; then
  BIN_EXT=".exe"
fi

cargo build --release --target "$TARGET"

BIN_PATH="target/${TARGET}/release/${BIN_NAME}${BIN_EXT}"
if [[ ! -f "$BIN_PATH" ]]; then
  echo "Binary not found at $BIN_PATH" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
STAGING_DIR=$(mktemp -d)
PACKAGE_NAME="${BIN_NAME}-${VERSION}-${TARGET}"

mkdir -p "$STAGING_DIR/$PACKAGE_NAME"
cp "$BIN_PATH" "$STAGING_DIR/$PACKAGE_NAME/"
cp README.md LICENSE "$STAGING_DIR/$PACKAGE_NAME/"
if [[ -f docs/installation.md ]]; then
  cp docs/installation.md "$STAGING_DIR/$PACKAGE_NAME/"
fi

ARCHIVE_PATH="${OUT_DIR}/${PACKAGE_NAME}"
if [[ "$TARGET" == *"windows"* ]]; then
  ARCHIVE_PATH+=".zip"
  STAGING_DIR="$STAGING_DIR" ARCHIVE_PATH="$ARCHIVE_PATH" PACKAGE_NAME="$PACKAGE_NAME" python3 - <<'PY'
import os
import pathlib
import zipfile

staging = pathlib.Path(os.environ["STAGING_DIR"])
archive = pathlib.Path(os.environ["ARCHIVE_PATH"])
package_name = os.environ["PACKAGE_NAME"]
package_dir = staging / package_name

with zipfile.ZipFile(archive, "w", compression=zipfile.ZIP_DEFLATED) as zf:
    for path in package_dir.rglob("*"):
        zf.write(path, package_dir.name + "/" + path.relative_to(package_dir).as_posix())
PY
else
  ARCHIVE_PATH+=".tar.gz"
  tar -C "$STAGING_DIR" -czf "$ARCHIVE_PATH" "$PACKAGE_NAME"
fi

CHECKSUMS_FILE="${OUT_DIR}/SHA256SUMS"
touch "$CHECKSUMS_FILE"

write_sha256() {
  local file_path="$1"
  FILE_PATH="$file_path" CHECKSUMS_FILE="$CHECKSUMS_FILE" python3 - <<'PY'
import hashlib
import os
import pathlib

file_path = pathlib.Path(os.environ["FILE_PATH"])
checksum_file = pathlib.Path(os.environ["CHECKSUMS_FILE"])
sha_path = file_path.with_name(file_path.name + ".sha256")

hash_value = hashlib.sha256(file_path.read_bytes()).hexdigest()
line = f"{hash_value}  {file_path.name}\n"
sha_path.write_text(line)
with checksum_file.open("a", encoding="utf-8") as handle:
    handle.write(line)
PY
}

write_sha256 "$ARCHIVE_PATH"

if [[ "${SIGN:-0}" == "1" ]]; then
  if command -v gpg >/dev/null 2>&1; then
    gpg --armor --detach-sign "$ARCHIVE_PATH"
    write_sha256 "${ARCHIVE_PATH}.asc"
  else
    echo "SIGN=1 set but gpg not found; skipping signature." >&2
  fi
fi

rm -rf "$STAGING_DIR"

echo "Package created: $ARCHIVE_PATH"
