#!/bin/bash

set -euo pipefail

usage() {
    cat <<'EOF'
Usage: ./scripts/export-cherry-training.sh [OUTPUT.tar.gz]

Create a checksummed, compressed export of training/cherry-main.
When OUTPUT is omitted, the archive is written under training/exports/.
Stop Cherry training before running this script.
EOF
}

fail() {
    printf 'export-cherry-training: %s\n' "$*" >&2
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
RUN_DIR=${CHERRY_RUN_DIR:-"$RUST_ROOT/training/cherry-main"}
if [[ $RUN_DIR != /* ]]; then
    RUN_DIR="$PWD/$RUN_DIR"
fi

[[ -d $RUN_DIR ]] || fail "run directory not found: $RUN_DIR"

if command -v pgrep >/dev/null 2>&1 &&
    pgrep -f '(^|[ /])cherry-train([ /]|$).*train([ /]|$)' >/dev/null 2>&1; then
    fail "Cherry training appears to be running; stop it before exporting"
fi

required_files=(
    state.txt
    latest.bin
    champion.bin
    optimizer.bin
    replay.bin
)
for file in "${required_files[@]}"; do
    [[ -s "$RUN_DIR/$file" ]] || fail "required run file is missing or empty: $RUN_DIR/$file"
done
grep -qx 'promotion_protocol=2' "$RUN_DIR/state.txt" ||
    fail "state.txt is missing Cherry's current promotion protocol marker"

unexpected=$(find "$RUN_DIR" ! -type f ! -type d -print)
[[ -z $unexpected ]] || fail "run contains an unsupported file type: $unexpected"
temporary=$(find "$RUN_DIR" -type f -name '*.tmp' -print)
[[ -z $temporary ]] || fail "run contains an unfinished temporary file: $temporary"

timestamp=$(date -u '+%Y%m%dT%H%M%SZ')
archive=${1:-"$RUST_ROOT/training/exports/cherry-main-$timestamp.tar.gz"}
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

work_dir=$(mktemp -d "${TMPDIR:-/tmp}/cherry-export.XXXXXX")
archive_tmp="$archive.tmp.$$"
cleanup() {
    rm -rf -- "$work_dir"
    rm -f -- "$archive_tmp"
}
trap cleanup EXIT INT TERM

cp -R "$RUN_DIR" "$work_dir/cherry-main"

source_commit=$(git -C "$RUST_ROOT" rev-parse HEAD 2>/dev/null || printf 'unknown')
cat >"$work_dir/MANIFEST.txt" <<EOF
format=cherry-training-export-v1
source_commit=$source_commit
exported_utc=$timestamp
EOF

(
    cd "$work_dir/cherry-main"
    find . -type f -print |
        LC_ALL=C sort |
        while IFS= read -r file; do
            shasum -a 256 "$file"
        done
) >"$work_dir/exported-files.sha256"

(
    cd "$RUN_DIR"
    find . -type f -print |
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
    find cherry-main -type f -print |
        LC_ALL=C sort |
        while IFS= read -r file; do
            shasum -a 256 "$file"
        done
) >"$work_dir/SHA256SUMS"
rm -f "$work_dir/exported-files.sha256" "$work_dir/source-files.sha256"

tar -C "$work_dir" -czf "$archive_tmp" MANIFEST.txt SHA256SUMS cherry-main
mv "$archive_tmp" "$archive"

printf 'Cherry training export created:\n%s\n' "$archive"
du -h "$archive"
