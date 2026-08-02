#!/bin/bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage: ./scripts/export-kiwi-training.sh [OUTPUT.tar.gz]

Create a checksummed, compressed export of training/kiwi-main.
When OUTPUT is omitted, the archive is written under training/exports/.
Stop Kiwi training before running this script.
EOF
}

fail() {
    printf 'export-kiwi-training: %s\n' "$*" >&2
    exit 1
}

if [[ $# -gt 1 ]]; then
    usage >&2
    exit 2
fi
if [[ ${1:-} == "--help" || ${1:-} == "-h" ]]; then
    usage
    exit 0
fi

SCRIPT_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
RUST_ROOT=$(CDPATH= cd -- "$SCRIPT_DIR/.." && pwd)
RUN_DIR=${KIWI_RUN_DIR:-"$RUST_ROOT/training/kiwi-main"}
if [[ $RUN_DIR != /* ]]; then
    RUN_DIR="$PWD/$RUN_DIR"
fi

[[ -d $RUN_DIR ]] || fail "run directory not found: $RUN_DIR"

if command -v pgrep >/dev/null 2>&1 &&
    pgrep -f '(^|[ /])kiwi-train([ /]|$).*train([ /]|$)' >/dev/null 2>&1; then
    fail "Kiwi training appears to be running; stop it before exporting"
fi

required_files=(
    state.txt
    latest.bin
    optimizer.bin
    replay.bin
)
for file in "${required_files[@]}"; do
    [[ -s "$RUN_DIR/$file" ]] || fail "required run file is missing or empty: $RUN_DIR/$file"
done
grep -qx 'purity=rules-only-fresh-seed' "$RUN_DIR/state.txt" ||
    fail "state.txt is missing Kiwi's rules-only fresh-seed marker"
grep -qx 'training_protocol=continuous-v1' "$RUN_DIR/state.txt" ||
    fail "state.txt is missing Kiwi's continuous-training protocol marker"

unexpected=$(find "$RUN_DIR" ! -type f ! -type d -print)
[[ -z $unexpected ]] || fail "run contains an unsupported file type: $unexpected"
temporary=$(find "$RUN_DIR" -type f -name '*.tmp' -print)
[[ -z $temporary ]] || fail "run contains an unfinished temporary file: $temporary"

timestamp=$(date -u '+%Y%m%dT%H%M%SZ')
archive=${1:-"$RUST_ROOT/training/exports/kiwi-main-$timestamp.tar.gz"}
if [[ $archive != /* ]]; then
    archive="$PWD/$archive"
fi
[[ $archive == *.tar.gz ]] || fail "output filename must end in .tar.gz"

archive_parent=$(dirname -- "$archive")
mkdir -p "$archive_parent"
archive_parent=$(CDPATH= cd -- "$archive_parent" && pwd)
archive="$archive_parent/$(basename -- "$archive")"
[[ ! -e $archive ]] || fail "refusing to overwrite existing archive: $archive"
case "$archive" in
    "$RUN_DIR"/*) fail "output archive cannot be placed inside the run directory" ;;
esac

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/kiwi-export.XXXXXX")
archive_tmp="$archive.tmp.$$"
cleanup() {
    rm -rf -- "$work_dir"
    rm -f -- "$archive_tmp"
}
trap cleanup EXIT INT TERM

cp -R "$RUN_DIR" "$work_dir/kiwi-main"

source_commit=$(git -C "$RUST_ROOT" rev-parse HEAD 2>/dev/null || printf 'unknown')
cat >"$work_dir/MANIFEST.txt" <<EOF
format=kiwi-training-export-v1
source_commit=$source_commit
exported_utc=$timestamp
EOF

(
    cd "$work_dir"
    find kiwi-main -type f -print |
        LC_ALL=C sort |
        while IFS= read -r file; do
            shasum -a 256 "$file"
        done
) >"$work_dir/exported-files.sha256"

(
    cd "$(dirname -- "$RUN_DIR")"
    find "$(basename -- "$RUN_DIR")" -type f -print |
        LC_ALL=C sort |
        while IFS= read -r file; do
            shasum -a 256 "$file"
        done
) >"$work_dir/source-files.sha256"

cmp -s "$work_dir/exported-files.sha256" "$work_dir/source-files.sha256" ||
    fail "the run changed while it was being copied; stop training and export again"

(
    cd "$work_dir"
    shasum -a 256 MANIFEST.txt
    cat exported-files.sha256
) >"$work_dir/SHA256SUMS"
rm -f "$work_dir/exported-files.sha256" "$work_dir/source-files.sha256"

tar -C "$work_dir" -czf "$archive_tmp" MANIFEST.txt SHA256SUMS kiwi-main
mv "$archive_tmp" "$archive"

printf 'Kiwi training export created:\n%s\n' "$archive"
du -h "$archive"
